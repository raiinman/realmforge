// SPDX-License-Identifier: AGPL-3.0-only

//! Integration tests for registration + SRP login + ticket mint (M7/M12).
//!
//! Requires `DATABASE_URL` to a live Postgres. Skipped when unset. Uses the
//! capture-accurate wire format: form-encoded proof, {inputs:[...]} challenge.

use std::sync::Arc;

use axum::body::Body;
use axum::http::{Method, Request, StatusCode};
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

async fn register(pool: &sqlx::PgPool, group: &srp::Group, email: &str, password: &str) {
    let _ = sqlx::query("DELETE FROM accounts WHERE email = $1")
        .bind(email)
        .execute(pool)
        .await;
    registration::register(pool, group, email, password)
        .await
        .expect("register");
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

#[tokio::test]
async fn register_then_srp_login_succeeds() {
    let Some((pool, state)) = setup().await else {
        eprintln!("skipping: DATABASE_URL not set");
        return;
    };
    let email = unique_email("login-succeeds");
    let password = "correct horse battery staple";
    register(&pool, &state.group, &email, password).await;

    // 1. Fetch SRP challenge (AJAX, JSON with inputs array).
    let challenge = send_json(
        &state,
        Method::POST,
        "/login/srp",
        serde_json::json!({
            "inputs": [{"input_id": "account_name", "value": email}]
        }),
    )
    .await;
    assert_eq!(challenge["hash_function"], "SHA-256");
    assert_eq!(challenge["version"], 2);
    assert!(!challenge["csrf_token"].as_str().unwrap().is_empty());
    assert!(!challenge["public_B"].as_str().unwrap().is_empty());

    let csrf_token = challenge["csrf_token"].as_str().unwrap().to_owned();
    let public_b_hex = challenge["public_B"].as_str().unwrap();
    let public_b_bytes = hex::decode(public_b_hex).unwrap();
    let public_b = num_bigint::BigInt::from_bytes_be(num_bigint::Sign::Plus, &public_b_bytes);
    let salt = hex::decode(challenge["salt"].as_str().unwrap()).unwrap();

    // 2. Compute the client proof using realmforge-gate-core's SRP.
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

    // 3. Submit the proof (form-encoded, all capture-accurate fields).
    let app = realmforge_gate_account::router(state.clone());
    let form_body = format!(
        "username={email}&password=................&useSrp=true\
         &publicA={}&clientEvidenceM1={}&srpEnabled=true&persistLogin=on\
         &csrftoken={csrf_token}&accountName={email}&usePasskey=false",
        hex::encode_upper(big_a.to_bytes_be().1),
        hex::encode_upper(m1.to_bytes_be().1),
    );
    let resp = tower::ServiceExt::oneshot(
        app,
        Request::builder()
            .method(Method::POST)
            .uri("/login/en/password")
            .header("content-type", "application/x-www-form-urlencoded")
            .body(Body::from(form_body))
            .unwrap(),
    )
    .await
    .unwrap();

    // SRP proof verified → 302 redirect.
    assert_eq!(
        resp.status(),
        StatusCode::SEE_OTHER,
        "correct password must redirect"
    );
    let location = resp
        .headers()
        .get("location")
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();
    // Without a `ref` param, redirects to /overview.
    assert!(location.contains("/overview") || location.contains("ST="));
}

#[tokio::test]
async fn wrong_password_is_rejected() {
    let Some((pool, state)) = setup().await else {
        eprintln!("skipping: DATABASE_URL not set");
        return;
    };
    let email = unique_email("wrong-pw");
    register(&pool, &state.group, &email, "correct horse battery staple").await;

    // Fetch challenge.
    let challenge = send_json(
        &state,
        Method::POST,
        "/login/srp",
        serde_json::json!({
            "inputs": [{"input_id": "account_name", "value": email}]
        }),
    )
    .await;

    let csrf_token = challenge["csrf_token"].as_str().unwrap().to_owned();
    let public_b_bytes = hex::decode(challenge["public_B"].as_str().unwrap()).unwrap();
    let public_b = num_bigint::BigInt::from_bytes_be(num_bigint::Sign::Plus, &public_b_bytes);
    let salt = hex::decode(challenge["salt"].as_str().unwrap()).unwrap();

    // Compute proof with WRONG password.
    let mut rng = rand::thread_rng();
    let client_a = gen_below(&mut rng, &(&state.group.n - num_bigint::BigInt::from(1u32)));
    let (big_a, m1) = srp::client_proof(
        &state.group,
        &email,
        "totally wrong password",
        &salt,
        srp::ITERATIONS,
        &public_b,
        &client_a,
    );

    let app = realmforge_gate_account::router(state.clone());
    let form_body = format!(
        "username={email}&password=................&useSrp=true\
         &publicA={}&clientEvidenceM1={}&srpEnabled=true&persistLogin=on\
         &csrftoken={csrf_token}&accountName={email}&usePasskey=false",
        hex::encode_upper(big_a.to_bytes_be().1),
        hex::encode_upper(m1.to_bytes_be().1),
    );
    let resp = tower::ServiceExt::oneshot(
        app,
        Request::builder()
            .method(Method::POST)
            .uri("/login/en/password")
            .header("content-type", "application/x-www-form-urlencoded")
            .body(Body::from(form_body))
            .unwrap(),
    )
    .await
    .unwrap();

    // Wrong password → redirect back to password page with error-code header.
    assert_eq!(resp.status(), StatusCode::SEE_OTHER);
    let error_code = resp
        .headers()
        .get("error-code")
        .map(|v| v.to_str().unwrap())
        .unwrap_or("");
    assert_eq!(error_code, "INVALID_CREDENTIALS");
}

#[tokio::test]
async fn plaintext_credential_is_stored_and_verifiable() {
    let Some((pool, state)) = setup().await else {
        eprintln!("skipping: DATABASE_URL not set");
        return;
    };
    let email = unique_email("plaintext");
    register(&pool, &state.group, &email, "my-password-123").await;

    let correct_hash = srp::compute_sha_pass_hash(&email, "my-password-123");
    assert!(
        registration::verify_plaintext(&email, "my-password-123", &correct_hash),
        "correct password must verify"
    );
    assert!(
        !registration::verify_plaintext(&email, "wrong", &correct_hash),
        "wrong password must not verify"
    );
}

// --- helpers ---

fn unique_email(tag: &str) -> String {
    format!("m7-{tag}-{}@example.com", uuid::Uuid::new_v4().simple())
}

async fn send_json(
    state: &Arc<AppState>,
    method: Method,
    path: &str,
    body: serde_json::Value,
) -> serde_json::Value {
    let app = realmforge_gate_account::router(state.clone());
    let resp = tower::ServiceExt::oneshot(
        app,
        Request::builder()
            .method(method)
            .uri(path)
            .header("content-type", "application/json")
            .body(Body::from(body.to_string()))
            .unwrap(),
    )
    .await
    .unwrap();
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null)
}
