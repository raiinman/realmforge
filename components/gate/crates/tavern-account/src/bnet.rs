// SPDX-License-Identifier: AGPL-3.0-only

//! Bnetserver game-client login handlers.
//!
//! Serves `/bnetserver/login/` and `/bnetserver/login/srp/` with the
//! `Battlenet::JSON::Login::LoginForm` JSON envelope. One path, one envelope,
//! with a password-proof branch:
//!
//! - 1.13/1.14 Vanilla: plaintext `sha_pass_hash` (no SRP).
//! - 2.5+/3.4/4.4: BnetSRP6v2 proof (`public_A` + `client_evidence_M1`).
//!
//! All flows mint a login ticket via the shared M7 backend.

use std::sync::Arc;

use super::client_login::build_authenticator_challenge;
use axum::Json;
use axum::extract::State;
use axum::http::HeaderValue;
use axum::http::StatusCode;
use axum::http::header;
use axum::response::IntoResponse;
use num_bigint::BigInt;
use tracing;

use tavern_core::srp;
use tavern_db::{accounts, credentials};

use crate::bnet_types::*;
use crate::ticket;
use crate::{AccountError, AppState};

// --- Pending SRP sessions (one per account_name) ---

/// A pending SRP session from `/bnetserver/login/srp/`, awaiting the proof.
pub struct BnetSrpSession {
    pub session: srp::ServerSession,
    pub account_id: i64,
    pub email: String,
    pub expires_at: std::time::Instant,
}

// --- Handlers ---

/// `GET /bnetserver/login/` — returns the form definition with the SRP endpoint
/// URL. The client uses this to discover what inputs it must send.
pub async fn get_form(State(_state): State<Arc<AppState>>) -> impl IntoResponse {
    let form = FormInputs {
        form_type: "LOGIN_FORM".to_string(),
        inputs: vec![
            FormInputDefinition {
                input_id: "account_name".to_string(),
                input_type: "text".to_string(),
                label: "Email".to_string(),
                max_length: Some(320),
            },
            FormInputDefinition {
                input_id: "password".to_string(),
                input_type: "password".to_string(),
                label: "Password".to_string(),
                max_length: Some(128),
            },
        ],
        srp_url: "/bnetserver/login/srp/".to_string(),
        srp_js: None,
    };
    (
        StatusCode::OK,
        [("content-type", "application/json;charset=utf-8")],
        Json(form),
    )
}

/// `POST /bnetserver/login/srp/` — issues the SRP challenge for SRP-capable
/// clients (2.5+/3.4/4.4). The client sends a LoginForm with `account_name`;
/// the server responds with the SRP parameters.
pub async fn post_srp_challenge(
    State(state): State<Arc<AppState>>,
    Json(form): Json<LoginForm>,
) -> Result<impl IntoResponse, AccountError> {
    let account_name = form
        .input("account_name")
        .ok_or_else(|| AccountError::BadRequest("missing account_name".into()))?;

    let account = accounts::find_by_email(&state.pool, account_name)
        .await?
        .ok_or(AccountError::NotFound)?;

    let cred = credentials::find_by_account_id(&state.pool, account.id)
        .await?
        .ok_or(AccountError::NotFound)?;

    let verifier = BigInt::from_bytes_be(num_bigint::Sign::Plus, &cred.srp_verifier);

    let state_arc = state.clone();
    let session = tokio::task::spawn_blocking(move || {
        let mut rng = rand::thread_rng();
        srp::ServerSession::new(&state_arc.group, &verifier, &mut rng)
    })
    .await
    .map_err(|e| AccountError::Internal(format!("SRP session creation panicked: {e}")))?;
    let public_b = session.public_b().clone();

    // Store the pending session keyed by the lowercased email.
    let key = account_name.to_lowercase();

    if let Some(_existing) = state
        .bnet_sessions
        .get(&key)
        .filter(|e| e.expires_at > std::time::Instant::now())
    {
        return Err(AccountError::BadRequest("login already in progress".into()));
    }

    state.bnet_sessions.insert(
        key,
        BnetSrpSession {
            session,
            account_id: account.id,
            email: account.email.clone(),
            expires_at: std::time::Instant::now() + std::time::Duration::from_secs(120),
        },
    );

    let challenge = SrpLoginChallenge {
        version: 2,
        iterations: cred.srp_iterations as u32,
        modulus: state.modulus_hex.clone(),
        generator: "2".to_string(),
        hash_function: "SHA-256".to_string(),
        username: srp::srp_username(&account.email),
        salt: hex::encode_upper(&cred.srp_salt),
        public_b: hex::encode_upper(&public_b.to_bytes_be().1),
    };

    Ok((
        StatusCode::OK,
        [("content-type", "application/json;charset=utf-8")],
        Json(challenge),
    ))
}

/// `POST /bnetserver/login/` — verifies the credential proof and mints a login
/// ticket. Handles both the SRP proof (public_A + client_evidence_M1) and the
/// plaintext proof (password).
pub async fn post_login(
    State(state): State<Arc<AppState>>,
    Json(form): Json<LoginForm>,
) -> Result<impl IntoResponse, AccountError> {
    let account_name = form
        .input("account_name")
        .ok_or_else(|| AccountError::BadRequest("missing account_name".into()))?;

    // Try the SRP path: if a pending session exists for this account, verify
    // the SRP proof.
    let key = account_name.to_lowercase();
    let pending = state.bnet_sessions.remove(&key).map(|(_, v)| v);

    if let Some(mut pending) = pending {
        // SRP proof verification.
        let public_a_hex = form
            .input("public_A")
            .ok_or_else(|| AccountError::BadRequest("missing public_A for SRP login".into()))?;
        let m1_hex = form.input("client_evidence_M1").ok_or_else(|| {
            AccountError::BadRequest("missing client_evidence_M1 for SRP login".into())
        })?;

        let public_a = parse_hex_bigint(public_a_hex)?;
        let client_m1 = parse_hex_bigint(m1_hex)?;

        let start = std::time::Instant::now();
        let pa = public_a.clone();
        let cm1 = client_m1.clone();
        let s = tokio::task::spawn_blocking(move || pending.session.verify(&pa, &cm1))
            .await
            .map_err(|e| AccountError::Internal(format!("SRP verify panicked: {e}")))?
            .ok_or(AccountError::InvalidCredentials)?;
        tavern_observability::record_srp_duration(start);

        // Server evidence M2 (optional, for mutual authentication).
        let m2 = srp::ServerSession::server_evidence(&public_a, &client_m1, &s);
        let server_evidence_m2 = hex::encode_upper(m2.to_bytes_be().1);

        let login_ticket =
            ticket::mint_login_ticket_with_region(&state.pool, pending.account_id, &state.region)
                .await?;
        tracing::info!(account_id = pending.account_id, "bnet SRP login succeeded");

        // Check if the account has an authenticator.
        let has_2fa: bool =
            sqlx::query_scalar("SELECT has_authenticator FROM accounts WHERE id = $1")
                .bind(pending.account_id)
                .fetch_one(&state.pool)
                .await
                .map_err(|e| AccountError::Internal(format!("DB error: {e}")))?;

        if has_2fa {
            let sid = uuid::Uuid::new_v4().to_string();
            state
                .pending_authenticators
                .insert(sid.clone(), pending.account_id);
            return Ok(build_authenticator_challenge(
                pending.account_id,
                &sid,
                &state.base_url,
            ));
        }

        let result = LoginResult {
            authentication_state: "DONE".to_string(),
            server_evidence_m2: Some(server_evidence_m2),
            ..LoginResult::done(login_ticket)
        };
        let mut response = Json(result).into_response();
        response.headers_mut().insert(
            header::CONTENT_TYPE,
            HeaderValue::from_static("application/json;charset=utf-8"),
        );
        return Ok(response);
    }

    // Plaintext path (1.13/1.14 Vanilla): verify sha_pass_hash.
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

    // Check if the account has an authenticator.
    let has_2fa: bool = sqlx::query_scalar("SELECT has_authenticator FROM accounts WHERE id = $1")
        .bind(account.id)
        .fetch_one(&state.pool)
        .await
        .map_err(|e| AccountError::Internal(format!("DB error: {e}")))?;

    if has_2fa {
        let sid = uuid::Uuid::new_v4().to_string();
        state.pending_authenticators.insert(sid.clone(), account.id);
        return Ok(build_authenticator_challenge(
            account.id,
            &sid,
            &state.base_url,
        ));
    }

    let login_ticket =
        ticket::mint_login_ticket_with_region(&state.pool, account.id, &state.region).await?;
    tracing::info!(account_id = account.id, "bnet plaintext login succeeded");

    let result = LoginResult::done(login_ticket);
    let mut response = Json(result).into_response();
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/json;charset=utf-8"),
    );
    Ok(response)
}

/// `POST /bnetserver/gameAccounts/` — lists the account's game accounts.
pub async fn post_game_accounts(
    State(state): State<Arc<AppState>>,
    Json(form): Json<LoginForm>,
) -> Result<impl IntoResponse, AccountError> {
    let account_name = form
        .input("account_name")
        .ok_or_else(|| AccountError::BadRequest("missing account_name".into()))?;

    let account = accounts::find_by_email(&state.pool, account_name)
        .await?
        .ok_or(AccountError::NotFound)?;

    let rows = sqlx::query!(
        "SELECT name FROM game_accounts WHERE account_id = $1 ORDER BY id",
        account.id,
    )
    .fetch_all(&state.pool)
    .await?;

    let game_accounts: Vec<GameAccountInfo> = rows
        .into_iter()
        .map(|r| GameAccountInfo {
            display_name: r.name,
            expansion: 4, // Cataclysm
        })
        .collect();

    let list = GameAccountList { game_accounts };
    Ok((
        StatusCode::OK,
        [("content-type", "application/json;charset=utf-8")],
        Json(list),
    ))
}

/// `POST /bnetserver/refreshLoginTicket/` — returns a refreshed ticket expiry.
pub async fn post_refresh_login_ticket(
    State(_state): State<Arc<AppState>>,
    Json(_form): Json<LoginForm>,
) -> impl IntoResponse {
    let result = LoginRefreshResult {
        login_ticket_expiry: (chrono::Utc::now().timestamp() as u64) + 300,
        is_expired: Some(false),
    };
    (
        StatusCode::OK,
        [("content-type", "application/json;charset=utf-8")],
        Json(result),
    )
}

fn parse_hex_bigint(hex_str: &str) -> Result<BigInt, AccountError> {
    let bytes = hex::decode(hex_str).map_err(|_| AccountError::InvalidHex)?;
    Ok(BigInt::from_bytes_be(num_bigint::Sign::Plus, &bytes))
}
