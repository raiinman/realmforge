// SPDX-License-Identifier: AGPL-3.0-only

//! Account creation flow — multi-step wizard matching the captured
//! `account.battle.net/creation/flow/creation-full` surface.
//!
//! State is held in server-side sessions (keyed by a session cookie UUID)
//! and persisted across steps. Account creation happens at step 7
//! (set-password). After the final step, the SPA navigates client-side to
//! the OAuth callback URL with a login ticket.

use askama::Template;

/// The account creation page template.
#[derive(Template)]
#[template(path = "creation.html")]
pub struct CreationPage {
    pub csrf_token: String,
    pub session_id: String,
}

use std::sync::Arc;

use axum::Json;
use axum::extract::{Multipart, Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::AppState;

/// Parse a multipart/form-data request into a typed struct.
///
/// Reads all text fields into a map, converts to JSON, then deserializes.
/// Returns a 400 response on parse errors (matching the JSON error shape).
async fn parse_multipart<T: DeserializeOwned>(multipart: &mut Multipart) -> Result<T, Response> {
    use axum::http::StatusCode;
    let mut map = serde_json::Map::new();
    while let Ok(Some(field)) = multipart.next_field().await {
        let name = field.name().unwrap_or("").to_string();
        let text = field.text().await.unwrap_or_default();
        map.insert(name, serde_json::Value::String(text));
    }
    serde_json::from_value(serde_json::Value::Object(map)).map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": format!("invalid request: {e}")})),
        )
            .into_response()
    })
}

/// Flow state stored server-side between creation steps.
#[derive(Debug, Clone)]
pub struct CreationState {
    /// The current step name.
    pub step: String,
    /// Country code from get-started.
    pub country: Option<String>,
    /// Date of birth components.
    pub dob_year: Option<String>,
    pub dob_month: Option<String>,
    pub dob_day: Option<String>,
    /// Account holder's name.
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    /// Contact details.
    pub email: Option<String>,
    pub phone_number: Option<String>,
    /// Legal agreements.
    pub tos_accepted: bool,
    pub opt_in_news: bool,
    /// Password (set-password step).
    pub password: Option<String>,
    /// Battletag (set-battletag step).
    pub battletag: Option<String>,
    /// CSRF token for this flow.
    pub csrf_token: Option<String>,
    /// OAuth callback URL to redirect to after completion.
    pub callback_url: Option<String>,
    /// Expiry for the reaper.
    pub expires_at: std::time::Instant,
}

impl CreationState {
    fn new() -> Self {
        Self {
            step: "get-started".to_string(),
            country: None,
            dob_year: None,
            dob_month: None,
            dob_day: None,
            first_name: None,
            last_name: None,
            email: None,
            phone_number: None,
            tos_accepted: false,
            opt_in_news: false,
            password: None,
            battletag: None,
            csrf_token: Some(Uuid::new_v4().to_string()),
            callback_url: None,
            expires_at: std::time::Instant::now() + std::time::Duration::from_secs(1800),
        }
    }
}
// --- Step request/response types ---

#[derive(Deserialize)]
pub struct GetStartedRequest {
    #[serde(rename = "_csrf")]
    pub csrf: String,
    pub country: String,
    #[serde(rename = "dob-year")]
    pub dob_year: String,
    #[serde(rename = "dob-month")]
    pub dob_month: String,
    #[serde(rename = "dob-day")]
    pub dob_day: String,
}

#[derive(Deserialize)]
pub struct ProvideNameRequest {
    #[serde(rename = "_csrf")]
    pub csrf: String,
    #[serde(rename = "firstName")]
    pub first_name: String,
    #[serde(rename = "lastName")]
    pub last_name: String,
}

#[derive(Deserialize)]
pub struct ProvideCredentialsRequest {
    #[serde(rename = "_csrf")]
    pub csrf: String,
    pub email: String,
    #[serde(default, rename = "phone-number")]
    pub phone_number: Option<String>,
}

#[derive(Deserialize)]
pub struct SmsCaptchaGateRequest {
    #[serde(rename = "_csrf")]
    pub csrf: String,
}

#[derive(Deserialize)]
pub struct PhoneVerificationRequest {
    #[serde(rename = "_csrf")]
    pub csrf: String,
    #[serde(rename = "sms-verification-code")]
    pub code: String,
}

#[derive(Deserialize)]
pub struct LegalOptInsRequest {
    #[serde(rename = "_csrf")]
    pub csrf: String,
    #[serde(default)]
    #[serde(rename = "tou-agreements-implicit")]
    pub tos: String,
    #[serde(default)]
    #[serde(rename = "opt-in-blizzard-news-special-offers")]
    pub opt_in_news: Option<String>,
}

#[derive(Deserialize)]
pub struct SetPasswordRequest {
    #[serde(rename = "_csrf")]
    pub csrf: String,
    pub password: String,
}

#[derive(Deserialize)]
pub struct SetBattletagRequest {
    #[serde(rename = "_csrf")]
    pub csrf: String,
    pub battletag: String,
}

#[derive(Serialize)]
struct StepResponse {
    success: bool,
    #[serde(rename = "nextStep")]
    next_step: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "csrfToken")]
    csrf_token: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

impl StepResponse {
    fn ok(next_step: &str, csrf_token: &str) -> Self {
        Self {
            success: true,
            next_step: next_step.to_string(),
            csrf_token: Some(csrf_token.to_string()),
            error: None,
        }
    }

    fn error(msg: &str) -> Self {
        Self {
            success: false,
            next_step: String::new(),
            csrf_token: None,
            error: Some(msg.to_string()),
        }
    }
}

/// Compute a person's age in years from a DOB given as separate string
/// fields (the creation form submits year/month/day as text). Returns
/// `None` if any field is missing or not a valid number.
fn compute_age(year: &str, month: &str, day: &str) -> Option<i32> {
    use chrono::Datelike;
    let year: i32 = year.parse().ok()?;
    let month: u32 = month.parse().ok()?;
    let day: u32 = day.parse().ok()?;
    let today = chrono::Utc::now().date_naive();
    let mut age = today.year() - year;
    let had_birthday = (today.month(), today.day()) >= (month, day);
    if !had_birthday {
        age -= 1;
    }
    Some(age)
}

// --- Session management ---

const CREATION_SESSION_COOKIE: &str = "creation-session";

/// Get or create a creation state from the session cookie.
fn get_or_create_state(state: &Arc<AppState>, session_id: &str) -> CreationState {
    let mut map = state
        .creation_sessions
        .lock()
        .expect("creation_sessions mutex poisoned");
    map.entry(session_id.to_string())
        .or_insert_with(CreationState::new)
        .clone()
}

/// Get creation state, returning an error response if missing.
#[allow(clippy::result_large_err)]
fn get_state(state: &Arc<AppState>, session_id: &str) -> Result<CreationState, Response> {
    let map = state
        .creation_sessions
        .lock()
        .expect("creation_sessions mutex poisoned");
    map.get(session_id)
        .cloned()
        .ok_or_else(|| (StatusCode::BAD_REQUEST, "creation session not found").into_response())
}

/// Save creation state.
fn save_state(state: &Arc<AppState>, session_id: &str, creation: CreationState) {
    let mut map = state
        .creation_sessions
        .lock()
        .expect("creation_sessions mutex poisoned");
    map.insert(session_id.to_string(), creation);
}

/// Remove creation state (cleanup after completion).
#[allow(dead_code)]
fn remove_state(state: &Arc<AppState>, session_id: &str) {
    let mut map = state
        .creation_sessions
        .lock()
        .expect("creation_sessions mutex poisoned");
    map.remove(session_id);
}

/// Validate the CSRF token in a step request against the stored token.
#[allow(clippy::result_large_err)]
fn validate_csrf(creation: &CreationState, csrf: &str) -> Result<(), Response> {
    if creation.csrf_token.as_deref() != Some(csrf) {
        return Err((
            StatusCode::FORBIDDEN,
            Json(StepResponse::error("CSRF token mismatch")),
        )
            .into_response());
    }
    Ok(())
}

// --- Entry point ---

/// `GET /creation/flow/creation-full` — the creation SPA/wizard page.
/// Creates a new session on first visit; subsequent visits return the
/// existing session state.
pub async fn creation_page(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
) -> impl IntoResponse {
    let session_id = extract_session(&headers).unwrap_or_else(|| Uuid::new_v4().to_string());

    // Only create a new state if one doesn't exist for this session.
    let _csrf = {
        let mut map = state
            .creation_sessions
            .lock()
            .expect("creation_sessions mutex poisoned");
        let entry = map
            .entry(session_id.clone())
            .or_insert_with(CreationState::new);
        entry.csrf_token.clone().unwrap_or_default()
    };

    let page = CreationPage {
        csrf_token: _csrf,
        session_id: session_id.clone(),
    };
    let mut resp = axum::response::Html(page.render().unwrap_or_default()).into_response();
    let headers = resp.headers_mut();
    headers.insert(
        axum::http::header::SET_COOKIE,
        format!("creation-session={session_id}; Path=/creation; HttpOnly")
            .parse()
            .unwrap(),
    );
    resp
}

/// `GET /creation/api/init` — returns the CSRF token and session ID as JSON.
/// Used by the creation flow SPA and integration tests.
pub async fn creation_init(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
) -> impl IntoResponse {
    let session_id = extract_session(&headers).unwrap_or_else(|| Uuid::new_v4().to_string());
    let csrf = {
        let mut map = state
            .creation_sessions
            .lock()
            .expect("creation_sessions mutex poisoned");
        let entry = map
            .entry(session_id.clone())
            .or_insert_with(CreationState::new);
        entry.csrf_token.clone().unwrap_or_default()
    };
    let mut resp = Json(serde_json::json!({
        "sessionId": session_id,
        "csrfToken": csrf,
        "step": "get-started",
    }))
    .into_response();
    resp.headers_mut().insert(
        axum::http::header::SET_COOKIE,
        format!("creation-session={session_id}; Path=/creation; HttpOnly")
            .parse()
            .unwrap(),
    );
    resp
}

/// `GET /creation/flow/creation-full/back` — navigate back one step.
/// Returns the previous step and state.
pub async fn creation_back(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
) -> Response {
    let session_id = extract_session(&headers).unwrap_or_else(|| Uuid::new_v4().to_string());
    let mut creation = get_state(&state, &session_id).unwrap_or_else(|_| CreationState::new());

    // Go back one step.
    let prev = match creation.step.as_str() {
        "provide-name" => "get-started",
        "provide-credentials" => "provide-name",
        "sms-captcha-gate" => "provide-credentials",
        "phone-number-verification" => "sms-captcha-gate",
        "legal-and-opt-ins" => "phone-number-verification",
        "set-password" => "legal-and-opt-ins",
        "set-battletag" => "set-password",
        _ => "get-started",
    };
    creation.step = prev.to_string();
    creation.csrf_token = Some(Uuid::new_v4().to_string());
    save_state(&state, &session_id, creation.clone());

    Json(serde_json::json!({
        "previousStep": prev,
        "csrfToken": creation.csrf_token,
    }))
    .into_response()
}

// --- Step handlers ---

/// `POST /creation/flow/creation-full/step/get-started`
pub async fn step_get_started(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    mut multipart: Multipart,
) -> Response {
    let req: GetStartedRequest = match parse_multipart(&mut multipart).await {
        Ok(r) => r,
        Err(resp) => return resp,
    };
    let session_id = extract_session(&headers).unwrap_or_else(|| Uuid::new_v4().to_string());
    let mut creation = get_or_create_state(&state, &session_id);

    if let Err(e) = validate_csrf(&creation, &req.csrf) {
        return e;
    }

    // Age-of-majority check: the selected country's adulthood age (from
    // country_adulthood, exposed via /api/location/country-age-of-adulthood-map)
    // must be met by the DOB on the form. Minors may not create accounts.
    let country = req.country.clone();
    let adulthood = sqlx::query!(
        "SELECT ca.adulthood_age, c.alpha3
        FROM country_adulthood ca
        JOIN countries c ON c.id = ca.country_id
        WHERE c.alpha2 = $1",
        country,
    )
    .fetch_optional(&state.pool)
    .await;
    let adulthood = match adulthood {
        Ok(adulthood) => adulthood,
        Err(e) => {
            tracing::error!(error = %e, "adulthood lookup failed");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(StepResponse::error("An error occurred. Please try again.")),
            )
                .into_response();
        }
    };

    let adulthood_age = adulthood.as_ref().map(|r| r.adulthood_age).unwrap_or(18);
    let country_alpha3 = adulthood.map(|r| r.alpha3.trim().to_string());
    let age = compute_age(&req.dob_year, &req.dob_month, &req.dob_day);
    if age.is_some_and(|age| age < adulthood_age) {
        return Json(StepResponse::error(&format!(
            "You must be at least {adulthood_age} years old to create a Tavern account."
        )))
        .into_response();
    }

    // Store the alpha-3 code (accounts.country_code uses alpha-3).
    creation.country = Some(country_alpha3.unwrap_or(req.country));
    creation.dob_year = Some(req.dob_year);
    creation.dob_month = Some(req.dob_month);
    creation.dob_day = Some(req.dob_day);
    creation.step = "provide-name".to_string();
    creation.csrf_token = Some(Uuid::new_v4().to_string());

    save_state(&state, &session_id, creation.clone());

    Json(StepResponse::ok(
        "provide-name",
        creation.csrf_token.as_ref().unwrap(),
    ))
    .into_response()
}

/// `POST /creation/flow/creation-full/step/provide-name`
pub async fn step_provide_name(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    mut multipart: Multipart,
) -> Response {
    let req: ProvideNameRequest = match parse_multipart(&mut multipart).await {
        Ok(r) => r,
        Err(resp) => return resp,
    };
    let session_id = extract_session(&headers).unwrap_or_else(|| Uuid::new_v4().to_string());
    let mut creation = get_state(&state, &session_id).unwrap_or_else(|_| CreationState::new());

    if let Err(e) = validate_csrf(&creation, &req.csrf) {
        return e;
    }

    creation.first_name = Some(req.first_name);
    creation.last_name = Some(req.last_name);
    creation.step = "provide-credentials".to_string();
    creation.csrf_token = Some(Uuid::new_v4().to_string());

    save_state(&state, &session_id, creation.clone());

    Json(StepResponse::ok(
        "provide-credentials",
        creation.csrf_token.as_ref().unwrap(),
    ))
    .into_response()
}

/// `POST /creation/flow/creation-full/step/provide-credentials`
pub async fn step_provide_credentials(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    mut multipart: Multipart,
) -> Response {
    let req: ProvideCredentialsRequest = match parse_multipart(&mut multipart).await {
        Ok(r) => r,
        Err(resp) => return resp,
    };
    let session_id = extract_session(&headers).unwrap_or_else(|| Uuid::new_v4().to_string());
    let mut creation = get_state(&state, &session_id).unwrap_or_else(|_| CreationState::new());

    if let Err(e) = validate_csrf(&creation, &req.csrf) {
        return e;
    }

    // Check for duplicate email.
    match tavern_db::accounts::find_by_email(&state.pool, &req.email).await {
        Ok(Some(_)) => {
            return Json(StepResponse::error(
                "An account with that email already exists",
            ))
            .into_response();
        }
        Ok(None) => {}
        Err(e) => {
            tracing::error!(error = %e, "db error checking email");
            return (StatusCode::INTERNAL_SERVER_ERROR, "internal error").into_response();
        }
    }

    creation.email = Some(req.email);
    creation.phone_number = req.phone_number;
    creation.step = "sms-captcha-gate".to_string();
    creation.csrf_token = Some(Uuid::new_v4().to_string());

    save_state(&state, &session_id, creation.clone());

    Json(StepResponse::ok(
        "sms-captcha-gate",
        creation.csrf_token.as_ref().unwrap(),
    ))
    .into_response()
}

/// `POST /creation/flow/creation-full/step/sms-captcha-gate`
/// Stubbed: always accepted.
pub async fn step_sms_captcha_gate(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    mut multipart: Multipart,
) -> Response {
    let req: SmsCaptchaGateRequest = match parse_multipart(&mut multipart).await {
        Ok(r) => r,
        Err(resp) => return resp,
    };
    let session_id = extract_session(&headers).unwrap_or_else(|| Uuid::new_v4().to_string());
    let mut creation = get_state(&state, &session_id).unwrap_or_else(|_| CreationState::new());

    if let Err(e) = validate_csrf(&creation, &req.csrf) {
        return e;
    }

    creation.step = "phone-number-verification".to_string();
    creation.csrf_token = Some(Uuid::new_v4().to_string());

    save_state(&state, &session_id, creation.clone());

    Json(StepResponse::ok(
        "phone-number-verification",
        creation.csrf_token.as_ref().unwrap(),
    ))
    .into_response()
}

/// `POST /creation/flow/creation-full/step/phone-number-verification`
/// Stubbed: any 6-digit code is accepted.
pub async fn step_phone_verification(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    mut multipart: Multipart,
) -> Response {
    let req: PhoneVerificationRequest = match parse_multipart(&mut multipart).await {
        Ok(r) => r,
        Err(resp) => return resp,
    };
    let session_id = extract_session(&headers).unwrap_or_else(|| Uuid::new_v4().to_string());
    let mut creation = get_state(&state, &session_id).unwrap_or_else(|_| CreationState::new());

    if let Err(e) = validate_csrf(&creation, &req.csrf) {
        return e;
    }

    creation.step = "legal-and-opt-ins".to_string();
    creation.csrf_token = Some(Uuid::new_v4().to_string());

    save_state(&state, &session_id, creation.clone());

    Json(StepResponse::ok(
        "legal-and-opt-ins",
        creation.csrf_token.as_ref().unwrap(),
    ))
    .into_response()
}

/// `POST /creation/flow/creation-full/step/legal-and-opt-ins`
pub async fn step_legal_opt_ins(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    mut multipart: Multipart,
) -> Response {
    let req: LegalOptInsRequest = match parse_multipart(&mut multipart).await {
        Ok(r) => r,
        Err(resp) => return resp,
    };
    let session_id = extract_session(&headers).unwrap_or_else(|| Uuid::new_v4().to_string());
    let mut creation = get_state(&state, &session_id).unwrap_or_else(|_| CreationState::new());

    if let Err(e) = validate_csrf(&creation, &req.csrf) {
        return e;
    }

    creation.tos_accepted = !req.tos.is_empty();
    creation.opt_in_news = req.opt_in_news.as_deref() == Some("true");
    creation.step = "set-password".to_string();
    creation.csrf_token = Some(Uuid::new_v4().to_string());

    save_state(&state, &session_id, creation.clone());

    Json(StepResponse::ok(
        "set-password",
        creation.csrf_token.as_ref().unwrap(),
    ))
    .into_response()
}

/// `POST /creation/flow/creation-full/step/set-password`
/// Account is created on this step.
pub async fn step_set_password(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    mut multipart: Multipart,
) -> Response {
    let req: SetPasswordRequest = match parse_multipart(&mut multipart).await {
        Ok(r) => r,
        Err(resp) => return resp,
    };
    let session_id = extract_session(&headers).unwrap_or_else(|| Uuid::new_v4().to_string());
    let mut creation = get_state(&state, &session_id).unwrap_or_else(|_| CreationState::new());

    if let Err(e) = validate_csrf(&creation, &req.csrf) {
        return e;
    }

    let email = creation.email.as_ref().ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(StepResponse::error("missing email")),
        )
            .into_response()
    });

    let email = match email {
        Ok(e) => e,
        Err(e) => return e,
    };

    // Create the account.
    let account_id = match crate::registration::register(
        &state.pool,
        &state.group,
        email,
        &req.password,
    )
    .await
    {
        Ok(id) => id,
        Err(e) => {
            tracing::error!(error = %e, "account registration failed");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(StepResponse::error("Account creation failed")),
            )
                .into_response();
        }
    };

    // Update profile fields.
    if let Some(ref first_name) = creation.first_name {
        let _ = sqlx::query!(
            "UPDATE accounts SET first_name = $2, updated_at = NOW() WHERE id = $1",
            account_id,
            first_name,
        )
        .execute(&state.pool)
        .await;
    }
    if let Some(ref last_name) = creation.last_name {
        let _ = sqlx::query!(
            "UPDATE accounts SET last_name = $2, updated_at = NOW() WHERE id = $1",
            account_id,
            last_name,
        )
        .execute(&state.pool)
        .await;
    }
    if let Some(ref country) = creation.country {
        let _ = sqlx::query!(
            "UPDATE accounts SET country_code = $2, updated_at = NOW() WHERE id = $1",
            account_id,
            country,
        )
        .execute(&state.pool)
        .await;
    }
    // Persist the date of birth (collected on the get-started step). The
    // DB column is mandatory; compose YYYY-MM-DD from the components.
    // Persist the date of birth (collected on the get-started step). The
    // DB column is mandatory; compose YYYY-MM-DD from the components.
    if let (Some(y), Some(m), Some(d)) =
        (&creation.dob_year, &creation.dob_month, &creation.dob_day)
        && let (Ok(y), Ok(m), Ok(d)) = (y.parse::<u32>(), m.parse::<u32>(), d.parse::<u32>())
    {
        let birth_date = format!("{y:04}-{m:02}-{d:02}");
        let _ = sqlx::query!(
            "UPDATE accounts SET birth_date = $2, updated_at = NOW() WHERE id = $1",
            account_id,
            birth_date,
        )
        .execute(&state.pool)
        .await;
    }

    creation.password = Some(req.password);
    creation.step = "set-battletag".to_string();
    creation.csrf_token = Some(Uuid::new_v4().to_string());
    // Store account_id for post-completion ticket minting.
    // We add a temporary field via a map.

    save_state(&state, &session_id, creation.clone());

    tracing::info!(account_id, "account created via creation flow");

    // Send welcome email in the background (non-blocking).
    let smtp_host = state.smtp_host.clone();
    let smtp_port = state.smtp_port;
    let smtp_from = state.smtp_from.clone();
    let email_addr = email.clone();
    let base_url = state.base_url.clone();
    let locale =
        crate::locale::country_to_locale(creation.country.as_deref().unwrap_or("USA")).to_string();

    // Update the account's locale.
    let _ = sqlx::query!(
        "UPDATE accounts SET locale = $2, updated_at = NOW() WHERE id = $1",
        account_id,
        locale,
    )
    .execute(&state.pool)
    .await;

    let signing_key_pem = state.signing_key_pem.clone();
    tokio::spawn(async move {
        if let Err(e) = crate::email::send_welcome_email(
            &signing_key_pem,
            &smtp_host,
            smtp_port,
            &smtp_from,
            &email_addr,
            account_id,
            &base_url,
            &locale,
        )
        .await
        {
            tracing::warn!(
                account_id,
                error = %e,
                "failed to send welcome email (non-fatal)"
            );
        }
    });

    // Return the account_id so the SPA can use it for the final step.
    Json(serde_json::json!({
        "success": true,
        "nextStep": "set-battletag",
        "csrfToken": creation.csrf_token,
        "accountId": account_id,
    }))
    .into_response()
}

/// `POST /creation/flow/creation-full/step/set-battletag`
/// Final step. Updates the battletag and returns completion.
pub async fn step_set_battletag(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    mut multipart: Multipart,
) -> Response {
    let req: SetBattletagRequest = match parse_multipart(&mut multipart).await {
        Ok(r) => r,
        Err(resp) => return resp,
    };
    let session_id = extract_session(&headers).unwrap_or_else(|| Uuid::new_v4().to_string());
    let mut creation = get_state(&state, &session_id).unwrap_or_else(|_| CreationState::new());

    if let Err(e) = validate_csrf(&creation, &req.csrf) {
        return e;
    }

    creation.battletag = Some(req.battletag.clone());
    creation.step = "done".to_string();

    // Update battletag in DB by email.
    if let Some(ref email) = creation.email {
        let _ = sqlx::query!(
            "UPDATE accounts SET battletag = $2, updated_at = NOW() WHERE email = $1",
            email,
            req.battletag,
        )
        .execute(&state.pool)
        .await;
    }

    save_state(&state, &session_id, creation.clone());

    Json(serde_json::json!({
        "success": true,
        "step": "done",
        "battletag": req.battletag,
    }))
    .into_response()
}

// --- BattleTag suggestion ---

/// `GET /creation/api/battletag-suggestion` — generates a random BattleTag.
pub async fn battletag_suggestion() -> impl IntoResponse {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let battle_tag = format!("Player{}", rng.gen_range(1000u32..9999u32));
    Json(serde_json::json!({
        "battletag": battle_tag,
        "suggestions": [battle_tag],
    }))
}

// --- Helpers ---

fn extract_session(headers: &axum::http::HeaderMap) -> Option<String> {
    let cookie_header = headers.get(axum::http::header::COOKIE)?.to_str().ok()?;
    for pair in cookie_header.split(';') {
        let pair = pair.trim();
        if let Some((_, v)) = pair
            .split_once('=')
            .filter(|(k, _)| *k == CREATION_SESSION_COOKIE)
        {
            return Some(v.to_string());
        }
    }
    None
}

// --- Step router ---

/// Route POST /creation/flow/creation-full/step/{step} to the correct handler.
pub async fn step_router(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    Path(step): Path<String>,
    multipart: Multipart,
) -> Response {
    match step.as_str() {
        "get-started" => step_get_started(State(state), headers, multipart).await,
        "provide-name" => step_provide_name(State(state), headers, multipart).await,
        "provide-credentials" => step_provide_credentials(State(state), headers, multipart).await,
        "sms-captcha-gate" => step_sms_captcha_gate(State(state), headers, multipart).await,
        "phone-number-verification" => {
            step_phone_verification(State(state), headers, multipart).await
        }
        "legal-and-opt-ins" => step_legal_opt_ins(State(state), headers, multipart).await,
        "set-password" => step_set_password(State(state), headers, multipart).await,
        "set-battletag" => step_set_battletag(State(state), headers, multipart).await,
        _ => (StatusCode::NOT_FOUND, "unknown step").into_response(),
    }
}
