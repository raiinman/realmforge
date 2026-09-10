// SPDX-License-Identifier: AGPL-3.0-only

//! Authorization code repository.

use chrono::{DateTime, Utc};
use realmforge_gate_core::AuthorizationCode;
use sqlx::Executor;

use crate::DbError;

#[derive(sqlx::FromRow)]
struct CodeRow {
    code: String,
    client_id: String,
    account_id: i64,
    scope: String,
    redirect_uri: String,
    code_challenge: Option<String>,
    nonce: Option<String>,
    expires_at: DateTime<Utc>,
    used: bool,
}

impl From<CodeRow> for AuthorizationCode {
    fn from(row: CodeRow) -> Self {
        Self {
            code: row.code,
            client_id: row.client_id,
            account_id: row.account_id,
            scope: row.scope,
            redirect_uri: row.redirect_uri,
            code_challenge: row.code_challenge,
            nonce: row.nonce,
            expires_at: row.expires_at,
            used: row.used,
        }
    }
}

/// Insert a fresh authorization code.
pub async fn insert<'e, E>(executor: E, code: &AuthorizationCode) -> Result<(), DbError>
where
    E: Executor<'e, Database = sqlx::Postgres>,
{
    sqlx::query!(
        r#"
        INSERT INTO authorization_codes
            (code, client_id, account_id, scope, redirect_uri, code_challenge, nonce, expires_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        "#,
        code.code,
        code.client_id.as_str(),
        code.account_id,
        code.scope.as_str(),
        code.redirect_uri.as_str(),
        code.code_challenge.as_deref(),
        code.nonce.as_deref(),
        code.expires_at,
    )
    .execute(executor)
    .await?;
    Ok(())
}

/// Atomically consume an unused, unexpired code. Returns `None` if the code is
/// missing, already used, or expired.
pub async fn consume<'e, E>(executor: E, code: &str) -> Result<Option<AuthorizationCode>, DbError>
where
    E: Executor<'e, Database = sqlx::Postgres>,
{
    let row = sqlx::query_as!(
        CodeRow,
        r#"
        UPDATE authorization_codes SET used = TRUE
        WHERE code = $1 AND used = FALSE AND expires_at > NOW()
        RETURNING code, client_id, account_id, scope, redirect_uri,
                  code_challenge, nonce, expires_at, used
        "#,
        code,
    )
    .fetch_optional(executor)
    .await?;
    Ok(row.map(AuthorizationCode::from))
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
    async fn insert_consume_round_trip() {
        let Some(url) = db_url() else {
            eprintln!("skipping: DATABASE_URL not set");
            return;
        };

        let pool = crate::connect(&url).await.expect("connect");
        crate::run_migrations(&pool).await.expect("migrate");
        let mut tx = pool.begin().await.expect("begin tx");

        let account = crate::accounts::create(&mut *tx, "auth-code@example.com")
            .await
            .expect("create account");
        let code = AuthorizationCode {
            code: "test-code-123".to_string(),
            client_id: "057adb2af62a4d59904f74754838c4c8".to_string(),
            account_id: account.id,
            scope: "openid".to_string(),
            redirect_uri: "https://account.battle.net/callback".to_string(),
            code_challenge: None,
            nonce: None,
            expires_at: Utc::now() + chrono::Duration::minutes(10),
            used: false,
        };

        insert(&mut *tx, &code).await.expect("insert code");

        let consumed = consume(&mut *tx, "test-code-123")
            .await
            .expect("consume")
            .expect("code consumed");
        assert_eq!(consumed.account_id, account.id);
        assert!(consumed.used);

        // Second consume returns None (already used).
        let replay = consume(&mut *tx, "test-code-123")
            .await
            .expect("consume replay");
        assert!(replay.is_none(), "used code cannot be consumed twice");

        tx.rollback().await.expect("rollback");
    }
}
