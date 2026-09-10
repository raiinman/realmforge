// SPDX-License-Identifier: AGPL-3.0-only

//! OAuth callback handler — exchanges the authorization code for a management
//! session.
//!
//! After the OAuth provider redirects to `/callback/oauth2/code/account-settings`
//! with a `code`, this handler:
//! 1. Looks up the authorization code in the database (M6).
//! 2. Creates a management session (30-min TTL).
//! 3. Sets `SESSIONID` (HttpOnly) and `XSRF-TOKEN` (JS-readable) cookies.
//! 4. Redirects to `/` (the SPA shell).

use std::sync::Arc;

use axum::extract::{Query, State};
use axum::http::header;
use axum::response::{IntoResponse, Redirect};
use chrono::{Duration, Utc};
use serde::Deserialize;
use uuid::Uuid;

use realmforge_gate_db::{authorization_codes, sessions};

use crate::{AccountError, AppState};

#[derive(Deserialize)]
pub struct CallbackParams {
    pub code: String,
    #[serde(default)]
    #[allow(dead_code)]
    pub state: Option<String>,
}

/// `GET /callback/oauth2/code/account-settings` — exchange the OAuth code for
/// a management session. Sets SESSIONID (30-min) + XSRF-TOKEN cookies.
pub async fn oauth_callback(
    State(state): State<Arc<AppState>>,
    Query(params): Query<CallbackParams>,
) -> Result<axum::response::Response, AccountError> {
    // Consume the authorization code.
    let auth_code = authorization_codes::consume(&state.pool, &params.code)
        .await?
        .ok_or_else(|| AccountError::BadRequest("invalid or expired authorization code".into()))?;

    // Create the management session.
    let session_id = Uuid::new_v4();
    let xsrf_token = Uuid::new_v4().to_string();

    let session = realmforge_gate_core::Session {
        session_id,
        account_id: auth_code.account_id,
        expires_at: Utc::now() + Duration::minutes(30),
        created_at: Utc::now(),
        user_agent: None,
        ip: None,
    };
    sessions::insert(&state.pool, &session).await?;

    tracing::info!(
        account_id = auth_code.account_id,
        session_id = %session_id,
        "management session established"
    );

    // Build the response: redirect to / with cookies.
    let insecure = std::env::var("INSECURE_COOKIES").is_ok_and(|v| v != "0");
    let (session_cookie, xsrf_cookie) = if insecure {
        (
            format!(
                "SESSIONID={}; Path=/; Max-Age=1800; HttpOnly; SameSite=Lax",
                crate::session::encode_session_id(session_id)
            ),
            format!("XSRF-TOKEN={xsrf_token}; Path=/; HttpOnly; SameSite=Lax"),
        )
    } else {
        (
            format!(
                "SESSIONID={}; Path=/; Max-Age=1800; Secure; HttpOnly; SameSite=None",
                crate::session::encode_session_id(session_id)
            ),
            format!("XSRF-TOKEN={xsrf_token}; Path=/; Secure; HttpOnly; SameSite=None"),
        )
    };

    let mut resp = Redirect::to("/").into_response();
    let headers = resp.headers_mut();
    headers.append(header::SET_COOKIE, session_cookie.parse().unwrap());
    headers.append(header::SET_COOKIE, xsrf_cookie.parse().unwrap());

    Ok(resp)
}
