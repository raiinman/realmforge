// SPDX-License-Identifier: AGPL-3.0-only

//! Repository for `account_licenses` — the per-account game license grants
//! returned by `AccountService.GetLicenses` (method 32) and the per-game-
//! account grants exposed via `AccountState.game_level_info.licenses`.
//!
//! Licenses may be account-scoped (`game_account_id IS NULL`) or bound to
//! a specific game account (legacy sub-accounts). Account-level grants act
//! as a fallback; game-account grants take precedence.

use sqlx::PgPool;

/// A license row as stored in the database.
#[derive(Debug, Clone)]
pub struct LicenseRow {
    pub license_id: i64,
    pub level: String,
    pub game_account_id: Option<i64>,
}

/// Find all licenses for an account, including game-account-bound grants.
pub async fn find_by_account(
    pool: &PgPool,
    account_id: i64,
) -> Result<Vec<LicenseRow>, sqlx::Error> {
    sqlx::query_as!(
        LicenseRow,
        r#"SELECT license_id, level, game_account_id
        FROM account_licenses
        WHERE account_id = $1
        ORDER BY game_account_id NULLS FIRST, license_id"#,
        account_id
    )
    .fetch_all(pool)
    .await
}

/// Find the account-level (fallback) licenses for an account.
pub async fn find_account_level(
    pool: &PgPool,
    account_id: i64,
) -> Result<Vec<LicenseRow>, sqlx::Error> {
    sqlx::query_as!(
        LicenseRow,
        r#"SELECT license_id, level, game_account_id
        FROM account_licenses
        WHERE account_id = $1 AND game_account_id IS NULL
        ORDER BY license_id"#,
        account_id
    )
    .fetch_all(pool)
    .await
}
