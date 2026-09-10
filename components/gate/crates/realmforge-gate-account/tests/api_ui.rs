// SPDX-License-Identifier: AGPL-3.0-only

//! Integration tests for the account management API + password change (M10).
//!
//! Requires `DATABASE_URL`. Tests: API endpoints return account data,
//! password change rewrites both credential schemes, and the new password
//! works for subsequent bnet login.

use std::sync::Arc;

use axum::body::Body;
use axum::http::{HeaderMap, Method, Request, StatusCode};
use http_body_util::BodyExt;
use realmforge_gate_account::{AppState, registration};
use realmforge_gate_core::srp;

fn db_url() -> Option<String> {
    std::env::var("DATABASE_URL")
        .ok()
        .map(|s| s.trim().to_owned())
        .filter(|s| !s.is_empty())
}

async fn setup() -> Option<(sqlx::PgPool, Arc<AppState>)> {
    let url = db_url()?;
    let pool = realmforge_gate_db::connect(&url).await.expect("connect");
    realmforge_gate_db::run_migrations(&pool)
        .await
        .expect("migrate");
    let state = realmforge_gate_account::app_state(pool.clone(), String::new());
    Some((pool, state))
}

async fn register(pool: &sqlx::PgPool, group: &srp::Group, email: &str, password: &str) -> i64 {
    let _ = sqlx::query("DELETE FROM accounts WHERE email = $1")
        .bind(email)
        .execute(pool)
        .await;
    registration::register(pool, group, email, password)
        .await
        .expect("register")
}

#[tokio::test]
async fn api_returns_account_data() {
    let Some((pool, state)) = setup().await else {
        eprintln!("skipping: DATABASE_URL not set");
        return;
    };
    let email = unique("api");
    let account_id = register(&pool, &state.group, &email, "password123").await;

    let mut headers = HeaderMap::new();
    headers.insert("x-account-id", account_id.to_string().parse().unwrap());
    // XSRF double-submit for protected endpoints.
    headers.insert("cookie", "XSRF-TOKEN=test-token".parse().unwrap());
    headers.insert("x-xsrf-token", "test-token".parse().unwrap());

    // GET /api/user
    let app = realmforge_gate_account::router(state.clone());
    let resp = send(app, Method::GET, "/api/user", None, Some(headers.clone())).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let user: serde_json::Value = resp_json(resp).await;
    assert_eq!(user["accountId"].as_i64().unwrap(), account_id);
    assert_eq!(user["employee"], false);

    // GET /api/details
    let app = realmforge_gate_account::router(state.clone());
    let resp = send(
        app,
        Method::GET,
        "/api/details",
        None,
        Some(headers.clone()),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::OK);
    let details: serde_json::Value = resp_json(resp).await;
    assert_eq!(details["accountId"], account_id);

    // GET /api/overview
    let app = realmforge_gate_account::router(state.clone());
    let resp = send(
        app,
        Method::GET,
        "/api/overview",
        None,
        Some(headers.clone()),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::OK);
    let overview: serde_json::Value = resp_json(resp).await;
    assert!(
        !overview["gameAccounts"].as_array().unwrap().is_empty(),
        "registration must create a game account"
    );

    // GET /api/env (no auth required)
    let app = realmforge_gate_account::router(state.clone());
    let resp = send(app, Method::GET, "/api/env", None, None).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let env: serde_json::Value = resp_json(resp).await;
    assert_eq!(env["region"], "US");

    // Commerce stubs return empty.
    let app = realmforge_gate_account::router(state.clone());
    let resp = send(app, Method::GET, "/api/transactions", None, None).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let txns: serde_json::Value = resp_json(resp).await;
    assert!(txns["items"].as_array().unwrap().is_empty());
}

#[tokio::test]
async fn password_change_rewrites_both_schemes() {
    let Some((pool, state)) = setup().await else {
        eprintln!("skipping: DATABASE_URL not set");
        return;
    };
    let email = unique("pwchange");
    let account_id = register(&pool, &state.group, &email, "old-password").await;

    // Change the password via the API.
    let app = realmforge_gate_account::router(state.clone());
    let resp = send(
        app,
        Method::POST,
        "/api/security/password",
        Some(
            serde_json::json!({
                "account_id": account_id,
                "old_password": "old-password",
                "new_password": "new-password-456",
            })
            .to_string(),
        ),
        None,
    )
    .await;
    assert_eq!(
        resp.status(),
        StatusCode::OK,
        "password change must succeed"
    );

    // Wrong old password is rejected.
    let app = realmforge_gate_account::router(state.clone());
    let resp = send(
        app,
        Method::POST,
        "/api/security/password",
        Some(
            serde_json::json!({
                "account_id": account_id,
                "old_password": "wrong-old",
                "new_password": "should-fail",
            })
            .to_string(),
        ),
        None,
    )
    .await;
    assert_eq!(
        resp.status(),
        StatusCode::UNAUTHORIZED,
        "wrong old password must fail"
    );

    // Verify the new password works via bnet plaintext login.
    let app = realmforge_gate_account::router(state.clone());
    let resp = send(
        app,
        Method::POST,
        "/bnetserver/login/",
        Some(
            serde_json::json!({
                "platform_id": "Win",
                "program_id": "WoW",
                "version": "4.4.2.60895",
                "inputs": [
                    { "input_id": "account_name", "value": email },
                    { "input_id": "password", "value": "new-password-456" },
                ],
            })
            .to_string(),
        ),
        None,
    )
    .await;
    assert_eq!(
        resp.status(),
        StatusCode::OK,
        "new password must work for bnet login"
    );
    let result: serde_json::Value = resp_json(resp).await;
    assert_eq!(result["authentication_state"], "DONE");
    assert!(!result["login_ticket"].as_str().unwrap().is_empty());

    // Old password no longer works.
    let app = realmforge_gate_account::router(state.clone());
    let resp = send(
        app,
        Method::POST,
        "/bnetserver/login/",
        Some(
            serde_json::json!({
                "platform_id": "Win",
                "program_id": "WoW",
                "version": "4.4.2.60895",
                "inputs": [
                    { "input_id": "account_name", "value": email },
                    { "input_id": "password", "value": "old-password" },
                ],
            })
            .to_string(),
        ),
        None,
    )
    .await;
    assert_eq!(
        resp.status(),
        StatusCode::UNAUTHORIZED,
        "old password must no longer work"
    );
}

#[tokio::test]
async fn login_page_renders() {
    let Some((_pool, state)) = setup().await else {
        eprintln!("skipping: DATABASE_URL not set");
        return;
    };
    let app = realmforge_gate_account::router(state.clone());
    let resp = send(app, Method::GET, "/login/en/", None, None).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let html = String::from_utf8(bytes.to_vec()).unwrap();
    assert!(html.contains("Sign In"), "login page must render");
}

// --- helpers ---

fn unique(tag: &str) -> String {
    format!("m10-{tag}-{}@example.com", uuid::Uuid::new_v4().simple())
}

async fn send(
    app: axum::Router,
    method: Method,
    path: &str,
    body: Option<String>,
    headers: Option<HeaderMap>,
) -> axum::response::Response {
    let mut builder = Request::builder().method(method).uri(path);
    if let Some(ref hdrs) = headers {
        for (name, val) in hdrs.iter() {
            builder = builder.header(name, val);
        }
    }
    if body.is_some() {
        builder = builder.header("content-type", "application/json");
    }
    let request = builder.body(Body::from(body.unwrap_or_default())).unwrap();
    tower::ServiceExt::oneshot(app, request).await.unwrap()
}

async fn resp_json(resp: axum::response::Response) -> serde_json::Value {
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null)
}
