// SPDX-License-Identifier: AGPL-3.0-only

//! Repository for RFC 8628 device authorization codes.
//!
//! Device authorization codes are short-lived (10 minutes). They bridge the
//! `/device/code` endpoint (device) with the interactive `/device` approval
//! page (browser) and the `/token` polling endpoint (device).

use chrono::{DateTime, Utc};
use sqlx::PgPool;

use crate::DbError;

/// A pending device authorization flow.
#[derive(Debug, Clone)]
pub struct DeviceAuthorization {
    pub device_code: String,
    pub user_code: String,
    pub client_id: String,
    pub account_id: Option<i64>,
    pub scope: String,
    pub expires_at: DateTime<Utc>,
    pub approved: bool,
}

/// Insert a new device authorization.
pub async fn insert(pool: &PgPool, auth: &DeviceAuthorization) -> Result<(), DbError> {
    sqlx::query!(
        "INSERT INTO device_authorizations (device_code, user_code, client_id, scope, expires_at)
         VALUES ($1, $2, $3, $4, $5)",
        auth.device_code,
        auth.user_code,
        auth.client_id,
        auth.scope,
        auth.expires_at,
    )
    .execute(pool)
    .await?;
    Ok(())
}

/// Find by device_code. Returns None if not found or expired.
pub async fn find_by_device_code(
    pool: &PgPool,
    device_code: &str,
) -> Result<Option<DeviceAuthorization>, DbError> {
    let row = sqlx::query!(
        "SELECT device_code, user_code, client_id, account_id, scope,
                expires_at, approved
         FROM device_authorizations
         WHERE device_code = $1 AND expires_at > NOW()",
        device_code,
    )
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|r| DeviceAuthorization {
        device_code: r.device_code,
        user_code: r.user_code,
        client_id: r.client_id,
        account_id: r.account_id,
        scope: r.scope,
        expires_at: r.expires_at,
        approved: r.approved,
    }))
}

/// Find by user_code. Returns None if not found or expired.
pub async fn find_by_user_code(
    pool: &PgPool,
    user_code: &str,
) -> Result<Option<DeviceAuthorization>, DbError> {
    let row = sqlx::query!(
        "SELECT device_code, user_code, client_id, account_id, scope,
                expires_at, approved
         FROM device_authorizations
         WHERE UPPER(user_code) = UPPER($1) AND expires_at > NOW()",
        user_code,
    )
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|r| DeviceAuthorization {
        device_code: r.device_code,
        user_code: r.user_code,
        client_id: r.client_id,
        account_id: r.account_id,
        scope: r.scope,
        expires_at: r.expires_at,
        approved: r.approved,
    }))
}

/// Approve a device authorization by user_code.
pub async fn approve_by_user_code(
    pool: &PgPool,
    user_code: &str,
    account_id: i64,
) -> Result<bool, DbError> {
    let result = sqlx::query!(
        "UPDATE device_authorizations
         SET approved = TRUE, account_id = $1
         WHERE UPPER(user_code) = UPPER($2) AND expires_at > NOW()",
        account_id,
        user_code,
    )
    .execute(pool)
    .await?;
    Ok(result.rows_affected() > 0)
}

/// Delete a device authorization.
pub async fn delete(pool: &PgPool, device_code: &str) -> Result<(), DbError> {
    sqlx::query!(
        "DELETE FROM device_authorizations WHERE device_code = $1",
        device_code,
    )
    .execute(pool)
    .await?;
    Ok(())
}
