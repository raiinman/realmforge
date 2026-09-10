// SPDX-License-Identifier: AGPL-3.0-only

//! Integration tests: create one account per locale and verify welcome emails.
//!
//! Run with `DATABASE_URL=postgres://...` and mailcrab on port 1025.
//! Check emails at http://localhost:1080.

use std::sync::Arc;

use axum::body::Body;
use axum::http::{HeaderMap, Method, Request, StatusCode};
use http_body_util::BodyExt;
use tavern_account::AppState;

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

async fn create_account(app: &axum::Router, email: &str, country: &str) -> i64 {
    // Get creation page + session cookie.
    let (status, resp_headers, resp) = send_json(
        app,
        Method::GET,
        "/creation/api/init",
        serde_json::Value::Null,
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let initial_csrf = resp["csrfToken"].as_str().unwrap().to_string();

    let session_cookie = resp_headers
        .get_all("set-cookie")
        .iter()
        .filter_map(|v| v.to_str().ok())
        .find(|c| c.starts_with("creation-session"))
        .map(|c| {
            c.split(';')
                .next()
                .unwrap_or("")
                .strip_prefix("creation-session=")
                .unwrap_or("")
                .to_string()
        })
        .unwrap();

    let mut headers = HeaderMap::new();
    headers.insert(
        "cookie",
        format!("creation-session={session_cookie}")
            .parse()
            .unwrap(),
    );

    // Step through the creation flow.
    let csrf = post_step(
        app,
        &headers,
        "get-started",
        &serde_json::json!({
            "_csrf": initial_csrf, "country": country,
            "dob-year": "1990", "dob-month": "01", "dob-day": "15"
        }),
    )
    .await;

    let csrf = post_step(
        app,
        &headers,
        "provide-name",
        &serde_json::json!({
            "_csrf": csrf, "firstName": "Test", "lastName": country
        }),
    )
    .await;

    let csrf = post_step(
        app,
        &headers,
        "provide-credentials",
        &serde_json::json!({
            "_csrf": csrf, "email": email
        }),
    )
    .await;

    let csrf = post_step(
        app,
        &headers,
        "sms-captcha-gate",
        &serde_json::json!({
            "_csrf": csrf
        }),
    )
    .await;

    let csrf = post_step(
        app,
        &headers,
        "phone-number-verification",
        &serde_json::json!({
            "_csrf": csrf, "sms-verification-code": "123456"
        }),
    )
    .await;

    let csrf = post_step(
        app,
        &headers,
        "legal-and-opt-ins",
        &serde_json::json!({
            "_csrf": csrf, "tou-agreements-implicit": "x;1"
        }),
    )
    .await;

    // Set-password — account is created.
    let (status, _, resp) = send_multipart(
        app,
        "/creation/flow/creation-full/step/set-password",
        &serde_json::json!({
            "_csrf": csrf, "password": "test-password-456"
        }),
        Some(headers),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let account_id = resp["accountId"]
        .as_i64()
        .expect("accountId must be returned");
    assert!(account_id > 0);

    account_id
}

async fn post_step(
    app: &axum::Router,
    headers: &HeaderMap,
    step: &str,
    body: &serde_json::Value,
) -> String {
    let (status, _, resp) = send_multipart(
        app,
        &format!("/creation/flow/creation-full/step/{step}"),
        body,
        Some(headers.clone()),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "step {step} failed: {resp}");
    resp["csrfToken"].as_str().unwrap().to_string()
}

/// Send a multipart/form-data request where each JSON field becomes a form field.
fn build_multipart(body: &serde_json::Value) -> (String, String) {
    let boundary = format!("boundary-{}", uuid::Uuid::new_v4().simple());
    let mut data = String::new();
    if let serde_json::Value::Object(map) = body {
        for (key, value) in map {
            let val = match value {
                serde_json::Value::String(s) => s.clone(),
                other => other.to_string(),
            };
            data.push_str(&format!(
                "--{boundary}\r\nContent-Disposition: form-data; name=\"{key}\"\r\n\r\n{val}\r\n"
            ));
        }
    }
    data.push_str(&format!("--{boundary}--\r\n"));
    (data, boundary)
}

async fn send_multipart(
    app: &axum::Router,
    path: &str,
    body: &serde_json::Value,
    headers: Option<HeaderMap>,
) -> (StatusCode, HeaderMap, serde_json::Value) {
    let (mp_body, boundary) = build_multipart(body);
    let ct = format!("multipart/form-data; boundary={boundary}");
    let mut builder = Request::builder()
        .method(Method::POST)
        .uri(path)
        .header("content-type", ct);
    if let Some(ref hdrs) = headers {
        for (name, val) in hdrs.iter() {
            builder = builder.header(name, val);
        }
    }
    let request = builder.body(Body::from(mp_body)).unwrap();
    let resp = tower::ServiceExt::oneshot(app.clone(), request)
        .await
        .unwrap();
    let status = resp.status();
    let resp_headers = resp.headers().clone();
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null);
    (status, resp_headers, json)
}

async fn send_json(
    app: &axum::Router,
    method: Method,
    path: &str,
    body: serde_json::Value,
    headers: Option<HeaderMap>,
) -> (StatusCode, HeaderMap, serde_json::Value) {
    let mut builder = Request::builder()
        .method(method)
        .uri(path)
        .header("content-type", "application/json");
    if let Some(ref hdrs) = headers {
        for (name, val) in hdrs.iter() {
            builder = builder.header(name, val);
        }
    }
    let request = builder.body(Body::from(body.to_string())).unwrap();
    let resp = tower::ServiceExt::oneshot(app.clone(), request)
        .await
        .unwrap();
    let status = resp.status();
    let resp_headers = resp.headers().clone();
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null);
    (status, resp_headers, json)
}

#[tokio::test]
async fn signup_emails_all_locales() {
    let Some((_pool, state)) = setup().await else {
        eprintln!("skipping: DATABASE_URL not set");
        return;
    };

    let app = tavern_account::router(state);

    // One account per supported email locale, using country codes that
    // resolve to that locale's region.
    let cases: &[(&str, &str, &str)] = &[
        ("enUS", "USA", "m15-enus-{}@example.com"),
        ("enGB", "GBR", "m15-engb-{}@example.com"),
        ("deDE", "DEU", "m15-dede-{}@example.com"),
        ("koKR", "KOR", "m15-kokr-{}@example.com"),
    ];

    for (locale, country, email_template) in cases {
        let email = email_template.replace("{}", &uuid::Uuid::new_v4().simple().to_string()[..8]);
        let account_id = create_account(&app, &email, country).await;
        assert!(account_id > 0, "account {locale} created");

        // Verify locale was set correctly.
        let account = tavern_db::accounts::find_by_id(&_pool, account_id)
            .await
            .expect("find")
            .expect("exists");
        assert_eq!(
            account.locale.as_str(),
            *locale,
            "account {locale}: expected locale {locale}, got {}",
            account.locale
        );
    }

    // Give the async email tasks time to complete.
    tokio::time::sleep(std::time::Duration::from_secs(2)).await;

    // All 4 emails should be in mailcrab now.
    // Visual verification: http://localhost:1080
    eprintln!("Check mailcrab at http://localhost:1080 — should have 4+ emails");
}
