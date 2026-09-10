// SPDX-License-Identifier: AGPL-3.0-only

//! BGS session token repository for session restore.

use chrono::{DateTime, Utc};
use sqlx::Executor;

use crate::DbError;

#[derive(sqlx::FromRow)]
pub struct BgsSessionRow {
    pub sso_id: Vec<u8>,
    pub account_id: i64,
    pub battle_tag: String,
    pub build: Option<i32>,
    pub platform: Option<String>,
    pub locale: Option<String>,
    pub expires_at: DateTime<Utc>,
    /// The 64-byte BGS session key, returned as `Param_BnetSessionKey`
    /// at realm join so restored sessions can still complete the world
    /// handshake (TLS material).
    pub session_key: Option<Vec<u8>>,
}

/// Insert a BGS session token (valid for 24 hours).
// One caller, one explicit column list; the count stays legible.
#[allow(clippy::too_many_arguments)]
pub async fn insert<'e, E>(
    executor: E,
    sso_id: &[u8],
    account_id: i64,
    battle_tag: &str,
    build: Option<i32>,
    platform: Option<&str>,
    locale: Option<&str>,
    session_key: Option<&[u8]>,
) -> Result<(), DbError>
where
    E: Executor<'e, Database = sqlx::Postgres>,
{
    sqlx::query!(
        "INSERT INTO bgs_sessions (sso_id, account_id, battle_tag, build, platform, locale, session_key, expires_at) VALUES ($1, $2, $3, $4, $5, $6, $7, NOW() + INTERVAL '24 hours')",
        sso_id,
        account_id,
        battle_tag,
        build,
        platform,
        locale,
        session_key,
    )
    .execute(executor)
    .await?;
    Ok(())
}

/// Look up a valid (non-expired) BGS session by SSO ID.
pub async fn find_valid<'e, E>(executor: E, sso_id: &[u8]) -> Result<Option<BgsSessionRow>, DbError>
where
    E: Executor<'e, Database = sqlx::Postgres>,
{
    let row = sqlx::query_as!(
        BgsSessionRow,
        "SELECT sso_id, account_id, battle_tag, build, platform, locale, expires_at, session_key FROM bgs_sessions WHERE sso_id = $1 AND expires_at > NOW()",
        sso_id,
    )
    .fetch_optional(executor)
    .await?;
    Ok(row)
}
