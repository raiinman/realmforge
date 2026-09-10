// SPDX-License-Identifier: AGPL-3.0-only

//! Account registration: create an account with dual-credential storage.

use realmforge_gate_core::srp;
use realmforge_gate_db::{accounts, credentials};

use crate::AccountError;

/// Register a new account. Stores credentials in both schemes so any client
/// line can authenticate: the BnetSRP6v2 verifier (2.5+/3.4/4.4) and the
/// plaintext `sha_pass_hash` (1.13/1.14 Vanilla). Creates a default game
/// account. Returns the new account id.
pub async fn register(
    pool: &sqlx::PgPool,
    group: &srp::Group,
    email: &str,
    password: &str,
) -> Result<i64, AccountError> {
    let mut tx = pool.begin().await?;

    let account = accounts::create(&mut *tx, email).await?;

    // SRP6a verifier for the 2.5+/3.4/4.4 lines.
    let salt = srp::generate_salt();
    let verifier = srp::compute_verifier(group, email, password, &salt, srp::ITERATIONS);
    let verifier_bytes = verifier.to_bytes_be().1;

    // Plaintext-line hash for 1.13/1.14 Vanilla.
    let sha_pass = srp::compute_sha_pass_hash(email, password);

    credentials::upsert(
        &mut *tx,
        account.id,
        &salt,
        &verifier_bytes,
        srp::ITERATIONS as i32,
        2,
        &sha_pass,
    )
    .await?;

    // Default game account: "<accountId>#1".
    let game_name = format!("{}#1", account.id);
    sqlx::query!(
        "INSERT INTO game_accounts (account_id, name) VALUES ($1, $2)",
        account.id,
        game_name,
    )
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    tracing::info!(account_id = account.id, "account registered");
    Ok(account.id)
}

/// Verify a plaintext password against the stored `sha_pass_hash`. Used by the
/// Vanilla bnet line (1.13/1.14) where the client sends the password in
/// plaintext over TLS.
pub fn verify_plaintext(login: &str, password: &str, stored_hash: &str) -> bool {
    let computed = srp::compute_sha_pass_hash(login, password);
    constant_time_eq(computed.as_bytes(), stored_hash.as_bytes())
}

/// Generate a login ticket string in the captured format:
/// `<REGION>-<32-char-hex>-<accountId>` (e.g. `KR-c0900ff7...-1505337751`).
pub(crate) fn format_ticket(region: &str, account_id: i64) -> String {
    use rand::RngCore;
    let mut bytes = [0u8; 16];
    rand::thread_rng().fill_bytes(&mut bytes);
    format!("{region}-{}-{account_id}", hex::encode(bytes))
}

fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}
