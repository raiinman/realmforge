// SPDX-License-Identifier: AGPL-3.0-only

//! Refresh token repository.

use chrono::{DateTime, Utc};
use sqlx::Executor;
use tavern_core::RefreshToken;

use crate::DbError;

#[derive(sqlx::FromRow)]
struct TokenRow {
    token: String,
    account_id: i64,
    client_id: String,
    scope: String,
    expires_at: DateTime<Utc>,
    rotated_from: Option<String>,
}

impl From<TokenRow> for RefreshToken {
    fn from(row: TokenRow) -> Self {
        Self {
            token: row.token,
            account_id: row.account_id,
            client_id: row.client_id,
            scope: row.scope,
            expires_at: row.expires_at,
            rotated_from: row.rotated_from,
        }
    }
}

/// Insert a refresh token.
pub async fn insert<'e, E>(executor: E, token: &RefreshToken) -> Result<(), DbError>
where
    E: Executor<'e, Database = sqlx::Postgres>,
{
    sqlx::query!(
        r#"
        INSERT INTO refresh_tokens (token, account_id, client_id, scope, expires_at, rotated_from)
        VALUES ($1, $2, $3, $4, $5, $6)
        "#,
        token.token,
        token.account_id,
        token.client_id.as_str(),
        token.scope.as_str(),
        token.expires_at,
        token.rotated_from.as_deref(),
    )
    .execute(executor)
    .await?;
    Ok(())
}

/// Find a refresh token. Returns `None` if it does not exist.
pub async fn find<'e, E>(executor: E, token: &str) -> Result<Option<RefreshToken>, DbError>
where
    E: Executor<'e, Database = sqlx::Postgres>,
{
    let row = sqlx::query_as!(
        TokenRow,
        r#"
        SELECT token, account_id, client_id, scope, expires_at, rotated_from
        FROM refresh_tokens
        WHERE token = $1
        "#,
        token,
    )
    .fetch_optional(executor)
    .await?;
    Ok(row.map(RefreshToken::from))
}

/// Revoke (delete) a refresh token. Returns `true` if a token was removed.
pub async fn revoke<'e, E>(executor: E, token: &str) -> Result<bool, DbError>
where
    E: Executor<'e, Database = sqlx::Postgres>,
{
    let result = sqlx::query!("DELETE FROM refresh_tokens WHERE token = $1", token)
        .execute(executor)
        .await?;
    Ok(result.rows_affected() > 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn db_url() -> Option<String> {
        std::env::var("DATABASE_URL")
            .ok()
            .map(|s| s.trim().to_owned())
            .filter(|s| !s.is_empty())
    }

    #[tokio::test]
    async fn insert_find_revoke_round_trip() {
        let Some(url) = db_url() else {
            eprintln!("skipping: DATABASE_URL not set");
            return;
        };

        let pool = crate::connect(&url).await.expect("connect");
        crate::run_migrations(&pool).await.expect("migrate");
        let mut tx = pool.begin().await.expect("begin tx");

        let account = crate::accounts::create(&mut *tx, "refresh@example.com")
            .await
            .expect("create account");

        let token = RefreshToken {
            token: "rt-abc".to_string(),
            account_id: account.id,
            client_id: "057adb2af62a4d59904f74754838c4c8".to_string(),
            scope: "openid".to_string(),
            expires_at: Utc::now() + chrono::Duration::days(30),
            rotated_from: None,
        };

        insert(&mut *tx, &token).await.expect("insert token");
        let found = find(&mut *tx, "rt-abc")
            .await
            .expect("find")
            .expect("token exists");
        assert_eq!(found.account_id, account.id);

        let revoked = revoke(&mut *tx, "rt-abc").await.expect("revoke");
        assert!(revoked, "token was revoked");

        let after = find(&mut *tx, "rt-abc").await.expect("find");
        assert!(after.is_none(), "revoked token is gone");

        tx.rollback().await.expect("rollback");
    }
}
