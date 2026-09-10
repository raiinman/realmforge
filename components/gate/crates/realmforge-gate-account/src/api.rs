// SPDX-License-Identifier: AGPL-3.0-only

//! Account management API endpoints (`/api/*`).
//!
//! The complete captured surface from `account.battle.net`. Most endpoints
//! return account profile data; commerce endpoints return stubs.

use std::sync::Arc;

use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use serde::{Deserialize, Serialize};

use realmforge_gate_db::accounts;

use crate::AccountError;

/// All API responses use these JSON shapes. Most return a flat object; the
/// data comes from the `accounts` table via the M7 registration and M10
/// password-change flows.

#[derive(Serialize)]
pub struct UserResponse {
    #[serde(rename = "accountId")]
    account_id: i64,
    #[serde(rename = "battleTag")]
    battle_tag: serde_json::Value,
    #[serde(rename = "countryId")]
    country_id: Option<i32>,
    #[serde(rename = "countryCodeAlpha3")]
    country_code: String,
    employee: bool,
}

impl From<realmforge_gate_core::Account> for UserResponse {
    fn from(a: realmforge_gate_core::Account) -> Self {
        let battle_tag = if a.battletag.is_empty() {
            serde_json::json!({"name": "", "code": "0"})
        } else {
            let (name, code) = a.battletag.split_once('#').unwrap_or((&a.battletag, "0"));
            serde_json::json!({"name": name, "code": code})
        };
        Self {
            account_id: a.id,
            battle_tag,
            country_id: a.country_id,
            country_code: a.country_code,
            employee: false,
        }
    }
}

#[derive(Serialize)]
pub struct DetailsResponse {
    metadata: serde_json::Value,
    #[serde(rename = "accountId")]
    account_id: i64,
    email: Option<String>,
    #[serde(rename = "firstName")]
    first_name: Option<String>,
    #[serde(rename = "lastName")]
    last_name: Option<String>,
    #[serde(rename = "birthDate")]
    birth_date: Option<String>,
    #[serde(rename = "battleTag")]
    battletag: Option<String>,
    #[serde(rename = "avatarUrl")]
    avatar_url: Option<String>,
    #[serde(rename = "countryId")]
    country_id: Option<i32>,
    #[serde(rename = "countryCodeAlpha3")]
    country_code_alpha3: Option<String>,
}

#[derive(Serialize)]
pub struct SecurityResponse {
    authenticator_enabled: bool,
    passkeys: Vec<String>,
    last_password_change: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OverviewResponse {
    account_id: i64,
    battletag: String,
    email: String,
    balance: f64,
    account_security_status: AccountSecurityStatus,
    game_accounts: Vec<GameAccountSummary>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AccountSecurityStatus {
    account_security_level: u32,
    email_verified: bool,
    authenticator_attached: bool,
    sms_protect_attached: bool,
    age_verification_required: bool,
}

#[derive(Serialize)]
struct GameAccountSummary {
    name: String,
    expansion: u32,
}

#[derive(Serialize)]
pub struct EmptyArrayResponse {
    items: Vec<serde_json::Value>,
}

#[derive(Serialize)]
pub struct EnvResponse {
    region: String,
    locale: String,
    env: String,
    metadata: serde_json::Value,
}

#[derive(Serialize)]
pub struct CountryListResponse {
    countries: Vec<serde_json::Value>,
}

// --- Session resolution ---
// Session is resolved from the SESSIONID cookie. Protected endpoints also
// validate the X-XSRF-TOKEN double-submit cookie. The bootstrap endpoint
// (POST /api/) skips XSRF per the capture.

/// Load an account using cookie-based session resolution. Falls back to the
/// X-Account-Id header for backward compatibility with existing tests.
async fn load_account(
    state: &Arc<crate::AppState>,
    headers: &axum::http::HeaderMap,
) -> Result<realmforge_gate_core::Account, AccountError> {
    // Try cookie-based session first.
    if let Ok(id) = crate::session::resolve_session(state, headers).await {
        return accounts::find_by_id(&state.pool, id)
            .await?
            .ok_or(AccountError::NotFound);
    }
    // Fallback: X-Account-Id header (for integration tests without cookies).
    let id = headers
        .get("x-account-id")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.parse().ok())
        .ok_or_else(|| {
            AccountError::BadRequest("missing SESSIONID cookie or X-Account-Id header".into())
        })?;
    accounts::find_by_id(&state.pool, id)
        .await?
        .ok_or(AccountError::NotFound)
}

/// Load an account with XSRF validation (for protected endpoints).
async fn load_account_protected(
    state: &Arc<crate::AppState>,
    headers: &axum::http::HeaderMap,
) -> Result<realmforge_gate_core::Account, AccountError> {
    crate::session::validate_xsrf(headers)?;
    load_account(state, headers).await
}

// --- Bootstrap endpoint ---

#[derive(Serialize)]
pub struct BootstrapResponse {
    #[serde(rename = "accountId")]
    account_id: i64,
    authenticated: bool,
    #[serde(rename = "loginUri")]
    login_uri: Option<String>,
    #[serde(rename = "logoutUri")]
    logout_uri: Option<String>,
    #[serde(rename = "accountCompletion")]
    account_completion: serde_json::Value,
    #[serde(rename = "userIp")]
    user_ip: String,
    #[serde(rename = "userProxiedIp")]
    user_proxied_ip: String,
}

/// POST /api/ — bootstrap identity check (no XSRF required per capture).
pub async fn bootstrap(
    State(state): State<Arc<crate::AppState>>,
    headers: axum::http::HeaderMap,
) -> Result<Json<BootstrapResponse>, AccountError> {
    match load_account(&state, &headers).await {
        Ok(account) => Ok(Json(BootstrapResponse {
            account_id: account.id,
            authenticated: true,
            login_uri: Some("/login/en/".to_string()),
            logout_uri: Some("/api/logout".to_string()),
            account_completion: serde_json::json!({
                "accountCompletionFields": [],
                "accountCountry": account.country_code,
                "completionUrl": "/creation/completion",
                "requiresHealup": false
            }),
            user_ip: "127.0.0.1".to_string(),
            user_proxied_ip: String::new(),
        })),
        Err(_) => Ok(Json(BootstrapResponse {
            account_id: 0,
            authenticated: false,
            login_uri: Some("/login/en/".to_string()),
            logout_uri: None,
            account_completion: serde_json::json!({
                "accountCompletionFields": [],
                "accountCountry": "US",
                "completionUrl": "/creation/completion",
                "requiresHealup": false
            }),
            user_ip: "127.0.0.1".to_string(),
            user_proxied_ip: String::new(),
        })),
    }
}

/// POST /api/logout — end the session.
pub async fn logout(
    State(state): State<Arc<crate::AppState>>,
    headers: axum::http::HeaderMap,
) -> impl axum::response::IntoResponse {
    // Extract the SESSIONID from the cookie and delete the session.
    if let Some(session_id) = crate::session::extract_session_id(&headers) {
        let _ = realmforge_gate_db::sessions::delete(&state.pool, session_id).await;
        tracing::info!(%session_id, "session ended");
    }
    // Clear the SESSIONID cookie.
    (
        axum::http::StatusCode::OK,
        [(
            axum::http::header::SET_COOKIE,
            "SESSIONID=; Path=/; Max-Age=0; Secure; HttpOnly",
        )],
    )
}

// --- Handlers ---

pub async fn get_user(
    State(state): State<Arc<crate::AppState>>,
    headers: axum::http::HeaderMap,
) -> Result<Json<UserResponse>, AccountError> {
    let account = load_account(&state, &headers).await?;
    Ok(Json(UserResponse::from(account)))
}

pub async fn get_details(
    State(state): State<Arc<crate::AppState>>,
    headers: axum::http::HeaderMap,
) -> Result<Json<DetailsResponse>, AccountError> {
    let a = load_account_protected(&state, &headers).await?;
    Ok(Json(DetailsResponse {
        metadata: serde_json::json!({"fieldMetadata": {}, "disableEditing": false}),
        account_id: a.id,
        email: Some(a.email),
        first_name: a.first_name,
        last_name: a.last_name,
        birth_date: a.birth_date,
        battletag: Some(a.battletag),
        avatar_url: None,
        country_id: a.country_id,
        country_code_alpha3: Some(a.country_code),
    }))
}

pub async fn get_security(
    State(state): State<Arc<crate::AppState>>,
    headers: axum::http::HeaderMap,
) -> Result<Json<SecurityResponse>, AccountError> {
    let _ = load_account_protected(&state, &headers).await?;
    Ok(Json(SecurityResponse {
        authenticator_enabled: false,
        passkeys: vec![],
        last_password_change: None,
    }))
}

pub async fn get_privacy(
    State(state): State<Arc<crate::AppState>>,
    headers: axum::http::HeaderMap,
) -> Result<Json<serde_json::Value>, AccountError> {
    let account = load_account_protected(&state, &headers).await?;

    let prefs = sqlx::query!(
        r#"SELECT * FROM privacy_settings WHERE account_id = $1"#,
        account.id
    )
    .fetch_optional(&state.pool)
    .await
    .map_err(|e| AccountError::Internal(format!("DB: {e}")))?;

    let resp = if let Some(p) = prefs {
        serde_json::json!({
            "sharingOptOutInProgress": false,
            "metadata": {"fieldMetadata": {}, "disableEditing": false},
            "enableTextChat": p.enable_text_chat,
            "enablePrivateTextChat": p.enable_private_text_chat,
            "onlyAllowFriendWhispers": p.only_allow_friend_whispers,
            "enableVoiceChat": p.enable_voice_chat,
            "enableVoiceChatSpeak": p.enable_voice_chat_speak,
            "enableFriendsManagement": p.enable_friends_management,
            "enableGroups": p.enable_groups,
            "enableRealId": p.enable_real_id,
            "enableFriendsOfFriends": p.enable_friends_of_friends,
            "enableParentalControl": p.enable_parental_control,
            "enableForceMute": p.enable_force_mute,
            "enableThirdPartySharing": p.enable_third_party_sharing,
            "hasSms": p.has_sms,
        })
    } else {
        serde_json::json!({})
    };
    Ok(Json(resp))
}

pub async fn get_privacy_profile(
    State(state): State<Arc<crate::AppState>>,
    headers: axum::http::HeaderMap,
) -> Result<Json<serde_json::Value>, AccountError> {
    let account = load_account_protected(&state, &headers).await?;

    let visibility = sqlx::query_scalar!(
        r#"SELECT profile_visibility FROM privacy_settings WHERE account_id = $1"#,
        account.id
    )
    .fetch_optional(&state.pool)
    .await
    .map_err(|e| AccountError::Internal(format!("DB: {e}")))?
    .unwrap_or_else(|| "FRIENDS".to_string());

    Ok(Json(serde_json::json!({
        "profileVisibility": visibility,
        "metadata": {"fieldMetadata": {}, "disableEditing": false},
    })))
}

/// GET /api/privacy/data — third-party data sharing flag.
pub async fn get_privacy_data(
    State(state): State<Arc<crate::AppState>>,
    headers: axum::http::HeaderMap,
) -> Result<Json<serde_json::Value>, AccountError> {
    let account = load_account_protected(&state, &headers).await?;
    let sharing: bool = sqlx::query_scalar!(
        r#"SELECT enable_third_party_sharing FROM privacy_settings WHERE account_id = $1"#,
        account.id
    )
    .fetch_optional(&state.pool)
    .await
    .map_err(|e| AccountError::Internal(format!("DB: {e}")))?
    .unwrap_or(false);
    Ok(Json(serde_json::json!({ "thirdPartySharing": sharing })))
}

/// PUT /api/privacy/data — update third-party data sharing.
pub async fn put_privacy_data(
    State(state): State<Arc<crate::AppState>>,
    headers: axum::http::HeaderMap,
    Json(body): Json<serde_json::Value>,
) -> Result<impl IntoResponse, AccountError> {
    let account = load_account_protected(&state, &headers).await?;
    let sharing = body["thirdPartySharing"].as_bool().unwrap_or(false);
    sqlx::query!(
        r#"INSERT INTO privacy_settings (account_id, enable_third_party_sharing)
        VALUES ($1, $2)
        ON CONFLICT (account_id) DO UPDATE SET enable_third_party_sharing = $2"#,
        account.id,
        sharing,
    )
    .execute(&state.pool)
    .await
    .map_err(|e| AccountError::Internal(format!("DB: {e}")))?;
    Ok(StatusCode::OK)
}

/// GET /api/privacy/social — chat/voice/real-id settings.
pub async fn get_privacy_social(
    State(state): State<Arc<crate::AppState>>,
    headers: axum::http::HeaderMap,
) -> Result<Json<serde_json::Value>, AccountError> {
    let account = load_account_protected(&state, &headers).await?;
    let p = sqlx::query!(
        r#"SELECT enable_text_chat, enable_private_text_chat, only_allow_friend_whispers,
        enable_voice_chat, enable_voice_chat_speak, enable_friends_management,
        enable_groups, enable_real_id, enable_friends_of_friends, show_real_id
        FROM privacy_settings WHERE account_id = $1"#,
        account.id,
    )
    .fetch_optional(&state.pool)
    .await
    .map_err(|e| AccountError::Internal(format!("DB: {e}")))?;
    let resp = if let Some(p) = p {
        serde_json::json!({
            "enableTextChat": p.enable_text_chat,
            "enableVoiceChat": p.enable_voice_chat,
            "enableVoiceChatSpeak": p.enable_voice_chat_speak,
            "enableFriendsManagement": p.enable_friends_management,
            "enableFriendsOfFriends": p.enable_friends_of_friends,
            "enableRealId": p.enable_real_id,
            "showRealId": p.show_real_id,
            "enableGroups": p.enable_groups,
            "onlyAllowFriendWhispers": p.only_allow_friend_whispers,
            "enablePrivateTextChat": p.enable_private_text_chat,
        })
    } else {
        serde_json::json!({})
    };
    Ok(Json(resp))
}

/// PUT /api/privacy/social — update chat/voice/real-id settings.
pub async fn put_privacy_social(
    State(state): State<Arc<crate::AppState>>,
    headers: axum::http::HeaderMap,
    Json(body): Json<serde_json::Value>,
) -> Result<impl IntoResponse, AccountError> {
    let account = load_account_protected(&state, &headers).await?;
    sqlx::query!(
        r#"INSERT INTO privacy_settings (account_id, enable_text_chat, enable_voice_chat,
        enable_voice_chat_speak, enable_friends_management, enable_friends_of_friends,
        enable_real_id, show_real_id, enable_groups, only_allow_friend_whispers,
        enable_private_text_chat)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
        ON CONFLICT (account_id) DO UPDATE SET
          enable_text_chat = $2, enable_voice_chat = $3, enable_voice_chat_speak = $4,
          enable_friends_management = $5, enable_friends_of_friends = $6,
          enable_real_id = $7, show_real_id = $8, enable_groups = $9,
          only_allow_friend_whispers = $10, enable_private_text_chat = $11"#,
        account.id,
        body["enableTextChat"].as_bool().unwrap_or(true),
        body["enableVoiceChat"].as_bool().unwrap_or(false),
        body["enableVoiceChatSpeak"].as_bool().unwrap_or(false),
        body["enableFriendsManagement"].as_bool().unwrap_or(true),
        body["enableFriendsOfFriends"].as_bool().unwrap_or(false),
        body["enableRealId"].as_bool().unwrap_or(true),
        body["showRealId"].as_bool().unwrap_or(true),
        body["enableGroups"].as_bool().unwrap_or(true),
        body["onlyAllowFriendWhispers"].as_bool().unwrap_or(true),
        body["enablePrivateTextChat"].as_bool().unwrap_or(true),
    )
    .execute(&state.pool)
    .await
    .map_err(|e| AccountError::Internal(format!("DB: {e}")))?;
    Ok(StatusCode::OK)
}

/// GET /api/communication-preferences/supported-locales — public.
pub async fn get_communication_supported_locales(
    State(state): State<Arc<crate::AppState>>,
) -> impl IntoResponse {
    let locales: Vec<String> =
        sqlx::query_scalar!(r#"SELECT locale FROM supported_locales ORDER BY sort_order"#,)
            .fetch_all(&state.pool)
            .await
            .unwrap_or_default();
    Json(locales)
}

/// POST /api/email/confirm — confirm the account email with a token.
pub async fn post_email_confirm(
    State(state): State<Arc<crate::AppState>>,
    headers: axum::http::HeaderMap,
    Json(body): Json<serde_json::Value>,
) -> Result<impl IntoResponse, AccountError> {
    let account = load_account_protected(&state, &headers).await?;
    let token = body["token"]
        .as_str()
        .ok_or_else(|| AccountError::BadRequest("missing token".into()))?;
    let row = sqlx::query!(
        r#"SELECT account_id, expires_at FROM email_confirm_tokens
        WHERE token = $1 AND account_id = $2"#,
        token,
        account.id,
    )
    .fetch_optional(&state.pool)
    .await
    .map_err(|e| AccountError::Internal(format!("DB: {e}")))?;
    let row = row.ok_or(AccountError::BadRequest("invalid or expired token".into()))?;
    if row.expires_at < chrono::Utc::now() {
        return Err(AccountError::BadRequest("token expired".into()));
    }
    sqlx::query!(
        r#"UPDATE accounts SET email_verified = TRUE WHERE id = $1"#,
        account.id,
    )
    .execute(&state.pool)
    .await
    .map_err(|e| AccountError::Internal(format!("DB: {e}")))?;
    sqlx::query!(
        r#"DELETE FROM email_confirm_tokens WHERE token = $1"#,
        token,
    )
    .execute(&state.pool)
    .await
    .map_err(|e| AccountError::Internal(format!("DB: {e}")))?;
    Ok(StatusCode::OK)
}

/// GET /api/rum — Real User Monitoring telemetry (public beacon).
pub async fn get_rum() -> impl IntoResponse {
    Json(serde_json::json!({ "enabled": false }))
}

pub async fn get_overview(
    State(state): State<Arc<crate::AppState>>,
    headers: axum::http::HeaderMap,
) -> Result<Json<OverviewResponse>, AccountError> {
    let account = load_account_protected(&state, &headers).await?;

    let game_accounts = sqlx::query!(
        "SELECT name FROM game_accounts WHERE account_id = $1 ORDER BY id",
        account.id,
    )
    .fetch_all(&state.pool)
    .await?;

    Ok(Json(OverviewResponse {
        account_id: account.id,
        battletag: account.battletag,
        email: account.email,
        balance: 0.0,
        account_security_status: AccountSecurityStatus {
            account_security_level: 100,
            email_verified: account.email_verified,
            authenticator_attached: false,
            sms_protect_attached: false,
            age_verification_required: false,
        },
        game_accounts: game_accounts
            .into_iter()
            .map(|r| GameAccountSummary {
                name: r.name,
                expansion: 4,
            })
            .collect(),
    }))
}

/// POST /api/email/verification — resend the verification email for an
/// unverified account. Mints a new sealed ticket and sends the welcome
/// email (which carries the /overview?ticket= link).
pub async fn post_resend_verification(
    State(state): State<Arc<crate::AppState>>,
    headers: axum::http::HeaderMap,
) -> Result<impl IntoResponse, AccountError> {
    let account = load_account_protected(&state, &headers).await?;
    if account.email_verified {
        return Ok(StatusCode::BAD_REQUEST);
    }
    let locale = crate::locale::country_to_locale(account.country_code.as_str()).to_string();
    crate::email::send_welcome_email(
        &state.signing_key_pem,
        &state.smtp_host,
        state.smtp_port,
        &state.smtp_from,
        &account.email,
        account.id,
        &state.base_url,
        &locale,
    )
    .await
    .map_err(|e| AccountError::Internal(format!("email send failed: {e}")))?;
    tracing::info!(account_id = account.id, "verification email resent");
    Ok(StatusCode::OK)
}

pub async fn get_games_and_subs(
    State(state): State<Arc<crate::AppState>>,
    headers: axum::http::HeaderMap,
) -> Result<Json<serde_json::Value>, AccountError> {
    let account = load_account_protected(&state, &headers).await?;

    // Query game accounts with status and subscription data, plus their
    // per-game-account license grants and the account-level (fallback)
    // grants.
    let ga_rows = sqlx::query!(
        r#"SELECT ga.id, ga.name, ga.region, ga.is_suspended, ga.is_banned,
        ga.game_time_expires,
        ARRAY_AGG(al.license_id) FILTER (WHERE al.game_account_id = ga.id) AS ga_license_ids,
        (SELECT COUNT(*) FROM account_licenses a2
         WHERE a2.account_id = ga.account_id AND a2.game_account_id IS NULL) > 0 AS has_account_licenses
        FROM game_accounts ga
        LEFT JOIN account_licenses al ON al.game_account_id = ga.id
        WHERE ga.account_id = $1
        GROUP BY ga.id ORDER BY ga.id"#,

        account.id
    )
    .fetch_all(&state.pool)
    .await
    .map_err(|e| AccountError::Internal(format!("DB: {e}")))?;

    let wow_program: i64 = 0x576F57; // "WoW" as FourCC
    let game_accounts: Vec<serde_json::Value> = ga_rows
        .iter()
        .map(|r| {
            let now_ms = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as i64;
            let has_game_time = r
                .game_time_expires
                .map(|t| t.timestamp_millis() > now_ms / 1000 * 1000)
                .unwrap_or(false);
            // Status mirrors the real surface: Inactive for banned/
            // suspended or license-less accounts (neither per-game-account
            // nor account-level grants), Trial with licenses but no active
            // game time, Good with active game time.
            let ga_lics = r.ga_license_ids.as_deref().unwrap_or(&[]);
            let has_licenses = !ga_lics.is_empty() || r.has_account_licenses.unwrap_or(false);
            let status = if r.is_banned || r.is_suspended || !has_licenses {
                "Inactive"
            } else if !has_game_time {
                "Trial"
            } else {
                "Good"
            };
            serde_json::json!({
                "titleId": wow_program,
                "localizedGameName": "World of Warcraft®",
                "gameAccountName": r.name,
                "gameAccountUniqueId": {
                    "gameAccountId": r.id,
                    "gameServiceRegionId": r.region as i64,
                    "programId": wow_program
                },
                "gameAccountRegion": region_label(r.region),
                "regionalGameFranchiseIconFilename": "world-of-warcraft.svg",
                "gameAccountStatus": status,
                "lastPlayedDateMillis": null,
                "titleHasSubscriptions": true,
                "titleHasGameTime": has_game_time,
                "accountSubscriptionView": if has_game_time {
                    serde_json::json!({
                        "subscriptionStatus": "ACTIVE",
                        "expires": r.game_time_expires.map(|t| t.timestamp_millis()),
                        "localizedSubscriptionProductName": "Subscription"
                    })
                } else {
                    serde_json::Value::Null
                },
                "gameTimeView": if has_game_time {
                    serde_json::json!({
                        "gameTimeType": "TIME_LIMITED",
                        "expireDate": r.game_time_expires.map(|t| t.timestamp_millis())
                    })
                } else {
                    serde_json::Value::Null
                },
                "displayOrder": 1,
                "customDownloadLink": null
            })
        })
        .collect();

    Ok(Json(serde_json::json!({
        "gameAccounts": game_accounts
    })))
}

fn region_label(region: i16) -> &'static str {
    match region {
        1 => "US",
        2 => "EU",
        3 => "KR",
        4 => "TW",
        5 => "CN",
        _ => "US",
    }
}
pub async fn get_classic_games(
    State(state): State<Arc<crate::AppState>>,
    headers: axum::http::HeaderMap,
) -> Result<Json<serde_json::Value>, AccountError> {
    let _account = load_account_protected(&state, &headers).await?;

    // Classic games are legacy CD-key activations (Diablo II, Warcraft III).
    // We don't store CD keys, so return empty. The real API format is:
    // [{"localizedGameName":..., "cdKeys":[...], "displayOrder":...}]
    Ok(Json(serde_json::json!({ "classicGames": [] })))
}
pub async fn get_env() -> impl IntoResponse {
    Json(serde_json::json!({
        "region": "US",
        "locale": "enUS",
        "env": "dev",
        "metadata": {"fieldMetadata": {}, "disableEditing": false},
    }))
}

pub async fn get_communication_preferences(
    State(state): State<Arc<crate::AppState>>,
    headers: axum::http::HeaderMap,
) -> Result<Json<serde_json::Value>, AccountError> {
    let account = load_account_protected(&state, &headers).await?;

    let prefs = sqlx::query!(
        r#"SELECT * FROM communication_preferences WHERE account_id = $1"#,
        account.id
    )
    .fetch_optional(&state.pool)
    .await
    .map_err(|e| AccountError::Internal(format!("DB: {e}")))?;

    let resp = if let Some(p) = prefs {
        serde_json::json!({
            "receiveTargetedAds": p.receive_targeted_ads,
            "receiveBlizzardOffers": p.receive_blizzard_offers,
            "enableBlizzardPersonalizedProductRecommendations": p.enable_personalized_recommendations,
            "limitedDescriptions": p.limited_descriptions,
            "worldOfWarcraftCommunications": p.world_of_warcraft_communications,
            "warcraftRumbleCommunications": p.warcraft_rumble_communications,
            "diabloCommunications": p.diablo_communications,
            "overwatchCommunications": p.overwatch_communications,
            "hearthstoneCommunications": p.hearthstone_communications,
            "callOfDutyCommunications": p.call_of_duty_communications,
            "blizzardClassicsCommunications": p.blizzard_classics_communications,
            "surveysCommunication": p.surveys_communication,
            "xboxCommunications": p.xbox_communications,
        })
    } else {
        serde_json::json!({})
    };
    Ok(Json(resp))
}

pub async fn get_location_country_list(
    State(state): State<Arc<crate::AppState>>,
) -> impl IntoResponse {
    let countries: Vec<serde_json::Value> =
        sqlx::query!(r#"SELECT id, alpha3, name FROM countries ORDER BY name"#,)
            .fetch_all(&state.pool)
            .await
            .unwrap_or_default()
            .into_iter()
            .map(|r| {
                serde_json::json!({
                    "id": r.id,
                    "countryCodeIsoAlpha3": r.alpha3,
                    "name": r.name,
                })
            })
            .collect();
    Json(countries)
}

pub async fn get_location_url() -> impl IntoResponse {
    Json(serde_json::json!({ "url": "/" }))
}

pub async fn get_passkeys(
    State(state): State<Arc<crate::AppState>>,
    headers: axum::http::HeaderMap,
) -> Result<Json<serde_json::Value>, AccountError> {
    load_account_protected(&state, &headers).await?;
    Ok(Json(serde_json::json!({ "items": [] })))
}

pub async fn get_approvals(
    State(state): State<Arc<crate::AppState>>,
    headers: axum::http::HeaderMap,
) -> Result<Json<serde_json::Value>, AccountError> {
    load_account_protected(&state, &headers).await?;
    Ok(Json(serde_json::json!({ "items": [] })))
}

pub async fn get_account_connections(
    State(state): State<Arc<crate::AppState>>,
    headers: axum::http::HeaderMap,
) -> Result<Json<serde_json::Value>, AccountError> {
    load_account_protected(&state, &headers).await?;
    Ok(Json(serde_json::json!({ "items": [] })))
}

// --- Commerce stubs (out of scope, return empty) ---

pub async fn get_transactions() -> impl IntoResponse {
    Json(EmptyArrayResponse { items: vec![] })
}

pub async fn get_wallet() -> impl IntoResponse {
    Json(serde_json::json!({ "balance": 0.0, "wallet_currency": "USD" }))
}

pub async fn get_external_subs() -> impl IntoResponse {
    Json(EmptyArrayResponse { items: vec![] })
}

pub async fn get_vc_last_used() -> impl IntoResponse {
    Json(EmptyArrayResponse { items: vec![] })
}

pub async fn get_vc_ecosystem() -> impl IntoResponse {
    Json(serde_json::json!({ "ecosystems": [] }))
}

pub async fn get_time_gated_games() -> impl IntoResponse {
    Json(EmptyArrayResponse { items: vec![] })
}

pub async fn get_game_account_creation_rules() -> impl IntoResponse {
    Json(serde_json::json!({
        "metadata": {"fieldMetadata": {
            "ptrCreation": {"readOnly": false, "hidden": false},
            "gameAccountRegions": {"readOnly": false, "hidden": false},
        }, "disableEditing": false},
        "gameAccountRegions": ["KR", "US", "EU"],
    }))
}

pub async fn get_details_battletag_rules() -> impl IntoResponse {
    // Real rules from Battle.net API capture.
    Json(serde_json::json!({
        "battleTagChangeType": "FREE",
        "characterSetRules": [
            {
                "battleTagMinLength": 3,
                "battleTagMaxLength": 12,
                "battleTagCharacterValidationRegex": "^[\\d\\u0041-\\u005a\\u0061-\\u007a\\u00c0-\\u00d6\\u00d8-\\u00f6\\u00f8-\\u017e\\u0180-\\u0188\\u0190-\\u0198\\u01c0-\\u0217]+$",
            },
            {
                "battleTagMinLength": 3,
                "battleTagMaxLength": 12,
                "battleTagCharacterValidationRegex": "^[\\d\\u0400-\\u04ff\\u0500-\\u052f]+$",
            },
        ],
        "additionalCharacterSetName": "cyrillic",
    }))
}

pub async fn get_age_verification(State(_state): State<Arc<crate::AppState>>) -> impl IntoResponse {
    Json(false)
}

pub async fn get_parental_controls() -> impl IntoResponse {
    (axum::http::StatusCode::FORBIDDEN, "")
}

pub async fn get_country_age_of_adulthood_map(
    State(state): State<Arc<crate::AppState>>,
) -> impl IntoResponse {
    let data: Vec<serde_json::Value> = sqlx::query!(
        r#"SELECT ca.country_id, c.alpha3, ca.adulthood_age
        FROM country_adulthood ca JOIN countries c ON c.id = ca.country_id
        ORDER BY ca.country_id"#,
    )
    .fetch_all(&state.pool)
    .await
    .unwrap_or_default()
    .into_iter()
    .map(|r| {
        serde_json::json!({
            "countryId": r.country_id,
            "countryCodeAlpha3": r.alpha3,
            "adulthoodAge": r.adulthood_age,
        })
    })
    .collect();
    Json(data)
}

pub async fn get_location_country_address_metadata() -> impl IntoResponse {
    Json(serde_json::json!({}))
}

// --- Address handlers (GET + PUT /api/details/address) ---

#[derive(Serialize)]
pub struct AddressResponse {
    primary: bool,
    #[serde(rename = "addressId")]
    address_id: i64,
    #[serde(rename = "firstName")]
    first_name: Option<String>,
    #[serde(rename = "lastName")]
    last_name: Option<String>,
    street1: Option<String>,
    street2: Option<String>,
    city: Option<String>,
    state: Option<String>,
    #[serde(rename = "countryCodeIsoAlpha3")]
    country_code_iso_alpha3: String,
    #[serde(rename = "postalCode")]
    postal_code: Option<String>,
}

#[derive(Deserialize)]
pub struct AddressRequest {
    #[serde(default)]
    #[allow(dead_code)]
    primary: Option<bool>,
    #[serde(rename = "addressId", default)]
    #[allow(dead_code)]
    address_id: Option<i64>,
    #[serde(rename = "firstName")]
    first_name: Option<String>,
    #[serde(rename = "lastName")]
    last_name: Option<String>,
    street1: Option<String>,
    street2: Option<String>,
    city: Option<String>,
    state: Option<String>,
    #[serde(rename = "countryCodeIsoAlpha3")]
    country_code_iso_alpha3: Option<String>,
    #[serde(rename = "postalCode")]
    postal_code: Option<String>,
}

/// `GET /api/details/address` — returns the account's address.
pub async fn get_address(
    State(state): State<Arc<crate::AppState>>,
    headers: axum::http::HeaderMap,
) -> Result<Json<AddressResponse>, AccountError> {
    let account = crate::api::load_account_protected(&state, &headers).await?;
    Ok(Json(AddressResponse {
        primary: true,
        address_id: account.id,
        first_name: account.first_name,
        last_name: account.last_name,
        street1: account.street1,
        street2: account.street2,
        city: account.city,
        state: account.state,
        country_code_iso_alpha3: account.country_code,
        postal_code: account.postal_code,
    }))
}

/// `PUT /api/details/address` — updates the account's address.
pub async fn put_address(
    State(state): State<Arc<crate::AppState>>,
    headers: axum::http::HeaderMap,
    Json(req): Json<AddressRequest>,
) -> Result<StatusCode, AccountError> {
    let mut account = crate::api::load_account_protected(&state, &headers).await?;
    account.first_name = req.first_name;
    account.last_name = req.last_name;
    account.street1 = req.street1;
    account.street2 = req.street2;
    account.city = req.city;
    account.state = req.state;
    account.postal_code = req.postal_code;
    if let Some(ref cca3) = req.country_code_iso_alpha3 {
        account.country_code = cca3.clone();
    }
    realmforge_gate_db::accounts::update(&state.pool, &account).await?;
    Ok(StatusCode::OK)
}
