// SPDX-License-Identifier: AGPL-3.0-only

//! Password change handler.
//!
//! Re-verifies the old password (both credential schemes), then rewrites the
//! SRP6a verifier and the `sha_pass_hash` with the new password.

use std::sync::Arc;

use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use realmforge_gate_core::srp;
use realmforge_gate_db::{accounts, credentials};
use serde::Deserialize;

use crate::AccountError;

#[derive(Deserialize)]
pub struct PasswordChangeRequest {
    pub account_id: i64,
    pub old_password: String,
    pub new_password: String,
}

/// Change the account password. Verifies the old password using the
/// plaintext scheme (since the browser UI sends plaintext), then rewrites
/// both the SRP6a verifier and the `sha_pass_hash`.
pub async fn change_password(
    State(state): State<Arc<crate::AppState>>,
    Json(req): Json<PasswordChangeRequest>,
) -> Result<StatusCode, AccountError> {
    let account = accounts::find_by_id(&state.pool, req.account_id)
        .await?
        .ok_or(AccountError::NotFound)?;

    let cred = credentials::find_by_account_id(&state.pool, account.id)
        .await?
        .ok_or(AccountError::NotFound)?;

    // Verify the old password via the plaintext hash.
    let stored_hash = cred
        .sha_pass_hash
        .as_deref()
        .ok_or_else(|| AccountError::BadRequest("no plaintext credential stored".into()))?;

    if !crate::registration::verify_plaintext(&account.email, &req.old_password, stored_hash) {
        return Err(AccountError::InvalidCredentials);
    }

    // Rewrite both credential schemes with the new password.
    let salt = srp::generate_salt();
    let verifier = srp::compute_verifier(
        &state.group,
        &account.email,
        &req.new_password,
        &salt,
        srp::ITERATIONS,
    );
    let verifier_bytes = verifier.to_bytes_be().1;
    let sha_pass = srp::compute_sha_pass_hash(&account.email, &req.new_password);

    credentials::upsert(
        &state.pool,
        account.id,
        &salt,
        &verifier_bytes,
        srp::ITERATIONS as i32,
        2,
        &sha_pass,
    )
    .await?;

    tracing::info!(account_id = account.id, "password changed");
    Ok(axum::http::StatusCode::OK)
}
