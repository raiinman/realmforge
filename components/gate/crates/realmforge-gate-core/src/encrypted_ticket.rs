// SPDX-License-Identifier: AGPL-3.0-only

//! Encrypted service tickets for email verification and other one-time
//! token flows. Uses AES-256-GCM with a key derived from the signing key.
//!
//! The ticket is an opaque, tamper-proof URL-safe string that encrypts an
//! `{account_id, expiry}` payload. Only the server (holder of the signing key)
//! can decrypt it. The ticket reveals nothing about the account.

use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Nonce};
use base64::Engine;
use serde::{Deserialize, Serialize};

/// Maximum ticket age (7 days).
const TICKET_TTL_SECS: i64 = 7 * 24 * 3600;

/// Payload encrypted inside a ticket.
#[derive(Serialize, Deserialize)]
struct TicketPayload {
    /// The account id this ticket authenticates.
    account_id: i64,
    /// Expiry as Unix timestamp.
    exp: i64,
}

/// Error type for ticket operations.
#[derive(Debug, thiserror::Error)]
pub enum TicketError {
    #[error("ticket has expired")]
    Expired,
    #[error("invalid ticket format: {0}")]
    Invalid(String),
}

/// Encrypt a one-time ticket for the given account.
///
/// `signing_key_pem` is the PKCS#8 PEM private key used to derive the
/// encryption key via SHA-256. The ticket expires after `TICKET_TTL_SECS`.
pub fn seal(account_id: i64, signing_key_pem: &str) -> Result<String, TicketError> {
    let key = derive_key(signing_key_pem);
    let cipher =
        Aes256Gcm::new_from_slice(&key).map_err(|e| TicketError::Invalid(e.to_string()))?;

    let payload = TicketPayload {
        account_id,
        exp: chrono::Utc::now().timestamp() + TICKET_TTL_SECS,
    };

    let plaintext =
        serde_json::to_vec(&payload).map_err(|e| TicketError::Invalid(e.to_string()))?;
    let nonce_bytes: [u8; 12] = rand::random();
    let nonce = Nonce::from_slice(&nonce_bytes);

    let ciphertext = cipher
        .encrypt(nonce, plaintext.as_ref())
        .map_err(|e| TicketError::Invalid(e.to_string()))?;

    // Format: nonce (12 bytes) || ciphertext
    let mut ticket_bytes = Vec::with_capacity(12 + ciphertext.len());
    ticket_bytes.extend_from_slice(&nonce_bytes);
    ticket_bytes.extend_from_slice(&ciphertext);

    Ok(base64_url(&ticket_bytes))
}

/// Decrypt a one-time ticket, returning the account id.
///
/// Returns `TicketError::Expired` if the ticket has passed its expiry.
/// Returns `TicketError::Invalid` for any decryption or format error.
pub fn open(ticket: &str, signing_key_pem: &str) -> Result<i64, TicketError> {
    let key = derive_key(signing_key_pem);
    let cipher =
        Aes256Gcm::new_from_slice(&key).map_err(|e| TicketError::Invalid(e.to_string()))?;

    let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(ticket)
        .map_err(|e| TicketError::Invalid(format!("base64: {e}")))?;

    if bytes.len() < 12 {
        return Err(TicketError::Invalid("ticket too short".into()));
    }

    let (nonce_bytes, ciphertext) = bytes.split_at(12);
    let nonce = Nonce::from_slice(nonce_bytes);

    let plaintext = cipher
        .decrypt(nonce, ciphertext)
        .map_err(|e| TicketError::Invalid(format!("decrypt: {e}")))?;

    let payload: TicketPayload = serde_json::from_slice(&plaintext)
        .map_err(|e| TicketError::Invalid(format!("json: {e}")))?;

    let now = chrono::Utc::now().timestamp();
    if payload.exp < now {
        return Err(TicketError::Expired);
    }

    Ok(payload.account_id)
}

/// Derive a 256-bit encryption key from the signing key's SHA-256 hash.
fn derive_key(signing_key_pem: &str) -> [u8; 32] {
    use sha2::Digest;
    let hash = sha2::Sha256::digest(signing_key_pem.as_bytes());
    let mut key = [0u8; 32];
    key.copy_from_slice(&hash);
    key
}

/// Base64url without padding.
fn base64_url(bytes: &[u8]) -> String {
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_KEY_PEM: &str = include_str!("../tests/data/test_key_pkcs8.pem");

    #[test]
    fn seal_open_round_trip() {
        let ticket = seal(42, TEST_KEY_PEM).unwrap();
        let account_id = open(&ticket, TEST_KEY_PEM).unwrap();
        assert_eq!(account_id, 42);
    }

    #[test]
    fn wrong_key_fails() {
        let ticket = seal(42, TEST_KEY_PEM).unwrap();
        // A different key should not decrypt.
        let other_key =
            "-----BEGIN PRIVATE KEY-----\nMIIEvQIBADANBgkqhkiG9w0BAQEFAASCBKcwggSjAgEAAoIBAQC7...";
        assert!(open(&ticket, other_key).is_err());
    }

    #[test]
    fn tampered_ticket_fails() {
        let mut ticket = seal(42, TEST_KEY_PEM).unwrap();
        // Flip the last character.
        let last = ticket.pop().unwrap();
        ticket.push(if last == 'A' { 'B' } else { 'A' });
        assert!(open(&ticket, TEST_KEY_PEM).is_err());
    }
}
