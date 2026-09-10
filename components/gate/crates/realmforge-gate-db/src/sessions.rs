// SPDX-License-Identifier: AGPL-3.0-only

//! Session repository.

use chrono::{DateTime, Utc};
use realmforge_gate_core::Session;
use sqlx::Executor;
use uuid::Uuid;

use crate::DbError;

#[derive(sqlx::FromRow)]
struct SessionRow {
    session_id: Uuid,
    account_id: i64,
    expires_at: DateTime<Utc>,
    created_at: DateTime<Utc>,
    user_agent: Option<String>,
    ip: Option<String>,
}

impl From<SessionRow> for Session {
    fn from(row: SessionRow) -> Self {
        Self {
            session_id: row.session_id,
            account_id: row.account_id,
            expires_at: row.expires_at,
            created_at: row.created_at,
            user_agent: row.user_agent,
            ip: row.ip,
        }
    }
}

/// Insert a new session.
pub async fn insert<'e, E>(executor: E, session: &Session) -> Result<(), DbError>
where
    E: Executor<'e, Database = sqlx::Postgres>,
{
    sqlx::query!(
        r#"
        INSERT INTO sessions (session_id, account_id, expires_at, created_at, user_agent, ip)
        VALUES ($1, $2, $3, $4, $5, $6)
        "#,
        session.session_id,
        session.account_id,
        session.expires_at,
        session.created_at,
        session.user_agent.as_deref(),
        session.ip.as_deref(),
    )
    .execute(executor)
    .await?;
    Ok(())
}

/// Find a session by its id.
pub async fn find_by_id<'e, E>(executor: E, session_id: Uuid) -> Result<Option<Session>, DbError>
where
    E: Executor<'e, Database = sqlx::Postgres>,
{
    let row = sqlx::query_as!(
        SessionRow,
        r#"
        SELECT session_id, account_id, expires_at, created_at, user_agent, ip
        FROM sessions
        WHERE session_id = $1
        "#,
        session_id,
    )
    .fetch_optional(executor)
    .await?;
    Ok(row.map(Session::from))
}

/// Find the most recent non-expired session, typically for resolving the
/// SESSIONID cookie after the first hop of the OAuth authorize redirect.
pub async fn find_recent<'e, E>(executor: E) -> Result<Option<Session>, DbError>
where
    E: Executor<'e, Database = sqlx::Postgres>,
{
    let row = sqlx::query_as!(
        SessionRow,
        r#"
        SELECT session_id, account_id, expires_at, created_at, user_agent, ip
        FROM sessions
        WHERE expires_at > NOW()
        ORDER BY created_at DESC
        LIMIT 1
        "#,
    )
    .fetch_optional(executor)
    .await?;
    Ok(row.map(Session::from))
}

/// Delete (invalidate) a session.
pub async fn delete<'e, E>(executor: E, session_id: Uuid) -> Result<bool, DbError>
where
    E: Executor<'e, Database = sqlx::Postgres>,
{
    let result = sqlx::query!("DELETE FROM sessions WHERE session_id = $1", session_id)
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
    async fn insert_find_delete_round_trip() {
        let Some(url) = db_url() else {
            eprintln!("skipping: DATABASE_URL not set");
            return;
        };

        let pool = crate::connect(&url).await.expect("connect");
        crate::run_migrations(&pool).await.expect("migrate");
        let mut tx = pool.begin().await.expect("begin tx");

        let account = crate::accounts::create(&mut *tx, "session@example.com")
            .await
            .expect("create account");

        let session = Session {
            session_id: Uuid::new_v4(),
            account_id: account.id,
            expires_at: Utc::now() + chrono::Duration::hours(24),
            created_at: Utc::now(),
            user_agent: Some("test-agent".to_string()),
            ip: Some("127.0.0.1".to_string()),
        };

        insert(&mut *tx, &session).await.expect("insert session");
        let found = find_by_id(&mut *tx, session.session_id)
            .await
            .expect("find")
            .expect("session exists");
        assert_eq!(found.account_id, account.id);
        assert_eq!(found.user_agent.as_deref(), Some("test-agent"));

        let deleted = delete(&mut *tx, session.session_id).await.expect("delete");
        assert!(deleted);

        tx.rollback().await.expect("rollback");
    }
}
