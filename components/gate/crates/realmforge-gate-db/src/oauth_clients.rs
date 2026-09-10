// SPDX-License-Identifier: AGPL-3.0-only

//! OAuth client repository.

use realmforge_gate_core::OAuthClient;
use sqlx::Executor;

use crate::DbError;

#[derive(sqlx::FromRow)]
struct ClientRow {
    client_id: String,
    client_secret_hash: Option<String>,
    redirect_uris: Vec<String>,
    scopes: Vec<String>,
    allowed_grants: Vec<String>,
    require_2fa: bool,
}

impl From<ClientRow> for OAuthClient {
    fn from(row: ClientRow) -> Self {
        Self {
            client_id: row.client_id,
            client_secret_hash: row.client_secret_hash,
            redirect_uris: row.redirect_uris,
            scopes: row.scopes,
            allowed_grants: row.allowed_grants,
            require_2fa: row.require_2fa,
        }
    }
}

/// Find a registered OAuth client by its id.
pub async fn find_by_id<'e, E>(executor: E, client_id: &str) -> Result<Option<OAuthClient>, DbError>
where
    E: Executor<'e, Database = sqlx::Postgres>,
{
    let row = sqlx::query_as!(
        ClientRow,
        r#"
        SELECT client_id, client_secret_hash, redirect_uris, scopes,
               allowed_grants, require_2fa
        FROM oauth_clients
        WHERE client_id = $1
        "#,
        client_id,
    )
    .fetch_optional(executor)
    .await?;
    Ok(row.map(OAuthClient::from))
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
    async fn seeded_clients_are_queryable() {
        let Some(url) = db_url() else {
            eprintln!("skipping: DATABASE_URL not set");
            return;
        };

        let pool = crate::connect(&url).await.expect("connect");
        crate::run_migrations(&pool).await.expect("migrate");

        let account_settings = find_by_id(&pool, "057adb2af62a4d59904f74754838c4c8")
            .await
            .expect("find")
            .expect("account-settings client seeded");
        assert!(!account_settings.redirect_uris.is_empty());
        assert!(
            account_settings
                .scopes
                .contains(&"account.full".to_string())
        );
        assert!(!account_settings.require_2fa);

        let dev_portal = find_by_id(&pool, "d24c87becefa45e788e556ed1f2386e3")
            .await
            .expect("find")
            .expect("dev-portal client seeded");
        assert!(dev_portal.require_2fa);

        assert!(
            find_by_id(&pool, "nonexistent")
                .await
                .expect("find")
                .is_none(),
            "unknown client returns None"
        );
    }
}
