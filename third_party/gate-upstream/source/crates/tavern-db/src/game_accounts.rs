// SPDX-License-Identifier: AGPL-3.0-only

//! Repository for `game_accounts` — the per-account game account rows
//! (e.g. "WoW1", "WoW2") created during account registration.

use sqlx::PgPool;

/// A game account row as stored in the database.
#[derive(Debug, Clone)]
pub struct GameAccountRow {
    pub id: i64,
    pub name: String,
    pub region: i16,
    pub is_suspended: bool,
    pub is_banned: bool,
    pub suspension_expires: Option<chrono::DateTime<chrono::Utc>>,
    pub game_time_expires: Option<chrono::DateTime<chrono::Utc>>,
}

/// Find all game accounts for an account.
pub async fn find_by_account(
    pool: &PgPool,
    account_id: i64,
) -> Result<Vec<GameAccountRow>, sqlx::Error> {
    sqlx::query_as!(
        GameAccountRow,
        "SELECT id, name, region, is_suspended, is_banned, suspension_expires, game_time_expires \
         FROM game_accounts WHERE account_id = $1 ORDER BY id",
        account_id
    )
    .fetch_all(pool)
    .await
}
