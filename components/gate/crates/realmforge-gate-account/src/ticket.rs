// SPDX-License-Identifier: AGPL-3.0-only

//! Login ticket minting: the shared backend every transport delegates to.
//!
//! `mint_login_ticket` creates an opaque, one-time service ticket bound to an
//! account. The OAuth authorize endpoint (M9) and the bnetserver (M8) both
//! consume it to bridge an authenticated login into the next flow step.

use chrono::{Duration, Utc};
use realmforge_gate_core::ServiceTicket;
use realmforge_gate_db::service_tickets;

use crate::{AccountError, registration};

/// Mint a one-time login ticket for the given account. The ticket expires after
/// 5 minutes and is consumed atomically by the first caller that validates it.
pub async fn mint_login_ticket(
    pool: &sqlx::PgPool,
    account_id: i64,
) -> Result<String, AccountError> {
    mint_login_ticket_with_region(pool, account_id, "US").await
}

/// Mint a one-time login ticket with a specific region tag.
pub async fn mint_login_ticket_with_region(
    pool: &sqlx::PgPool,
    account_id: i64,
    region: &str,
) -> Result<String, AccountError> {
    let ticket = registration::format_ticket(region, account_id);
    let region_code = match region {
        "EU" => 1,
        "KR" => 2,
        "TW" => 3,
        _ => 0,
    };
    service_tickets::insert(
        pool,
        &ServiceTicket {
            st: ticket.clone(),
            account_id,
            region: region_code,
            expires_at: Utc::now() + Duration::minutes(5),
            used: false,
        },
    )
    .await?;
    Ok(ticket)
}

/// Consume a one-time login ticket. Returns the account id on success, or
/// `None` if the ticket is missing, expired, or already used.
pub async fn consume_login_ticket(
    pool: &sqlx::PgPool,
    st: &str,
) -> Result<Option<i64>, AccountError> {
    let ticket = service_tickets::consume(pool, st).await?;
    Ok(ticket.map(|t| t.account_id))
}
