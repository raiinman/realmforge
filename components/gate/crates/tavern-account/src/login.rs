// SPDX-License-Identifier: AGPL-3.0-only

//! Browser SRP login handlers — capture-accurate wire format.
//!
//! Implements the web-login flow observed in the SRP login capture:
//!
//! 1. `POST /login/{locale}/` — email submission (form-encoded) → 302 to
//!    `/login/{locale}/password`.
//! 2. `POST /login/srp?csrfToken=true` — SRP challenge (JSON, AJAX). Request
//!    body: `{"inputs":[{"input_id":"account_name","value":"<email>"}]}`.
//!    Response includes `csrf_token`.
//! 3. `POST /login/{locale}/password` — SRP proof (form-encoded) → 302 to
//!    `oauth/authorize?...&ST=<region>-<hex>-<accountId>`.

use std::sync::Arc;
use std::time::{Duration, Instant};

use axum::Json;
use axum::extract::{Form, Path, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Redirect, Response};
use num_bigint::BigInt;
use serde::{Deserialize, Serialize};
use tracing;

use tavern_core::srp::{self, ServerSession};
use tavern_db::{accounts, credentials};

use crate::{AccountError, AppState, ticket};

// --- Types ---

/// `POST /login/{locale}/` form body.
#[derive(Deserialize)]
pub struct LoginEmailForm {
    #[serde(rename = "accountName")]
    pub account_name: String,
    #[serde(default)]
    pub srp_enabled: Option<String>,
    #[serde(default)]
    pub csrftoken: Option<String>,
    #[serde(default)]
    #[allow(dead_code)]
    pub session_timeout: Option<String>,
}

/// Query params on the login page URL (carried through from the OAuth ref).
#[derive(Deserialize)]
pub struct LoginQuery {
    #[serde(default)]
    #[allow(dead_code)]
    pub r#ref: Option<String>,
    #[serde(default)]
    #[allow(dead_code)]
    pub app: Option<String>,
    #[serde(default)]
    #[allow(dead_code)]
    pub hosting_app: Option<String>,
    /// 1.14.0 game-client SRP login: `?externalChallenge=login&app=wow`.
    #[serde(default, rename = "externalChallenge")]
    pub external_challenge: Option<String>,
}

/// `POST /login/srp?csrfToken=true` request body.
#[derive(Deserialize)]
pub struct SrpChallengeRequest {
    pub inputs: Vec<SrpInput>,
    #[serde(default)]
    pub program_id: Option<String>,
    #[serde(default)]
    pub platform_id: Option<String>,
    /// Client-reported version (e.g. "1.0" for 1.14.0 game client).
    #[serde(default)]
    pub version: Option<String>,
    /// LoginForm-style account_name field (browser SRP).
    #[serde(default)]
    pub account_name: Option<String>,
}

#[derive(Deserialize)]
pub struct SrpInput {
    pub input_id: String,
    pub value: String,
}

/// `POST /login/srp` response — all 10 fields from the capture.
#[derive(Serialize)]
pub struct SrpChallengeResponse {
    pub modulus: String,
    pub generator: String,
    pub hash_function: String,
    pub username: String,
    pub salt: String,
    #[serde(rename = "public_B")]
    pub public_b: String,
    pub version: u32,
    pub iterations: u32,
    pub eligible_credential_upgrade: bool,
    pub csrf_token: String,
}

/// `POST /login/{locale}/password` form body — all 11 fields from the capture.
#[derive(Deserialize)]
pub struct SrpProofForm {
    #[serde(default)]
    #[allow(dead_code)]
    pub username: String,
    #[serde(default)]
    #[allow(dead_code)]
    pub password: String,
    #[serde(default)]
    #[allow(dead_code)]
    pub upgrade_verifier: Option<String>,
    #[serde(default)]
    #[allow(dead_code)]
    pub use_srp: Option<String>,
    #[serde(rename = "publicA")]
    pub public_a: String,
    #[serde(rename = "clientEvidenceM1")]
    pub client_evidence_m1: String,
    #[serde(default)]
    #[allow(dead_code)]
    pub srp_enabled: Option<String>,
    #[serde(default)]
    #[allow(dead_code)]
    pub persist_login: Option<String>,
    pub csrftoken: String,
    #[serde(rename = "accountName")]
    pub account_name: String,
    #[serde(default)]
    #[allow(dead_code)]
    pub use_passkey: Option<String>,
}

// --- Handlers ---

/// `POST /login/{locale}/` — email submission step.
///
/// If the account exists, 302 to `/login/{locale}/password`. If not, 302 to
/// `/creation/` with error headers (captured wire format).
pub async fn submit_email(
    State(state): State<Arc<AppState>>,
    Path(locale): Path<String>,
    Query(query): Query<LoginQuery>,
    Form(form): Form<LoginEmailForm>,
) -> Response {
    let account = accounts::find_by_email(&state.pool, &form.account_name).await;

    match account {
        Ok(Some(_)) => {
            // Account exists → redirect to password page.
            let mut password_url = format!(
                "/login/{locale}/password?accountName={}",
                form.account_name.replace('@', "%40")
            );
            // Pass the OAuth ref through so the login redirect works.
            if let Some(ref r) = query.r#ref {
                password_url.push_str(&format!(
                    "&ref={}",
                    r.replace('%', "%25").replace('&', "%26")
                ));
            }
            let mut resp = Redirect::to(&password_url).into_response();
            let headers = resp.headers_mut();
            headers.insert("authentication-state", "LOGIN_CREDENTIAL".parse().unwrap());
            // Set bnet.extra cookie (capture: long-lived opaque value).
            let bnet_extra = generate_client_nonce();
            set_cookie(
                headers,
                "bnet.extra",
                &bnet_extra,
                "/login",
                2_147_483_647,
                &state.cookie_domain,
            );
            resp
        }
        Ok(None) => {
            // Account not found → redirect to creation flow.
            let mut resp = Redirect::to("/creation/flow/creation-full").into_response();
            let headers = resp.headers_mut();
            headers.insert("authentication-state", "LOGIN".parse().unwrap());
            headers.insert("error-code", "INVALID_ACCOUNT".parse().unwrap());
            resp
        }
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}

/// `GET /login/` — 1.14.0 game-client external challenge login entry point.
///
/// The 1.14.0 client hits `GET /login/?externalChallenge=login&app=wow` to
/// start the web SRP login flow. Redirect to the password page.
pub async fn external_login(
    State(state): State<Arc<AppState>>,
    Query(query): Query<LoginQuery>,
) -> Response {
    // The client expects the form to be served at this path.
    // We redirect to the /login/enUS/ password page so the user
    // sees the existing SRP login form. The redirect omits the
    // locale path segment since the game client doesn't expect it.
    let mut resp = Redirect::to("/login/enUS/").into_response();
    let headers = resp.headers_mut();
    let bnet_extra = generate_client_nonce();
    set_cookie(
        headers,
        "bnet.extra",
        &bnet_extra,
        "/login",
        2_147_483_647,
        &state.cookie_domain,
    );
    let _ = query;
    resp
}

/// `POST /login/srp?csrfToken=true` — SRP challenge (AJAX, JSON).
///
/// Request: `{"inputs":[{"input_id":"account_name","value":"<email>"}]}`.
/// Response: all 10 fields including `csrf_token`.
pub async fn srp_challenge(
    State(state): State<Arc<AppState>>,
    Query(query): Query<LoginQuery>,
    Json(req): Json<SrpChallengeRequest>,
) -> Result<Json<SrpChallengeResponse>, AccountError> {
    let account_name = req
        .inputs
        .iter()
        .find(|i| i.input_id == "account_name")
        .map(|i| i.value.as_str())
        .ok_or_else(|| AccountError::BadRequest("missing account_name input".into()))?;

    let account = accounts::find_by_email(&state.pool, account_name)
        .await?
        .ok_or(AccountError::NotFound)?;

    let cred = credentials::find_by_account_id(&state.pool, account.id)
        .await?
        .ok_or(AccountError::NotFound)?;

    let state_arc = state.clone();
    let verifier = BigInt::from_bytes_be(num_bigint::Sign::Plus, &cred.srp_verifier);
    let session = tokio::task::spawn_blocking(move || {
        let mut rng = rand::thread_rng();
        ServerSession::new(&state_arc.group, &verifier, &mut rng)
    })
    .await
    .map_err(|e| AccountError::Internal(format!("SRP session creation panicked: {e}")))?;

    let public_b = session.public_b().clone();

    // Generate CSRF token for this challenge (returned to the client, used in
    // the password submission).
    let csrf_token = uuid::Uuid::new_v4().to_string();

    // Store the pending challenge keyed by the CSRF token.
    state.challenges.insert(
        csrf_token.clone(),
        crate::PendingChallenge {
            session,
            account_id: account.id,
            expires_at: Instant::now() + Duration::from_secs(120),
        },
    );

    // Detect 1.14.0 game-client SRP login: `?externalChallenge=login&app=wow`.
    // The game client expects version 1 and iterations 1 (not the browser
    // values of version 2 and 15000).
    let is_game_client =
        query.external_challenge.as_deref() == Some("login") && query.app.as_deref() == Some("wow");

    let (srp_version, srp_iterations) = if is_game_client {
        (1u32, 1u32)
    } else {
        (2u32, cred.srp_iterations as u32)
    };

    Ok(Json(SrpChallengeResponse {
        modulus: state.modulus_hex.clone(),
        generator: "2".to_string(),
        hash_function: "SHA-256".to_string(),
        username: srp::srp_username(&account.email),
        salt: hex::encode_upper(&cred.srp_salt),
        public_b: hex::encode_upper(&public_b.to_bytes_be().1),
        version: srp_version,
        iterations: srp_iterations,
        eligible_credential_upgrade: false,
        csrf_token,
    }))
}

/// `POST /login/{locale}/password` — SRP proof verification (form-encoded).
///
/// On success: 302 redirect to `oauth/authorize?...&ST=<ticket>`.
/// On failure: 302 back to password page with error headers.
pub async fn srp_proof(
    State(state): State<Arc<AppState>>,
    Path(locale): Path<String>,
    Query(query): Query<LoginQuery>,
    Form(form): Form<SrpProofForm>,
) -> Response {
    match verify_srp_proof(&state, &form).await {
        Ok(account_id) => {
            // Mint login ticket and redirect to OAuth authorize.
            let ticket =
                match ticket::mint_login_ticket_with_region(&state.pool, account_id, &state.region)
                    .await
                {
                    Ok(t) => t,
                    Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
                };

            // Build the redirect URL. If `ref` is present (the original OAuth
            // authorize URL), append the ST. Otherwise redirect to overview.
            let redirect_url = if let Some(ref oauth_url) = query.r#ref {
                format!("{oauth_url}&ST={ticket}")
            } else {
                "/overview".to_string()
            };

            let mut resp = Redirect::to(&redirect_url).into_response();
            let headers = resp.headers_mut();
            headers.insert("authentication-state", "DONE".parse().unwrap());

            // Set all 6 SSO cookies matching the capture (section C).
            let login_key = generate_login_key();
            let client_nonce = generate_client_nonce();
            let ba_tassadar = format!("{region}-{login_key}-{account_id}", region = state.region);

            set_cookie(
                headers,
                "BA-tassadar",
                &ba_tassadar,
                "/login",
                31556900,
                &state.cookie_domain,
            );
            set_cookie(
                headers,
                "BA-tassadar-login.key",
                &login_key,
                "/",
                31556900,
                &state.cookie_domain,
            );
            set_cookie(
                headers,
                "login.key",
                &login_key,
                "/",
                31556900,
                &state.cookie_domain,
            );
            set_cookie(headers, "opt", "1", "/", 31556900, &state.cookie_domain);
            set_cookie(
                headers,
                "BA-tassadar-cl",
                &client_nonce,
                "/login",
                0,
                &state.cookie_domain,
            );
            set_cookie(
                headers,
                "cl",
                &client_nonce,
                "/login",
                0,
                &state.cookie_domain,
            );

            // If redirecting directly to /overview (no OAuth flow), create
            // a management session so the dashboard loads without an error.
            if !redirect_url.contains("authorize") {
                let session_id = uuid::Uuid::new_v4();
                let session = tavern_core::Session {
                    session_id,
                    account_id,
                    expires_at: chrono::Utc::now() + chrono::Duration::minutes(30),
                    created_at: chrono::Utc::now(),
                    user_agent: None,
                    ip: None,
                };
                let _ = tavern_db::sessions::insert(&state.pool, &session).await;
                set_cookie(
                    headers,
                    "SESSIONID",
                    &crate::session::encode_session_id(session_id),
                    "/",
                    1800,
                    &state.cookie_domain,
                );
            }
            resp
        }
        Err(e) => {
            // Redirect back to password page with error headers.
            // Preserve accountName so the user can retry without re-entering email.
            let mut fail_url = format!("/login/{locale}/password");
            if !form.account_name.is_empty() {
                fail_url.push_str(&format!(
                    "?accountName={}",
                    form.account_name.replace('@', "%40")
                ));
            }
            let mut resp = Redirect::to(&fail_url).into_response();
            let headers = resp.headers_mut();
            headers.insert("authentication-state", "LOGIN".parse().unwrap());
            headers.insert("error-code", "INVALID_CREDENTIALS".parse().unwrap());
            let _ = e; // error logged by verify_srp_proof
            resp
        }
    }
}

/// Verify the SRP proof, returning the account_id on success.
async fn verify_srp_proof(state: &Arc<AppState>, form: &SrpProofForm) -> Result<i64, AccountError> {
    let public_a = parse_hex_bigint(&form.public_a)?;
    let client_m1 = parse_hex_bigint(&form.client_evidence_m1)?;

    // Look up the pending challenge by CSRF token.
    let pending = state.challenges.remove(&form.csrftoken).map(|(_, v)| v);

    let mut pending = pending.ok_or(AccountError::ChallengeExpired)?;
    if pending.expires_at < Instant::now() {
        return Err(AccountError::ChallengeExpired);
    }

    let account_id = pending.account_id;
    let start = std::time::Instant::now();
    tokio::task::spawn_blocking(move || pending.session.verify(&public_a, &client_m1))
        .await
        .map_err(|e| AccountError::Internal(format!("SRP verify panicked: {e}")))?
        .ok_or(AccountError::InvalidCredentials)?;
    tavern_observability::record_srp_duration(start);

    tracing::info!(account_id, "SRP login succeeded");
    Ok(account_id)
}

fn parse_hex_bigint(hex_str: &str) -> Result<BigInt, AccountError> {
    let bytes = hex::decode(hex_str).map_err(|_| AccountError::InvalidHex)?;
    Ok(BigInt::from_bytes_be(num_bigint::Sign::Plus, &bytes))
}

/// Generate a random login key (32-char hex, matching the capture format).
fn generate_login_key() -> String {
    use rand::RngCore;
    let mut bytes = [0u8; 16];
    rand::thread_rng().fill_bytes(&mut bytes);
    hex::encode(bytes)
}

/// Generate a 64-char hex client nonce for the `BA-tassadar-cl` / `cl` cookies.
fn generate_client_nonce() -> String {
    use rand::RngCore;
    let mut bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut bytes);
    hex::encode(bytes)
}

/// Set a cookie with the given name, value, path, and max-age.
/// A max-age of 0 means session cookie (no Max-Age).
pub(crate) fn set_cookie(
    headers: &mut axum::http::HeaderMap,
    name: &str,
    value: &str,
    path: &str,
    max_age: u32,
    domain: &str,
) {
    let domain_part = if domain.is_empty() {
        String::new()
    } else {
        format!("; Domain={domain}")
    };
    // Dev mode: omit Secure + SameSite=None so cookies work over plain HTTP
    // (localhost). Production behind HTTPS keeps the full attributes.
    let insecure = std::env::var("INSECURE_COOKIES").is_ok_and(|v| v != "0");
    let (secure, samesite) = if insecure {
        ("", "; SameSite=Lax")
    } else {
        ("; Secure", "; SameSite=None")
    };
    let cookie = if max_age > 0 {
        format!(
            "{name}={value}; Path={path}{domain_part}; Max-Age={max_age}{secure}; HttpOnly{samesite}"
        )
    } else {
        format!("{name}={value}; Path={path}{domain_part}{secure}; HttpOnly{samesite}")
    };
    // SAFETY: cookie format is under our control, so parse should always work.
    headers.append(axum::http::header::SET_COOKIE, cookie.parse().unwrap());
}
