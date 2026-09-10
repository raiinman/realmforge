// SPDX-License-Identifier: AGPL-3.0-only

//! Session resolution and XSRF double-submit validation.
//!
//! After the OAuth callback (A.7) establishes the management session, every
//! `/api/*` request is authenticated via the `SESSIONID` cookie and protected
//! by the `X-XSRF-TOKEN` double-submit CSRF pattern.

use std::sync::Arc;

use axum::http::HeaderMap;
use base64::Engine;
use uuid::Uuid;

use tavern_db::sessions;

use crate::{AccountError, AppState};

/// Resolve the account id from the `SESSIONID` cookie. Returns `NotFound` if
/// the session is missing, expired, or invalid.
pub async fn resolve_session(
    state: &Arc<AppState>,
    headers: &HeaderMap,
) -> Result<i64, AccountError> {
    let session_id = extract_cookie(headers, "SESSIONID")
        .ok_or_else(|| AccountError::BadRequest("missing SESSIONID cookie".into()))?;

    let uuid = decode_session_id(&session_id)
        .ok_or_else(|| AccountError::BadRequest("invalid SESSIONID cookie".into()))?;

    let session = sessions::find_by_id(&state.pool, uuid)
        .await?
        .ok_or(AccountError::NotFound)?;

    if session.expires_at < chrono::Utc::now() {
        return Err(AccountError::NotFound);
    }

    Ok(session.account_id)
}

/// Validate the XSRF double-submit: the `X-XSRF-TOKEN` header must match the
/// `XSRF-TOKEN` cookie. Used by non-bootstrap `/api/*` endpoints.
pub fn validate_xsrf(headers: &HeaderMap) -> Result<(), AccountError> {
    let header_token = headers
        .get("x-xsrf-token")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| AccountError::BadRequest("missing X-XSRF-TOKEN header".into()))?;

    let cookie_token = extract_cookie(headers, "XSRF-TOKEN")
        .ok_or_else(|| AccountError::BadRequest("missing XSRF-TOKEN cookie".into()))?;

    if header_token != cookie_token {
        return Err(AccountError::BadRequest("XSRF token mismatch".into()));
    }

    Ok(())
}

/// Resolve session AND validate XSRF in one call. Used by protected endpoints.
pub async fn resolve_and_validate(
    state: &Arc<AppState>,
    headers: &HeaderMap,
) -> Result<i64, AccountError> {
    validate_xsrf(headers)?;
    resolve_session(state, headers).await
}

/// Extract a cookie value by name from the Cookie header.
fn extract_cookie(headers: &HeaderMap, name: &str) -> Option<String> {
    let cookie_header = headers.get(axum::http::header::COOKIE)?.to_str().ok()?;
    for pair in cookie_header.split(';') {
        let pair = pair.trim();
        if let Some((_, v)) = pair.split_once('=').filter(|(k, _)| *k == name) {
            return Some(v.to_string());
        }
    }
    None
}

/// Decode a SESSIONID value: it's a base64-encoded UUID (captured format).
fn decode_session_id(raw: &str) -> Option<Uuid> {
    // The capture shows base64-encoded UUIDs (e.g.,
    // "ZmVjZTgxYjItNzc4Mi00ZDFiLTk2NTEtOGU2NWJhOWUyOTI3"). Tavern also
    // accepts plain UUIDs for simplicity.
    if let Ok(uuid) = Uuid::parse_str(raw) {
        return Some(uuid);
    }
    // Try base64 decode.
    if let Ok(bytes) = base64::engine::general_purpose::STANDARD.decode(raw)
        && let Ok(s) = std::str::from_utf8(&bytes)
        && let Ok(uuid) = Uuid::parse_str(s)
    {
        return Some(uuid);
    }
    None
}

/// Encode a UUID as base64 for the SESSIONID cookie (captured format).
pub fn encode_session_id(uuid: Uuid) -> String {
    base64::engine::general_purpose::STANDARD.encode(uuid.to_string())
}

/// Encode a session token for auth permits. Produces an opaque, verifiable
/// token from account_id + signing key (HMAC-SHA256, prefix-stripped).
pub fn encode_session_token(account_id: i64, signing_key_pem: &str) -> String {
    use hmac::Mac;
    use sha2::Sha256;

    let mut mac = hmac::Hmac::<Sha256>::new_from_slice(signing_key_pem.as_bytes())
        // PANIC: unreachable. HMAC accepts keys of any length, so
        // new_from_slice never returns Err for Hmac<Sha256>.
        .expect("HMAC-SHA256 key init is infallible");
    mac.update(&account_id.to_le_bytes());
    mac.update(b":auth-permit");
    let result = mac.finalize();
    format!("AP-{}", hex::encode(&result.into_bytes()[..16]))
}

/// Extract the SESSIONID from a cookie header and decode to UUID.
pub fn extract_session_id(headers: &HeaderMap) -> Option<Uuid> {
    let raw = extract_cookie(headers, "SESSIONID")?;
    decode_session_id(&raw)
}
