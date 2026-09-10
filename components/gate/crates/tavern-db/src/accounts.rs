// SPDX-License-Identifier: AGPL-3.0-only

//! Account repository.
//!
//! Compile-time-checked queries against the `accounts` table. Public functions
//! take an [`sqlx::Executor`] so they run against either the pool or a
//! transaction, and return [`tavern_core::Account`] domain types.

use chrono::{DateTime, Utc};
use sqlx::Executor;
use tavern_core::Account;

use crate::DbError;

/// Internal row type for the `accounts` table.
///
/// Kept private: the public API returns the domain [`Account`]. Decoupling the
/// row type from the domain keeps `tavern-core` free of any `sqlx` dependency.
#[derive(sqlx::FromRow)]
struct AccountRow {
    id: i64,
    email: String,
    email_verified: bool,
    battletag: String,
    country_code: String,
    country_id: Option<i32>,
    region: i16,
    locale: String,
    first_name: Option<String>,
    last_name: Option<String>,
    birth_date: Option<String>,
    mobile_number: Option<String>,
    street1: Option<String>,
    street2: Option<String>,
    city: Option<String>,
    state: Option<String>,
    postal_code: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl From<AccountRow> for Account {
    fn from(row: AccountRow) -> Self {
        Self {
            id: row.id,
            email: row.email,
            email_verified: row.email_verified,
            battletag: row.battletag,
            country_code: row.country_code,
            country_id: row.country_id,
            region: row.region,
            locale: row.locale,
            first_name: row.first_name,
            last_name: row.last_name,
            birth_date: row.birth_date,
            mobile_number: row.mobile_number,
            street1: row.street1,
            street2: row.street2,
            city: row.city,
            state: row.state,
            postal_code: row.postal_code,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

/// Create an account with the given email. Other fields take their schema
/// defaults. Returns the created row.
pub async fn create<'e, E>(executor: E, email: &str) -> Result<Account, DbError>
where
    E: Executor<'e, Database = sqlx::Postgres>,
{
    let row = sqlx::query_as!(
        AccountRow,
        r#"
        INSERT INTO accounts (email)
        VALUES ($1)
        RETURNING id, email, email_verified, battletag, country_code, country_id,
                  region, locale, first_name, last_name, birth_date, mobile_number,
                  street1, street2, city, state, postal_code,
                  created_at, updated_at
        "#,
        email
    )
    .fetch_one(executor)
    .await?;
    Ok(row.into())
}

/// Find an account by its numeric id.
pub async fn find_by_id<'e, E>(executor: E, id: i64) -> Result<Option<Account>, DbError>
where
    E: Executor<'e, Database = sqlx::Postgres>,
{
    let row = sqlx::query_as!(
        AccountRow,
        r#"
        SELECT id, email, email_verified, battletag, country_code, country_id,
               region, locale, first_name, last_name, birth_date, mobile_number,
               street1, street2, city, state, postal_code,
               created_at, updated_at
        FROM accounts
        WHERE id = $1
        "#,
        id
    )
    .fetch_optional(executor)
    .await?;
    Ok(row.map(Account::from))
}

/// Find an account by email. Match is case-insensitive (the `email` column is
/// `CITEXT`).
pub async fn find_by_email<'e, E>(executor: E, email: &str) -> Result<Option<Account>, DbError>
where
    E: Executor<'e, Database = sqlx::Postgres>,
{
    let row = sqlx::query_as!(
        AccountRow,
        r#"
        SELECT id, email, email_verified, battletag, country_code, country_id,
               region, locale, first_name, last_name, birth_date, mobile_number,
               street1, street2, city, state, postal_code,
               created_at, updated_at
        FROM accounts
        WHERE email = $1::citext
        "#,
        email
    )
    .fetch_optional(executor)
    .await?;
    Ok(row.map(Account::from))
}

/// Update the mutable profile fields of an account, refreshing `updated_at`.
/// Returns the updated row, or `None` if no account has the given id.
pub async fn update<'e, E>(executor: E, account: &Account) -> Result<Option<Account>, DbError>
where
    E: Executor<'e, Database = sqlx::Postgres>,
{
    let row = sqlx::query_as!(
        AccountRow,
        r#"
        UPDATE accounts SET
            email = $2,
            email_verified = $3,
            battletag = $4,
            country_code = $5,
            country_id = $6,
            region = $7,
            locale = $8,
            first_name = $9,
            last_name = $10,
            birth_date = $11,
            mobile_number = $12,
            street1 = $13,
            street2 = $14,
            city = $15,
            state = $16,
            postal_code = $17,
            updated_at = NOW()
        WHERE id = $1
        RETURNING id, email, email_verified, battletag, country_code, country_id,
                  region, locale, first_name, last_name, birth_date, mobile_number,
                  street1, street2, city, state, postal_code,
                  created_at, updated_at
        "#,
        account.id,
        account.email.as_str(),
        account.email_verified,
        account.battletag.as_str(),
        account.country_code.as_str(),
        account.country_id,
        account.region,
        account.locale.as_str(),
        account.first_name.as_deref(),
        account.last_name.as_deref(),
        account.birth_date.as_deref(),
        account.mobile_number.as_deref(),
        account.street1.as_deref(),
        account.street2.as_deref(),
        account.city.as_deref(),
        account.state.as_deref(),
        account.postal_code.as_deref(),
    )
    .fetch_optional(executor)
    .await?;
    Ok(row.map(Account::from))
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
    async fn create_find_update_round_trip() {
        let Some(url) = db_url() else {
            eprintln!("skipping: DATABASE_URL not set");
            return;
        };

        let pool = crate::connect(&url).await.expect("connect");
        crate::run_migrations(&pool).await.expect("migrate");
        let mut tx = pool.begin().await.expect("begin tx");

        let created = create(&mut *tx, "repo@example.com").await.expect("create");
        assert!(created.id > 0);
        assert_eq!(created.email, "repo@example.com");
        assert_eq!(created.region, 0);
        assert_eq!(created.locale, "enUS");

        // find by id
        let found = find_by_id(&mut *tx, created.id)
            .await
            .expect("find by id")
            .expect("row exists");
        assert_eq!(found.email, created.email);

        // CITEXT: case-insensitive email match
        let found_ci = find_by_email(&mut *tx, "REPO@example.com")
            .await
            .expect("find by email");
        assert!(found_ci.is_some(), "CITEXT should match case-insensitively");

        // update profile fields
        let mut updated = found;
        updated.battletag = "Repo#1234".to_string();
        updated.region = 1;
        updated.locale = "deDE".to_string();
        let after = update(&mut *tx, &updated)
            .await
            .expect("update")
            .expect("row updated");
        assert_eq!(after.battletag, "Repo#1234");
        assert_eq!(after.region, 1);
        assert_eq!(after.locale, "deDE");

        tx.rollback().await.expect("rollback");
    }
}
