// SPDX-License-Identifier: AGPL-3.0-only

//! Integration tests for the bnetserver game-client login (M8).
//!
//! Tests both the SRP path (2.5+/3.4/4.4) and the plaintext path (1.13/1.14
//! Vanilla). Requires `DATABASE_URL`.

use std::sync::Arc;

use axum::body::Body;
use axum::http::{Method, Request, StatusCode};
use http_body_util::BodyExt;
use tavern_account::{AppState, registration};
use tavern_core::srp;

fn db_url() -> Option<String> {
    std::env::var("DATABASE_URL")
        .ok()
        .map(|s| s.trim().to_owned())
        .filter(|s| !s.is_empty())
}

async fn setup() -> Option<(sqlx::PgPool, Arc<AppState>)> {
    let url = db_url()?;
    let pool = tavern_db::connect(&url).await.expect("connect");
    tavern_db::run_migrations(&pool).await.expect("migrate");
    let state = tavern_account::app_state(pool.clone(), String::new());
    Some((pool, state))
}

async fn register(pool: &sqlx::PgPool, group: &srp::Group, email: &str, password: &str) {
    let _ = sqlx::query("DELETE FROM accounts WHERE email = $1")
        .bind(email)
        .execute(pool)
        .await;
    registration::register(pool, group, email, password)
        .await
        .expect("register");
}

fn login_form(account_name: &str, extras: &[(&str, &str)]) -> serde_json::Value {
    let mut inputs = vec![serde_json::json!({ "input_id": "account_name", "value": account_name })];
    for (id, val) in extras {
        inputs.push(serde_json::json!({ "input_id": id, "value": val }));
    }
    serde_json::json!({
        "platform_id": "Win",
        "program_id": "WoW",
        "version": "4.4.2.60895",
        "inputs": inputs,
    })
}

#[tokio::test]
async fn bnet_srp_login_succeeds() {
    let Some((pool, state)) = setup().await else {
        eprintln!("skipping: DATABASE_URL not set");
        return;
    };
    let email = unique("srp");
    let password = "correct horse battery staple";
    register(&pool, &state.group, &email, password).await;

    // 1. GET form.
    let app = tavern_account::router(state.clone());
    let resp = oneshot(app, GET, "/bnetserver/login/").await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: serde_json::Value = resp_body(resp).await;
    assert_eq!(body["type"], "LOGIN_FORM");
    assert_eq!(body["srp_url"], "/bnetserver/login/srp/");

    // 2. POST /srp/ to get the challenge.
    let app = tavern_account::router(state.clone());
    let resp = oneshot_json(app, POST, "/bnetserver/login/srp/", login_form(&email, &[])).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let challenge: serde_json::Value = resp_body(resp).await;
    assert_eq!(challenge["version"], 2);
    assert_eq!(challenge["generator"], "2");
    assert!(!challenge["public_B"].as_str().unwrap().is_empty());

    let public_b = parse_hex(&challenge, "public_B");
    let salt = hex::decode(challenge["salt"].as_str().unwrap()).unwrap();

    // 3. Compute client proof.
    let mut rng = rand::thread_rng();
    let client_a = gen_below(&mut rng, &(&state.group.n - num_bigint::BigInt::from(1u32)));
    let (big_a, m1) = srp::client_proof(
        &state.group,
        &email,
        password,
        &salt,
        srp::ITERATIONS,
        &public_b,
        &client_a,
    );

    // 4. POST /login/ with proof.
    let app = tavern_account::router(state.clone());
    let resp = oneshot_json(
        app,
        POST,
        "/bnetserver/login/",
        login_form(
            &email,
            &[
                ("public_A", &hex::encode_upper(big_a.to_bytes_be().1)),
                ("client_evidence_M1", &hex::encode_upper(m1.to_bytes_be().1)),
            ],
        ),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::OK);
    let result: serde_json::Value = resp_body(resp).await;
    assert_eq!(result["authentication_state"], "DONE");
    assert!(!result["login_ticket"].as_str().unwrap().is_empty());
}

#[tokio::test]
async fn bnet_plaintext_login_succeeds() {
    let Some((pool, state)) = setup().await else {
        eprintln!("skipping: DATABASE_URL not set");
        return;
    };
    let email = unique("plain");
    let password = "my-password-123";
    register(&pool, &state.group, &email, password).await;

    let app = tavern_account::router(state.clone());
    let resp = oneshot_json(
        app,
        POST,
        "/bnetserver/login/",
        login_form(&email, &[("password", password)]),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::OK);
    let result: serde_json::Value = resp_body(resp).await;
    assert_eq!(result["authentication_state"], "DONE");
    assert!(!result["login_ticket"].as_str().unwrap().is_empty());
}

#[tokio::test]
async fn bnet_srp_wrong_password_rejected() {
    let Some((pool, state)) = setup().await else {
        eprintln!("skipping: DATABASE_URL not set");
        return;
    };
    let email = unique("wrong");
    register(&pool, &state.group, &email, "real-password").await;

    // Get a valid challenge with the real account.
    let app = tavern_account::router(state.clone());
    let resp = oneshot_json(app, POST, "/bnetserver/login/srp/", login_form(&email, &[])).await;
    let challenge: serde_json::Value = resp_body(resp).await;
    let public_b = parse_hex(&challenge, "public_B");
    let salt = hex::decode(challenge["salt"].as_str().unwrap()).unwrap();

    // Compute proof with WRONG password.
    let mut rng = rand::thread_rng();
    let client_a = gen_below(&mut rng, &(&state.group.n - num_bigint::BigInt::from(1u32)));
    let (big_a, m1) = srp::client_proof(
        &state.group,
        &email,
        "wrong-password",
        &salt,
        srp::ITERATIONS,
        &public_b,
        &client_a,
    );

    let app = tavern_account::router(state.clone());
    let resp = oneshot_json(
        app,
        POST,
        "/bnetserver/login/",
        login_form(
            &email,
            &[
                ("public_A", &hex::encode_upper(big_a.to_bytes_be().1)),
                ("client_evidence_M1", &hex::encode_upper(m1.to_bytes_be().1)),
            ],
        ),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn bnet_game_accounts() {
    let Some((pool, state)) = setup().await else {
        eprintln!("skipping: DATABASE_URL not set");
        return;
    };
    let email = unique("games");
    register(&pool, &state.group, &email, "pw").await;

    let app = tavern_account::router(state.clone());
    let resp = oneshot_json(
        app,
        POST,
        "/bnetserver/gameAccounts/",
        login_form(&email, &[]),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::OK);
    let list: serde_json::Value = resp_body(resp).await;
    assert!(
        !list["game_accounts"].as_array().unwrap().is_empty(),
        "registration must create a default game account"
    );
}

// --- helpers ---

const GET: Method = Method::GET;
const POST: Method = Method::POST;

fn unique(tag: &str) -> String {
    format!("m8-{tag}-{}@example.com", uuid::Uuid::new_v4().simple())
}

fn parse_hex(json: &serde_json::Value, key: &str) -> num_bigint::BigInt {
    let bytes = hex::decode(json[key].as_str().unwrap()).unwrap();
    num_bigint::BigInt::from_bytes_be(num_bigint::Sign::Plus, &bytes)
}

fn gen_below(rng: &mut impl rand::RngCore, bound: &num_bigint::BigInt) -> num_bigint::BigInt {
    use num_bigint::Sign;
    let bits = bound.bits() as usize;
    let byte_len = bits.div_ceil(8);
    loop {
        let mut bytes = vec![0u8; byte_len];
        rng.fill_bytes(&mut bytes);
        let candidate = num_bigint::BigInt::from_bytes_be(Sign::Plus, &bytes);
        if candidate < *bound {
            return candidate;
        }
    }
}

async fn oneshot(app: axum::Router, method: Method, path: &str) -> axum::response::Response {
    tower::ServiceExt::oneshot(
        app,
        Request::builder()
            .method(method)
            .uri(path)
            .body(Body::empty())
            .unwrap(),
    )
    .await
    .unwrap()
}

async fn oneshot_json(
    app: axum::Router,
    method: Method,
    path: &str,
    body: serde_json::Value,
) -> axum::response::Response {
    tower::ServiceExt::oneshot(
        app,
        Request::builder()
            .method(method)
            .uri(path)
            .header("content-type", "application/json")
            .body(Body::from(body.to_string()))
            .unwrap(),
    )
    .await
    .unwrap()
}

async fn resp_body(resp: axum::response::Response) -> serde_json::Value {
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null)
}
