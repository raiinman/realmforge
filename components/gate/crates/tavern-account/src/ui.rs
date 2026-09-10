// SPDX-License-Identifier: AGPL-3.0-only

//! Server-rendered UI pages (`askama` templates).
//!
//! M10 provides the login page and the account dashboard. Both are simple HTML
//! with inline styles matching the Battle.net dark theme. The login page has
//! minimal JS for the SRP flow; the dashboard includes a password-change form.

use std::sync::Arc;

use askama::Template;
use axum::extract::{Path, State};
use axum::response::{Html, IntoResponse, Response};

use crate::AccountError;

/// The login page template.
#[derive(Template)]
#[template(path = "login.html")]
pub struct LoginPage {
    pub srp6a_js: String,
}

/// The landing page template.
#[derive(Template)]
#[template(path = "landing.html")]
pub struct LandingPage;

/// The account dashboard SPA template (no template variables — all data is
/// loaded by the SPA JavaScript via /api/ endpoints).
#[derive(Template)]
#[template(path = "dashboard.html")]
pub struct DashboardPage;

/// `GET /` — landing page with sign-in and registration links.
pub async fn landing() -> impl IntoResponse {
    Html(LandingPage.render().unwrap_or_default())
}

/// `GET /login/{locale}/` — the browser login page.
pub async fn login_page(
    State(_state): State<Arc<crate::AppState>>,
    Path(_locale): Path<String>,
) -> impl IntoResponse {
    Html(
        LoginPage {
            srp6a_js: include_str!("../static/srp6a.js").to_string(),
        }
        .render()
        .unwrap_or_default(),
    )
}

/// `GET /login/{locale}/password` — the password entry page.
///
/// Sets the `bnet.pam` cookie (matching the capture: `base64|PASSWORD`).
pub async fn password_page() -> impl IntoResponse {
    use crate::login::set_cookie;

    let mut resp = Html(
        LoginPage {
            srp6a_js: include_str!("../static/srp6a.js").to_string(),
        }
        .render()
        .unwrap_or_default(),
    )
    .into_response();
    let headers = resp.headers_mut();
    // authentication-state: LOGIN_CREDENTIAL (capture shows this on the password page GET).
    headers.insert("authentication-state", "LOGIN_CREDENTIAL".parse().unwrap());
    // bnet.pam: <random-base64>|PASSWORD (capture shows "PASSWORD" as the page type).
    let pam_payload = base64_pam_payload();
    set_cookie(
        headers,
        "bnet.pam",
        &format!("{pam_payload}|PASSWORD"),
        "/login",
        31_536_000,
        "",
    );
    resp
}

/// Generate a random base64 string for the `bnet.pam` cookie payload.
fn base64_pam_payload() -> String {
    use base64::Engine;
    use rand::RngCore;
    let mut bytes = [0u8; 24];
    rand::thread_rng().fill_bytes(&mut bytes);
    base64::engine::general_purpose::STANDARD.encode(bytes)
}

/// `GET /logout` — clear the session and redirect to the landing page.
pub async fn logout(
    State(state): State<Arc<crate::AppState>>,
    headers: axum::http::HeaderMap,
) -> impl IntoResponse {
    if let Some(session_id) = crate::session::extract_session_id(&headers) {
        let _ = tavern_db::sessions::delete(&state.pool, session_id).await;
        tracing::info!(%session_id, "web session ended");
    }
    (
        axum::http::StatusCode::FOUND,
        [
            (axum::http::header::LOCATION, "/"),
            (
                axum::http::header::SET_COOKIE,
                "SESSIONID=; Path=/; Max-Age=0; Secure; HttpOnly",
            ),
        ],
    )
}

/// `GET /overview` — the account dashboard SPA. Redirects to login if
/// unauthenticated. The SPA loads all data via /api/ endpoints.
pub async fn dashboard(
    State(state): State<Arc<crate::AppState>>,
    headers: axum::http::HeaderMap,
    query: axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Result<Response, AccountError> {
    let _account_id = if let Ok(id) = crate::session::resolve_session(&state, &headers).await {
        id
    } else if let Some(id) = headers
        .get("x-account-id")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.parse::<i64>().ok())
    {
        id
    } else {
        return Ok(axum::response::Redirect::to("/login/en/").into_response());
    };

    // The welcome email links to /overview?ticket=<sealed> (matches real
    // Battle.net). Consume the ticket here: open the sealed account id and
    // mark the email verified. The SPA reflects the verified state via
    // /api/overview.
    if let Some(ticket) = query
        .get("ticket")
        .and_then(|t| tavern_core::encrypted_ticket::open(t, &state.signing_key_pem).ok())
    {
        let _ = sqlx::query!(
            r#"UPDATE accounts SET email_verified = TRUE WHERE id = $1"#,
            ticket,
        )
        .execute(&state.pool)
        .await;
        tracing::info!(account_id = ticket, "email verified via /overview?ticket=");
    }

    let mut resp = Html(DashboardPage.render().unwrap_or_default()).into_response();
    // Set XSRF-TOKEN cookie so the SPA can call /api/* endpoints.
    // No Secure flag — the JS needs to read this cookie on any protocol.
    let xsrf_token = uuid::Uuid::new_v4().to_string();
    let xsrf_cookie = format!("XSRF-TOKEN={xsrf_token}; Path=/; SameSite=Lax");
    resp.headers_mut()
        .append(axum::http::header::SET_COOKIE, xsrf_cookie.parse().unwrap());
    Ok(resp)
}

/// `GET /device` — device authorization verification page.
///
/// Presents a form to enter the user code shown on the device.
/// On submit, POSTs to `/device/approve` on the OAuth server.
pub async fn device_page() -> impl IntoResponse {
    Html(
        r#"<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <meta name="color-scheme" content="light dark">
  <title>Device Authorization - Tavern</title>
  <link rel="stylesheet" href="/static/style.css">
  <link rel="icon" type="image/svg+xml" href="/favicon.svg">
</head>
<body class="page-center">
  <div class="card" style="text-align:center">
    <div style="margin-bottom: var(--space-md)">
      <img class="logo-mark logo-on-light" src="/static/tavern-logo-light.png"
           width="64" height="64" alt="Tavern" />
      <img class="logo-mark logo-on-dark" src="/static/tavern-logo-dark.png"
           width="64" height="64" alt="Tavern" />
    </div>
    <h1>Device Authorization</h1>
    <p>Enter the code shown on your device.</p>
    <form method="post" action="/device/approve" style="margin-top:1rem">
      <div class="form-group">
        <label for="code">User Code</label>
        <input type="text" id="code" name="user_code" placeholder="ABCD-EFGH"
               maxlength="9" autocomplete="off" required
               style="text-align:center;font-size:1.25rem;letter-spacing:0.1em;text-transform:uppercase">
      </div>
      <button type="submit" class="btn btn-primary">Approve</button>
    </form>
  </div>
</body>
</html>"#,
    )
}
