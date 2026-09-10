// SPDX-License-Identifier: AGPL-3.0-only

//! Credential repository.
//!
//! Compile-time-checked queries against the `credentials` table. Stores both
//! login schemes: the SRP6a verifier and the plaintext `sha_pass_hash`. Never
//! stores a reversible password.

use chrono::{DateTime, Utc};
use sqlx::Executor;
use tavern_core::Credential;

use crate::DbError;

/// Internal row type for the `credentials` table.
#[derive(sqlx::FromRow)]
struct CredentialRow {
    account_id: i64,
    srp_salt: Vec<u8>,
    srp_verifier: Vec<u8>,
    srp_iterations: i32,
    srp_version: i16,
    sha_pass_hash: Option<String>,
    updated_at: DateTime<Utc>,
}

impl From<CredentialRow> for Credential {
    fn from(row: CredentialRow) -> Self {
        Self {
            account_id: row.account_id,
            srp_salt: row.srp_salt,
            srp_verifier: row.srp_verifier,
            srp_iterations: row.srp_iterations,
            srp_version: row.srp_version,
            sha_pass_hash: row.sha_pass_hash,
            updated_at: row.updated_at,
        }
    }
}

/// Insert or replace an account's credential material (upsert on
/// `account_id`). Stores both the SRP6a verifier and the plaintext
/// `sha_pass_hash`.
pub async fn upsert<'e, E>(
    executor: E,
    account_id: i64,
    salt: &[u8],
    verifier: &[u8],
    iterations: i32,
    version: i16,
    sha_pass_hash: &str,
) -> Result<(), DbError>
where
    E: Executor<'e, Database = sqlx::Postgres>,
{
    sqlx::query!(
        r#"
        INSERT INTO credentials
            (account_id, srp_salt, srp_verifier, srp_iterations, srp_version, sha_pass_hash)
        VALUES ($1, $2, $3, $4, $5, $6)
        ON CONFLICT (account_id) DO UPDATE SET
            srp_salt = EXCLUDED.srp_salt,
            srp_verifier = EXCLUDED.srp_verifier,
            srp_iterations = EXCLUDED.srp_iterations,
            srp_version = EXCLUDED.srp_version,
            sha_pass_hash = EXCLUDED.sha_pass_hash,
            updated_at = NOW()
        "#,
        account_id,
        salt,
        verifier,
        iterations,
        version,
        sha_pass_hash,
    )
    .execute(executor)
    .await?;
    Ok(())
}

/// Find the SRP6a credential material for an account.
pub async fn find_by_account_id<'e, E>(
    executor: E,
    account_id: i64,
) -> Result<Option<Credential>, DbError>
where
    E: Executor<'e, Database = sqlx::Postgres>,
{
    let row = sqlx::query_as!(
        CredentialRow,
        r#"
        SELECT account_id, srp_salt, srp_verifier, srp_iterations, srp_version,
               sha_pass_hash, updated_at
        FROM credentials
        WHERE account_id = $1
        "#,
        account_id,
    )
    .fetch_optional(executor)
    .await?;
    Ok(row.map(Credential::from))
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
    async fn upsert_find_round_trip() {
        let Some(url) = db_url() else {
            eprintln!("skipping: DATABASE_URL not set");
            return;
        };

        let pool = crate::connect(&url).await.expect("connect");
        crate::run_migrations(&pool).await.expect("migrate");
        let mut tx = pool.begin().await.expect("begin tx");

        // credentials reference accounts; create a parent account first.
        let account = crate::accounts::create(&mut *tx, "cred@example.com")
            .await
            .expect("create account");

        let salt = [0u8; 16];
        let verifier = [1u8; 32];
        upsert(
            &mut *tx, account.id, &salt, &verifier, 15_000, 1, "aabbccdd",
        )
        .await
        .expect("upsert credential");

        let found = find_by_account_id(&mut *tx, account.id)
            .await
            .expect("find")
            .expect("credential exists");
        assert_eq!(found.account_id, account.id);
        assert_eq!(found.srp_salt, salt.to_vec());
        assert_eq!(found.srp_verifier, verifier.to_vec());
        assert_eq!(found.srp_iterations, 15_000);
        assert_eq!(found.srp_version, 1);

        // upsert replaces the verifier
        let new_verifier = [9u8; 32];
        upsert(
            &mut *tx,
            account.id,
            &salt,
            &new_verifier,
            15_000,
            2,
            "eeff0011",
        )
        .await
        .expect("upsert replace");
        let after = find_by_account_id(&mut *tx, account.id)
            .await
            .expect("find")
            .expect("credential exists");
        assert_eq!(after.srp_verifier, new_verifier.to_vec());
        assert_eq!(after.srp_version, 2);

        tx.rollback().await.expect("rollback");
    }
}
