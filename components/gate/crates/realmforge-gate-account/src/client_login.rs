// SPDX-License-Identifier: AGPL-3.0-only

//! In-client login handler for WoW Classic 1.13.2.
//!
//! Serves `POST /client/login/external?targetRegion=<region>` — the path the
//! 1.13.2 game client uses for email/password login (no SRP). The JSON envelope
//! uses the same `inputs[{input_id, value}]` shape as `/bnetserver/login/` but
//! with a different response shape that includes `auth.permit` and
//! `remember.auth.permit`.
//!
//! The login ticket minted here is later submitted to BGS v1
//! `VerifyWebCredentials` (method 7) as `web_credentials` bytes.

use std::sync::Arc;

use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::{HeaderValue, header};
use axum::response::IntoResponse;

use realmforge_gate_db::{accounts, credentials};

use crate::bnet_types::{ClientLoginResponse, LoginForm};
use crate::ticket;
use crate::{AccountError, AppState};

/// Query parameters for `POST /client/login/external`.
#[derive(serde::Deserialize)]
pub struct LoginExternalQuery {
    #[serde(default, rename = "targetRegion")]
    pub target_region: String,
}

/// `POST /client/login/external` — handles the 1.13.2 in-client login flow.
///
/// The client sends `inputs[{input_id, value}]` with `account_name` and
/// `password`. The server verifies via `sha_pass_hash` (plaintext, no SRP),
/// mints a login ticket, and returns `authentication_state: "DONE"` with
/// `auth.permit` and `remember.auth.permit` for session persistence.
pub async fn post_client_login(
    State(state): State<Arc<AppState>>,
    Query(query): Query<LoginExternalQuery>,
    Json(form): Json<LoginForm>,
) -> Result<impl IntoResponse, AccountError> {
    let _region = query.target_region;

    let account_name = form
        .input("account_name")
        .ok_or_else(|| AccountError::BadRequest("missing account_name".into()))?;

    let password = form
        .input("password")
        .ok_or_else(|| AccountError::BadRequest("missing password".into()))?;

    let account = accounts::find_by_email(&state.pool, account_name)
        .await?
        .ok_or(AccountError::NotFound)?;

    let cred = credentials::find_by_account_id(&state.pool, account.id)
        .await?
        .ok_or(AccountError::NotFound)?;

    let stored_hash = cred
        .sha_pass_hash
        .as_deref()
        .ok_or_else(|| AccountError::BadRequest("no plaintext credential stored".into()))?;

    if !crate::registration::verify_plaintext(&account.email, password, stored_hash) {
        return Err(AccountError::InvalidCredentials);
    }

    // ToS gate (LEGAL state): until the account has accepted the current
    // agreement version, return the LEGAL challenge and hand out no
    // login_ticket (realmforge-interop-analysis.md §4.11a).
    if !account_has_accepted_tos(&state.pool, account.id, &state.tos_version).await? {
        let session_id = uuid::Uuid::new_v4().to_string();
        state.pending_legal.insert(session_id.clone(), account.id);
        return Ok(build_legal_challenge(
            account.id,
            &session_id,
            &state.base_url,
            &state.tos_version,
        ));
    }

    finish_client_login(state.as_ref(), account.id).await
}

/// True when the account's accepted ToS version matches the current one.
async fn account_has_accepted_tos(
    pool: &sqlx::PgPool,
    account_id: i64,
    current_version: &str,
) -> Result<bool, AccountError> {
    let accepted: String = sqlx::query_scalar("SELECT tos_version FROM accounts WHERE id = $1")
        .bind(account_id)
        .fetch_one(pool)
        .await
        .map_err(|e| AccountError::Internal(format!("DB error: {e}")))?;
    Ok(accepted == current_version)
}

/// LEGAL (ToS) challenge: state + agreement URL + acceptance endpoint.
/// Shape per realmforge-interop-analysis.md §4.11a (binary-verified).
fn build_legal_challenge(
    account_id: i64,
    session_id: &str,
    base_url: &str,
    tos_version: &str,
) -> axum::response::Response {
    let body = serde_json::json!({
        "authentication_state": "LEGAL",
        "next_url": format!("{base_url}/client/login/tos/accept"),
        "legal_form": {
            "agreements": [
                { "url": format!("{base_url}/legal/agreement/{tos_version}") }
            ]
        }
    });
    tracing::info!(account_id, "ToS (LEGAL) challenge returned");
    let jsessionid = format!("JSESSIONID={session_id}; Path=/; Secure; HttpOnly; SameSite=None");
    let mut response = Json(body).into_response();
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/json;charset=utf-8"),
    );
    response.headers_mut().append(
        header::SET_COOKIE,
        HeaderValue::from_str(&jsessionid).unwrap(),
    );
    response
}

/// Continue a client login past the ToS gate: 2FA check, then DONE.
async fn finish_client_login(
    state: &AppState,
    account_id: i64,
) -> Result<axum::response::Response, AccountError> {
    // Check if the account has an authenticator.
    let has_2fa: bool = sqlx::query_scalar("SELECT has_authenticator FROM accounts WHERE id = $1")
        .bind(account_id)
        .fetch_one(&state.pool)
        .await
        .map_err(|e| AccountError::Internal(format!("DB error: {e}")))?;

    if has_2fa {
        let session_id = uuid::Uuid::new_v4().to_string();
        state
            .pending_authenticators
            .insert(session_id.clone(), account_id);
        return Ok(build_authenticator_challenge(
            account_id,
            &session_id,
            &state.base_url,
        ));
    }

    done_response(state, account_id).await
}

/// DONE response: minted login_ticket + auth.permit cookies.
async fn done_response(
    state: &AppState,
    account_id: i64,
) -> Result<axum::response::Response, AccountError> {
    let login_ticket =
        ticket::mint_login_ticket_with_region(&state.pool, account_id, &state.region).await?;
    tracing::info!(account_id, "client-login succeeded");

    // auth.permit is derived from account state (HMAC over account_id +
    // signing key), so it stays stable across logins for the same account.
    // The RE doc (realmforge-interop-analysis §2.2) confirms the value is opaque
    // to the client; stability is a defensive choice in case the client
    // compares permits on restart. remember.auth.permit stays a random UUID:
    // it models cross-device persistence, so per-login randomness fits.
    let auth_permit = crate::session::encode_session_token(account_id, &state.signing_key_pem);
    let remember_auth_permit = format!("RAP-{}", uuid::Uuid::new_v4().to_string().replace('-', ""));

    let body = ClientLoginResponse {
        authentication_state: "DONE".to_string(),
        login_ticket: Some(login_ticket),
        auth_permit: Some(auth_permit.clone()),
        remember_auth_permit: Some(remember_auth_permit.clone()),
        error_code: None,
        error_message: None,
    };

    // The 1.13.2 client reads `auth.permit` and `remember.auth.permit` from
    // Set-Cookie response headers (verified in the client binary:
    // ProcessLogonFormResponse matches "Set-Cookie:" and parses cookie
    // name=value pairs). Keys with the same names in the JSON body are
    // ignored by the client; the JSON copies above are inert documentation.
    let session_id = uuid::Uuid::new_v4().to_string();
    let jsessionid = format!("JSESSIONID={session_id}; Path=/; Secure; HttpOnly; SameSite=None");
    let auth_permit_cookie =
        format!("auth.permit={auth_permit}; Path=/; Secure; HttpOnly; SameSite=None");
    let rap_cookie = format!(
        "remember.auth.permit={remember_auth_permit}; Path=/; Secure; HttpOnly; SameSite=None"
    );

    // Build the response from the JSON body, then append owned cookie
    // HeaderValues. HeaderValue::from_str avoids leaking the cookie strings
    // for the process lifetime (String::leak does) and keeps three distinct
    // Set-Cookie headers via HeaderMap::append.
    let cookie = |raw: &str| -> Result<HeaderValue, AccountError> {
        HeaderValue::from_str(raw)
            .map_err(|_| AccountError::BadRequest("invalid cookie value".into()))
    };
    let mut response = Json(body).into_response();
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/json;charset=utf-8"),
    );
    response
        .headers_mut()
        .append(header::SET_COOKIE, cookie(&jsessionid)?);
    response
        .headers_mut()
        .append(header::SET_COOKIE, cookie(&auth_permit_cookie)?);
    response
        .headers_mut()
        .append(header::SET_COOKIE, cookie(&rap_cookie)?);
    Ok(response)
}

/// `POST /client/login/tos/accept` — accept the current ToS and continue the
/// login. The 1.13.2 client POSTs `accept_beula`/`accept_chat` (both "true")
/// to the LEGAL `next_url` (realmforge-interop-analysis.md §4.11a step 3); the
/// account is identified via the JSESSIONID cookie set by the LEGAL challenge.
pub async fn post_client_accept_tos(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    Json(form): Json<LoginForm>,
) -> Result<impl IntoResponse, AccountError> {
    // The client sends both inputs hardcoded as "true"; the server treats
    // them as account policy (the doc: the client sends both as true and
    // does not distinguish them).
    for id in ["accept_beula", "accept_chat"] {
        if form.input(id) != Some("true") {
            return Err(AccountError::BadRequest(format!(
                "missing or non-true {id} input"
            )));
        }
    }

    let session_id = extract_jsessionid(&headers)
        .ok_or_else(|| AccountError::BadRequest("missing JSESSIONID cookie".into()))?;
    let (_, account_id) = state
        .pending_legal
        .remove(&session_id)
        .ok_or_else(|| AccountError::BadRequest("unknown or expired session".into()))?;

    // Record acceptance, then continue the login (2FA check → DONE).
    sqlx::query("UPDATE accounts SET tos_version = $2, tos_accepted_at = NOW() WHERE id = $1")
        .bind(account_id)
        .bind(&state.tos_version)
        .execute(&state.pool)
        .await
        .map_err(|e| AccountError::Internal(format!("DB error: {e}")))?;
    tracing::info!(account_id, tos_version = %state.tos_version, "ToS accepted");

    finish_client_login(state.as_ref(), account_id).await
}

/// `GET /legal/agreement/{version}` — the agreement document the client
/// fetches during the LEGAL state (with an `Accept-Language` header).
pub async fn get_agreement(
    State(state): State<Arc<AppState>>,
    Path(version): Path<String>,
) -> Result<impl IntoResponse, AccountError> {
    if version != state.tos_version {
        return Err(AccountError::NotFound);
    }
    let text = format!(
        "WoW Emulation Realmforge — Terms of Service (version {version})\n\n\
         By logging in you agree to use this private server for educational \
         and interoperability purposes. The service provides no warranty; \
         data may be reset at any time.\n"
    );
    Ok(([(header::CONTENT_TYPE, "text/plain;charset=utf-8")], text))
}

pub(crate) fn build_authenticator_challenge(
    account_id: i64,
    session_id: &str,
    base_url: &str,
) -> axum::response::Response {
    let body = serde_json::json!({
        "authentication_state": "AUTHENTICATOR",
        "next_url": format!("{base_url}/client/login/authenticator"),
        "authenticator_form": {
            "type": "AUTHENTICATOR_FORM",
            "prompt": "Enter a code from your authenticator device.",
            "inputs": [
                {
                    "input_id": "authenticator_input",
                    "type": "text",
                    "label": "Verification Code",
                    "max_length": 20
                },
                {
                    "input_id": "remember_authenticator",
                    "type": "checkbox",
                    "label": "Don't ask me again on this device"
                }
            ]
        }
    });
    tracing::info!(account_id, "authenticator challenge returned");
    let jsessionid = format!("JSESSIONID={session_id}; Path=/; Secure; HttpOnly; SameSite=None");
    let mut response = Json(body).into_response();
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/json;charset=utf-8"),
    );
    response.headers_mut().append(
        header::SET_COOKIE,
        HeaderValue::from_str(&jsessionid).unwrap(),
    );
    response
}

/// POST /client/login/authenticator — verify the 8-digit authenticator code.
/// The client identifies the account via the JSESSIONID cookie set during
/// the initial login.
pub async fn post_client_authenticator(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    Json(form): Json<LoginForm>,
) -> Result<impl IntoResponse, AccountError> {
    let code = form
        .input("authenticator_input")
        .ok_or_else(|| AccountError::BadRequest("missing authenticator_input".into()))?;

    // Accept "12345678" as the valid code until proper MFA is wired up.
    if code != "12345678" {
        return Err(AccountError::InvalidCredentials);
    }

    // Identify the account via the JSESSIONID cookie.
    let session_id = extract_jsessionid(&headers)
        .ok_or_else(|| AccountError::BadRequest("missing JSESSIONID cookie".into()))?;

    let (_, account_id) = state
        .pending_authenticators
        .remove(&session_id)
        .ok_or_else(|| AccountError::BadRequest("unknown or expired session".into()))?;

    let login_ticket =
        crate::ticket::mint_login_ticket_with_region(&state.pool, account_id, &state.region)
            .await?;

    tracing::info!(account_id, "authenticator code accepted");

    // server_evidence_M2 is ignored by the client (RE doc §4.11).
    let body = serde_json::json!({
        "authentication_state": "DONE",
        "login_ticket": login_ticket
    });

    Ok(Json(body))
}

/// Extract the JSESSIONID value from Cookie headers.
fn extract_jsessionid(headers: &axum::http::HeaderMap) -> Option<String> {
    headers
        .get_all(axum::http::header::COOKIE)
        .iter()
        .filter_map(|v| v.to_str().ok())
        .flat_map(|s| s.split(';'))
        .map(|s| s.trim())
        .find_map(|part| {
            let (key, value) = part.split_once('=')?;
            if key.trim() == "JSESSIONID" {
                Some(value.trim().to_string())
            } else {
                None
            }
        })
}
