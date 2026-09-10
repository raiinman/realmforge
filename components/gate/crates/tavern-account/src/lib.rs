// SPDX-License-Identifier: AGPL-3.0-only

//! Account management service.
//!
//! Exposes an `axum` router implementing the `account.battle.net` login surface.
//! Registration stores dual credentials (BnetSRP6v2 + plaintext sha_pass_hash)
//! so any client line can authenticate. The shared ticket-mint backend bridges
//! login into the OAuth authorize flow.
//!
//! Milestone 7 adds registration, ticket minting, and browser SRP login.
//! Milestone 8 adds the game-client bnet transport (`/bnetserver/login/`).
//! Milestone 10 adds the management API (`/api/*`) and SSR UI.

pub mod api;
pub mod bnet;
pub mod bnet_types;
pub mod callback;
pub mod client_login;
pub mod creation;
pub mod email;
pub mod locale;
pub mod login;
pub mod registration;
pub mod security;
pub mod session;
pub mod ticket;
pub mod ui;

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::Instant;

use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use tavern_core::srp::{Group, ServerSession};

/// Shared application state injected into every handler.
pub struct AppState {
    /// The Postgres connection pool.
    pub pool: sqlx::PgPool,
    /// The SRP6a group (modulus, generator).
    pub group: Group,
    /// The modulus as uppercase hex, precomputed for challenge responses.
    pub modulus_hex: String,
    /// Pending SRP challenges, keyed by nonce.
    pub challenges: dashmap::DashMap<String, PendingChallenge>,
    pub bnet_sessions: dashmap::DashMap<String, crate::bnet::BnetSrpSession>,
    /// Pending authenticator logins: session_id → account_id.
    pub pending_authenticators: dashmap::DashMap<String, i64>,
    /// Pending ToS acceptances (LEGAL state): session_id → account_id.
    pub pending_legal: dashmap::DashMap<String, i64>,
    /// Current required Terms-of-Service version (env `TOS_VERSION`).
    /// Accounts whose `tos_version` differs must accept before login.
    pub tos_version: String,
    /// Creation flow sessions, keyed by session UUID.
    pub creation_sessions: Mutex<HashMap<String, crate::creation::CreationState>>,
    /// SMTP configuration for outgoing mail.
    pub smtp_host: String,
    pub smtp_port: u16,
    pub smtp_from: String,
    /// PKCS#8 PEM signing key for encrypted ticket generation.
    pub signing_key_pem: String,
    /// 2-letter region code (e.g. `US`, `KR`). Used in service tickets.
    pub region: String,
    /// Cookie domain for SSO cookies (empty for localhost, "wowemu.dev" for deployment).
    pub cookie_domain: String,
    /// Public base URL for links this server emits (welcome email, authenticator
    /// next_url). Defaults to the local dev URL; override with ACCOUNT_BASE_URL
    /// in deployments.
    pub base_url: String,
}

/// A pending SRP challenge awaiting the client's proof.
pub struct PendingChallenge {
    pub session: ServerSession,
    pub account_id: i64,
    pub expires_at: Instant,
}

/// Build the account-service router.
pub fn router(state: std::sync::Arc<AppState>) -> Router {
    Router::new()
        // Landing page.
        .route("/", get(ui::landing))
        // Browser web login + UI (M7, M10, M12).
        .route(
            "/login/{locale}/",
            get(ui::login_page).post(login::submit_email),
        )
        .route("/login/srp", post(login::srp_challenge))
        .route(
            "/login/{locale}/password",
            get(ui::password_page).post(login::srp_proof),
        )
        // 1.14.0 game-client external challenge entry (web SRP login).
        .route("/login/", get(login::external_login))
        // Game-client bnet login (M8).
        .route(
            "/bnetserver/login/",
            get(bnet::get_form).post(bnet::post_login),
        )
        // In-client login for WoW Classic 1.13.2 (no SRP).
        .route(
            "/client/login/external",
            post(client_login::post_client_login),
        )
        .route(
            "/client/login/authenticator",
            post(client_login::post_client_authenticator),
        )
        .route(
            "/client/login/tos/accept",
            post(client_login::post_client_accept_tos),
        )
        .route(
            "/legal/agreement/{version}",
            get(client_login::get_agreement),
        )
        .route("/bnetserver/login/srp/", post(bnet::post_srp_challenge))
        .route("/bnetserver/gameAccounts/", post(bnet::post_game_accounts))
        .route(
            "/bnetserver/refreshLoginTicket/",
            post(bnet::post_refresh_login_ticket),
        )
        // Account management API (M10).
        .route("/api/user", get(api::get_user))
        .route("/api/details", get(api::get_details))
        .route(
            "/api/details/address",
            get(api::get_address).put(api::put_address),
        )
        .route(
            "/api/details/battletag/rules",
            get(api::get_details_battletag_rules),
        )
        .route("/api/security", get(api::get_security))
        .route("/api/security/password", post(security::change_password))
        .route("/api/privacy", get(api::get_privacy))
        .route("/api/privacy/profile", get(api::get_privacy_profile))
        .route(
            "/api/privacy/data",
            get(api::get_privacy_data).put(api::put_privacy_data),
        )
        .route(
            "/api/privacy/social",
            get(api::get_privacy_social).put(api::put_privacy_social),
        )
        .route(
            "/api/communication-preferences",
            get(api::get_communication_preferences),
        )
        .route(
            "/api/communication-preferences/supported-locales",
            get(api::get_communication_supported_locales),
        )
        .route("/api/email/confirm", post(api::post_email_confirm))
        .route(
            "/api/email/verification",
            post(api::post_resend_verification),
        )
        .route("/api/rum", get(api::get_rum))
        .route("/api/passkeys", get(api::get_passkeys))
        .route("/api/approvals", get(api::get_approvals))
        .route(
            "/api/account-connections",
            get(api::get_account_connections),
        )
        .route("/api/overview", get(api::get_overview))
        .route("/api/games-and-subs", get(api::get_games_and_subs))
        .route("/api/classic-games", get(api::get_classic_games))
        .route(
            "/api/game-account/creation/rules",
            get(api::get_game_account_creation_rules),
        )
        .route("/api/time-gated-games", get(api::get_time_gated_games))
        .route("/api/env", get(api::get_env))
        .route("/api/location/url", get(api::get_location_url))
        .route(
            "/api/location/country-list",
            get(api::get_location_country_list),
        )
        .route(
            "/api/location/country-address-metadata",
            get(api::get_location_country_address_metadata),
        )
        .route(
            "/api/location/country-age-of-adulthood-map",
            get(api::get_country_age_of_adulthood_map),
        )
        .route("/api/age-verification", get(api::get_age_verification))
        .route("/api/parental-controls", get(api::get_parental_controls))
        // Commerce stubs.
        .route("/api/transactions", get(api::get_transactions))
        .route("/api/wallet", get(api::get_wallet))
        .route("/api/external-subs", get(api::get_external_subs))
        .route("/api/vc/lastUsed", get(api::get_vc_last_used))
        .route("/api/vc/ecosystem", get(api::get_vc_ecosystem))
        // Session management (M13).
        .route("/api/", post(api::bootstrap))
        .route("/api/logout", post(api::logout))
        // Creation flow (M15).
        .route("/creation/flow/creation-full", get(creation::creation_page))
        // API endpoint for SPA/test CSRF token retrieval.
        .route("/creation/api/init", get(creation::creation_init))
        .route(
            "/creation/flow/creation-full/back",
            get(creation::creation_back),
        )
        .route(
            "/creation/flow/creation-full/step/{step}",
            post(creation::step_router),
        )
        .route(
            "/creation/api/battletag-suggestion",
            get(creation::battletag_suggestion),
        )
        // Email verification via the /overview?ticket= link (email.rs).
        // GeoIP and SSO (M16).
        .route("/geoip", get(geoip))
        .route("/login/sso", get(sso_redirect))
        .route("/device", get(ui::device_page))
        // Static assets (M20).
        .route("/static/style.css", get(static_style_css))
        .route("/static/spa.js", get(static_spa_js))
        .route(
            "/static/tavern-logo-light.png",
            get(static_tavern_logo_light),
        )
        .route("/static/tavern-logo-dark.png", get(static_tavern_logo_dark))
        .route("/favicon.svg", get(static_favicon_svg))
        // SSR UI (M10).
        .route("/overview", get(ui::dashboard))
        // Web logout (SSR) — clears session, redirects to landing.
        .route("/logout", get(ui::logout))
        // OAuth code callback (M13).
        .route(
            "/callback/oauth2/code/account-settings",
            get(callback::oauth_callback),
        )
        .with_state(state)
}

/// Construct shared application state from a pool.
pub fn app_state(pool: sqlx::PgPool, signing_key_pem: String) -> std::sync::Arc<AppState> {
    let group = Group::battle_net_v2();
    let modulus_hex = hex::encode_upper(group.n.to_bytes_be().1);
    std::sync::Arc::new(AppState {
        pool,
        group,
        modulus_hex,
        challenges: dashmap::DashMap::new(),
        bnet_sessions: dashmap::DashMap::new(),
        pending_authenticators: dashmap::DashMap::new(),
        pending_legal: dashmap::DashMap::new(),
        tos_version: std::env::var("TOS_VERSION").unwrap_or_else(|_| "2026-08-05".to_string()),
        creation_sessions: Mutex::new(HashMap::new()),
        smtp_host: "localhost".to_string(),
        smtp_port: 1025,
        smtp_from: "noreply@wowemu.dev".to_string(),
        signing_key_pem,
        region: "US".to_string(),
        cookie_domain: String::new(),
        base_url: std::env::var("ACCOUNT_BASE_URL")
            .unwrap_or_else(|_| "http://localhost:8080".to_string()),
    })
}

/// Construct shared application state with an explicit region code.
pub fn app_state_with_region(
    pool: sqlx::PgPool,
    signing_key_pem: String,
    region: &str,
    smtp_host: &str,
    smtp_port: u16,
    smtp_from: &str,
) -> std::sync::Arc<AppState> {
    let group = Group::battle_net_v2();
    let modulus_hex = hex::encode_upper(group.n.to_bytes_be().1);
    std::sync::Arc::new(AppState {
        pool,
        group,
        modulus_hex,
        challenges: dashmap::DashMap::new(),
        bnet_sessions: dashmap::DashMap::new(),
        pending_authenticators: dashmap::DashMap::new(),
        pending_legal: dashmap::DashMap::new(),
        tos_version: std::env::var("TOS_VERSION").unwrap_or_else(|_| "2026-08-05".to_string()),
        creation_sessions: Mutex::new(HashMap::new()),
        smtp_host: smtp_host.to_string(),
        smtp_port,
        smtp_from: smtp_from.to_string(),
        signing_key_pem,
        region: region.to_string(),
        cookie_domain: String::new(),
        base_url: std::env::var("ACCOUNT_BASE_URL")
            .unwrap_or_else(|_| "http://localhost:8080".to_string()),
    })
}

/// Errors returned by the account service.
#[derive(Debug, thiserror::Error)]
pub enum AccountError {
    #[error("account not found")]
    NotFound,

    #[error("invalid credentials")]
    InvalidCredentials,

    #[error("challenge expired or not found")]
    ChallengeExpired,

    #[error("invalid request: {0}")]
    BadRequest(String),

    #[error("invalid hex in SRP proof")]
    InvalidHex,

    #[error("database error: {0}")]
    Database(#[from] tavern_db::DbError),

    #[error("internal error: {0}")]
    Internal(String),
}

impl From<sqlx::Error> for AccountError {
    fn from(e: sqlx::Error) -> Self {
        Self::Database(e.into())
    }
}

impl IntoResponse for AccountError {
    fn into_response(self) -> Response {
        let status = match &self {
            Self::NotFound => StatusCode::NOT_FOUND,
            Self::InvalidCredentials | Self::ChallengeExpired => StatusCode::UNAUTHORIZED,
            Self::BadRequest(_) | Self::InvalidHex => StatusCode::BAD_REQUEST,
            Self::Database(_) | Self::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        };
        (
            status,
            Json(serde_json::json!({ "error": self.to_string() })),
        )
            .into_response()
    }
}

/// Spawn a background task that removes expired entries from the in-memory
/// SRP session maps every 60 seconds.
pub fn spawn_reaper(state: std::sync::Arc<AppState>) {
    tokio::spawn(async move {
        let mut tick = tokio::time::interval(std::time::Duration::from_secs(60));
        loop {
            tick.tick().await;
            let now = std::time::Instant::now();

            state.challenges.retain(|_, c| c.expires_at > now);

            state.bnet_sessions.retain(|_, s| s.expires_at > now);
            if let Ok(mut m) = state.creation_sessions.lock() {
                m.retain(|_, c| c.expires_at > now);
            }
        }
    });
}

/// `GET /geoip` — region detection called by Phoenix desktop app and
/// the bootstrapper. Returns the configured datacenter region and a
/// default country code.
#[allow(dead_code)]
async fn geoip(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let region = state.region.clone();
    let country = match region.as_str() {
        "US" => "US",
        "EU" => "GB",
        "KR" => "KR",
        "CN" => "CN",
        _ => "US",
    };
    Json(serde_json::json!({
        "datacenter_region": region,
        "country_alpha2": country,
    }))
}

/// `GET /login/sso` — cross-site SSO redirector with encrypted ticket.
#[derive(serde::Deserialize)]
#[allow(dead_code)]
struct SsoQuery {
    token: Option<String>,
    #[serde(rename = "ref")]
    r#ref: Option<String>,
}

#[allow(dead_code)]
async fn sso_redirect(
    State(state): State<std::sync::Arc<AppState>>,
    Query(params): Query<SsoQuery>,
) -> Result<impl IntoResponse, AccountError> {
    use axum::http::header;
    use axum::response::Redirect;

    let token = params
        .token
        .as_deref()
        .ok_or_else(|| AccountError::BadRequest("missing SSO token".into()))?;

    let ref_url = params
        .r#ref
        .as_deref()
        .ok_or_else(|| AccountError::BadRequest("missing ref URL".into()))?;

    // Decrypt the ticket to get the account_id.
    let account_id = tavern_core::encrypted_ticket::open(token, &state.signing_key_pem)
        .map_err(|e| AccountError::BadRequest(format!("invalid SSO token: {e}")))?;

    let _account = tavern_db::accounts::find_by_id(&state.pool, account_id)
        .await?
        .ok_or(AccountError::NotFound)?;

    // Create a management session.
    let session_id = uuid::Uuid::new_v4();
    let xsrf_token = uuid::Uuid::new_v4().to_string();
    let session = tavern_core::Session {
        session_id,
        account_id,
        expires_at: chrono::Utc::now() + chrono::Duration::minutes(30),
        created_at: chrono::Utc::now(),
        user_agent: None,
        ip: None,
    };
    tavern_db::sessions::insert(&state.pool, &session).await?;

    let mut resp = Redirect::to(ref_url).into_response();
    let headers = resp.headers_mut();
    crate::login::set_cookie(
        headers,
        "SESSIONID",
        &crate::session::encode_session_id(session_id),
        "/",
        1800,
        &state.cookie_domain,
    );
    let insecure = std::env::var("INSECURE_COOKIES").is_ok_and(|v| v != "0");
    let xsrf_attrs = if insecure {
        "Path=/; HttpOnly; SameSite=Lax"
    } else {
        "Path=/; Secure; HttpOnly; SameSite=None"
    };
    headers.append(
        header::SET_COOKIE,
        format!("XSRF-TOKEN={xsrf_token}; {xsrf_attrs}")
            .parse()
            .unwrap(),
    );

    tracing::info!(account_id, "SSO redirect");
    Ok(resp)
}

// ---------------------------------------------------------------------------
// Static file handlers (embedded at compile time)
// ---------------------------------------------------------------------------

/// `GET /static/style.css` — embedded design system stylesheet.
async fn static_style_css() -> impl IntoResponse {
    (
        axum::http::StatusCode::OK,
        [(axum::http::header::CONTENT_TYPE, "text/css; charset=utf-8")],
        include_str!("../static/style.css"),
    )
}

/// `GET /static/spa.js` — embedded SPA client JavaScript.
async fn static_spa_js() -> impl IntoResponse {
    (
        axum::http::StatusCode::OK,
        [(
            axum::http::header::CONTENT_TYPE,
            "application/javascript; charset=utf-8",
        )],
        include_str!("../static/spa.js"),
    )
}

/// `GET /static/tavern-logo-light.png` — black-glyph emblem for light
/// theme surfaces.
async fn static_tavern_logo_light() -> impl IntoResponse {
    let png: &'static [u8] = include_bytes!("../static/tavern-logo-light.png");
    (
        axum::http::StatusCode::OK,
        [(axum::http::header::CONTENT_TYPE, "image/png")],
        png,
    )
}

/// `GET /static/tavern-logo-dark.png` — white-glyph emblem for dark
/// theme surfaces.
async fn static_tavern_logo_dark() -> impl IntoResponse {
    let png: &'static [u8] = include_bytes!("../static/tavern-logo-dark.png");
    (
        axum::http::StatusCode::OK,
        [(axum::http::header::CONTENT_TYPE, "image/png")],
        png,
    )
}

/// `GET /favicon.svg` — the monochrome emblem as a theme-aware favicon
/// (the source SVG self-inverts under `prefers-color-scheme: dark`).
async fn static_favicon_svg() -> impl IntoResponse {
    let svg: &'static [u8] = include_bytes!("../static/favicon.svg");
    (
        axum::http::StatusCode::OK,
        [(axum::http::header::CONTENT_TYPE, "image/svg+xml")],
        svg,
    )
}
