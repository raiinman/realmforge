// SPDX-License-Identifier: AGPL-3.0-only

//! Service ticket repository.
//!
//! Service tickets are one-time tokens that bridge an authenticated web session
//! (after SRP login) into the OAuth authorize flow.

use chrono::{DateTime, Utc};
use realmforge_gate_core::ServiceTicket;
use sqlx::Executor;

use crate::DbError;

#[derive(sqlx::FromRow)]
struct TicketRow {
    st: String,
    account_id: i64,
    region: i16,
    expires_at: DateTime<Utc>,
    used: bool,
}

impl From<TicketRow> for ServiceTicket {
    fn from(row: TicketRow) -> Self {
        Self {
            st: row.st,
            account_id: row.account_id,
            region: row.region,
            expires_at: row.expires_at,
            used: row.used,
        }
    }
}

/// Insert a fresh service ticket.
pub async fn insert<'e, E>(executor: E, ticket: &ServiceTicket) -> Result<(), DbError>
where
    E: Executor<'e, Database = sqlx::Postgres>,
{
    sqlx::query!(
        r#"
        INSERT INTO service_tickets (st, account_id, region, expires_at)
        VALUES ($1, $2, $3, $4)
        "#,
        ticket.st,
        ticket.account_id,
        ticket.region,
        ticket.expires_at,
    )
    .execute(executor)
    .await?;
    Ok(())
}

/// Atomically consume an unused, unexpired ticket. Returns the account id on
/// success, or `None` if the ticket is missing, used, or expired.
pub async fn consume<'e, E>(executor: E, st: &str) -> Result<Option<ServiceTicket>, DbError>
where
    E: Executor<'e, Database = sqlx::Postgres>,
{
    let row = sqlx::query_as!(
        TicketRow,
        r#"
        UPDATE service_tickets SET used = TRUE
        WHERE st = $1 AND used = FALSE AND expires_at > NOW()
        RETURNING st, account_id, region, expires_at, used
        "#,
        st,
    )
    .fetch_optional(executor)
    .await?;
    Ok(row.map(ServiceTicket::from))
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

        let account = crate::accounts::create(&mut *tx, "ticket@example.com")
            .await
            .expect("create account");

        let ticket = ServiceTicket {
            st: "ST-test-001".to_string(),
            account_id: account.id,
            region: 1,
            expires_at: Utc::now() + chrono::Duration::minutes(5),
            used: false,
        };

        insert(&mut *tx, &ticket).await.expect("insert ticket");
        let consumed = consume(&mut *tx, "ST-test-001")
            .await
            .expect("consume")
            .expect("ticket consumed");
        assert_eq!(consumed.account_id, account.id);
        assert_eq!(consumed.region, 1);

        let replay = consume(&mut *tx, "ST-test-001")
            .await
            .expect("consume replay");
        assert!(replay.is_none(), "ticket cannot be consumed twice");

        tx.rollback().await.expect("rollback");
    }
}
