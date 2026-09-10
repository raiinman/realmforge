// SPDX-License-Identifier: AGPL-3.0-only

//! Integration tests for the 1.13.2 in-client LEGAL (ToS) login gate.
//!
//! Covers realmforge-interop-analysis.md §4.11a: password-verified accounts
//! that have not accepted the current ToS version receive
//! `authentication_state: "LEGAL"` with `next_url` + `legal_form`, and no
//! `login_ticket` is handed out until the client POSTs `accept_beula` /
//! `accept_chat` to the acceptance endpoint. Requires `DATABASE_URL`.

use std::sync::Arc;

use axum::body::Body;
use axum::http::{Method, Request, StatusCode, header};
use http_body_util::BodyExt;
use realmforge_gate_account::{AppState, registration};

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

async fn register(
    pool: &sqlx::PgPool,
    group: &realmforge_gate_core::srp::Group,
    email: &str,
    password: &str,
) {
    let _ = sqlx::query("DELETE FROM accounts WHERE email = $1")
        .bind(email)
        .execute(pool)
        .await;
    registration::register(pool, group, email, password)
        .await
        .expect("register");
}

fn unique(tag: &str) -> String {
    format!("tos-{tag}-{}@example.com", uuid::Uuid::new_v4().simple())
}

fn client_login_form(account_name: &str, password: &str) -> serde_json::Value {
    serde_json::json!({
        "program_id": "WoW",
        "platform_id": "Wn64",
        "version": "1.13.2.31650",
        "inputs": [
            { "input_id": "account_name", "value": account_name },
            { "input_id": "password", "value": password },
        ],
    })
}

fn tos_accept_form() -> serde_json::Value {
    serde_json::json!({
        "program_id": "WoW",
        "platform_id": "Wn64",
        "inputs": [
            { "input_id": "accept_beula", "value": "true" },
            { "input_id": "accept_chat", "value": "true" },
        ],
    })
}

async fn post_json(
    app: axum::Router,
    path: &str,
    body: serde_json::Value,
    cookie: Option<&str>,
) -> axum::response::Response {
    let mut builder = Request::builder()
        .method(Method::POST)
        .uri(path)
        .header("content-type", "application/json");
    if let Some(c) = cookie {
        builder = builder.header(header::COOKIE, c);
    }
    tower::ServiceExt::oneshot(app, builder.body(Body::from(body.to_string())).unwrap())
        .await
        .unwrap()
}

async fn get(app: axum::Router, path: &str) -> axum::response::Response {
    tower::ServiceExt::oneshot(
        app,
        Request::builder()
            .method(Method::GET)
            .uri(path)
            .body(Body::empty())
            .unwrap(),
    )
    .await
    .unwrap()
}

async fn resp_body(resp: axum::response::Response) -> serde_json::Value {
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null)
}

fn jsessionid(resp: &axum::response::Response) -> Option<String> {
    resp.headers()
        .get_all(header::SET_COOKIE)
        .iter()
        .filter_map(|v| v.to_str().ok())
        .find_map(|c| {
            let (name, rest) = c.split_once('=')?;
            (name.trim() == "JSESSIONID").then(|| rest.split(';').next().unwrap_or("").to_string())
        })
}

#[tokio::test]
async fn client_login_legal_then_accept_then_done() {
    let Some((pool, state)) = setup().await else {
        eprintln!("skipping: DATABASE_URL not set");
        return;
    };
    let email = unique("legal");
    let password = "tos-password-1";
    register(&pool, &state.group, &email, password).await;

    // 1. First login: account has not accepted ToS -> LEGAL, no ticket.
    let app = realmforge_gate_account::router(state.clone());
    let resp = post_json(
        app,
        "/client/login/external?targetRegion=us",
        client_login_form(&email, password),
        None,
    )
    .await;
    assert_eq!(resp.status(), StatusCode::OK);
    let sid = jsessionid(&resp).expect("JSESSIONID cookie on LEGAL challenge");
    let body: serde_json::Value = resp_body(resp).await;
    assert_eq!(body["authentication_state"], "LEGAL");
    assert_eq!(body["login_ticket"], serde_json::Value::Null);
    let next_url = body["next_url"].as_str().expect("next_url");
    assert!(next_url.ends_with("/client/login/tos/accept"));
    let agreement_url = body["legal_form"]["agreements"][0]["url"]
        .as_str()
        .expect("agreement url");
    assert!(agreement_url.ends_with(&format!("/legal/agreement/{}", state.tos_version)));

    // 2. No ticket is handed out before acceptance.
    assert_eq!(body["login_ticket"], serde_json::Value::Null);

    // 3. Accept via the next_url endpoint with the session cookie.
    let app = realmforge_gate_account::router(state.clone());
    let resp = post_json(
        app,
        "/client/login/tos/accept",
        tos_accept_form(),
        Some(&format!("JSESSIONID={sid}")),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::OK);
    assert!(
        jsessionid(&resp).is_some(),
        "DONE carries a fresh JSESSIONID"
    );
    let body: serde_json::Value = resp_body(resp).await;
    assert_eq!(body["authentication_state"], "DONE");
    assert!(!body["login_ticket"].as_str().unwrap().is_empty());

    // 4. Acceptance is persisted per account.
    let tos_version: String = sqlx::query_scalar(
        "SELECT tos_version FROM accounts WHERE id = (SELECT id FROM accounts WHERE email = $1)",
    )
    .bind(&email)
    .fetch_one(&pool)
    .await
    .expect("tos_version");
    assert_eq!(tos_version, state.tos_version);

    // 5. Next login skips the LEGAL gate.
    let app = realmforge_gate_account::router(state.clone());
    let resp = post_json(
        app,
        "/client/login/external?targetRegion=us",
        client_login_form(&email, password),
        None,
    )
    .await;
    let body: serde_json::Value = resp_body(resp).await;
    assert_eq!(body["authentication_state"], "DONE");
}

#[tokio::test]
async fn client_login_tos_accept_requires_consent_and_session() {
    let Some((pool, state)) = setup().await else {
        eprintln!("skipping: DATABASE_URL not set");
        return;
    };
    let email = unique("consent");
    register(&pool, &state.group, &email, "pw").await;

    // Get a LEGAL challenge to obtain the session.
    let app = realmforge_gate_account::router(state.clone());
    let resp = post_json(
        app,
        "/client/login/external?targetRegion=us",
        client_login_form(&email, "pw"),
        None,
    )
    .await;
    let sid = jsessionid(&resp).expect("JSESSIONID");

    // Accept without consent inputs -> 400.
    let app = realmforge_gate_account::router(state.clone());
    let resp = post_json(
        app,
        "/client/login/tos/accept",
        serde_json::json!({ "inputs": [] }),
        Some(&format!("JSESSIONID={sid}")),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

    // Accept without a session cookie -> 400.
    let app = realmforge_gate_account::router(state.clone());
    let resp = post_json(app, "/client/login/tos/accept", tos_accept_form(), None).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

    // No ticket was handed out in any branch.
    let tos_version: String = sqlx::query_scalar(
        "SELECT tos_version FROM accounts WHERE id = (SELECT id FROM accounts WHERE email = $1)",
    )
    .bind(&email)
    .fetch_one(&pool)
    .await
    .expect("tos_version");
    assert_eq!(tos_version, "", "ToS must remain unaccepted");
}

#[tokio::test]
async fn agreement_document_served_only_for_current_version() {
    let Some((_, state)) = setup().await else {
        eprintln!("skipping: DATABASE_URL not set");
        return;
    };

    let app = realmforge_gate_account::router(state.clone());
    let resp = get(app, &format!("/legal/agreement/{}", state.tos_version)).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let app = realmforge_gate_account::router(state.clone());
    let resp = get(app, "/legal/agreement/1900-01-01").await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}
