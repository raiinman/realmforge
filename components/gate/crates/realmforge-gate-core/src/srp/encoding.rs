// SPDX-License-Identifier: AGPL-3.0-only

//! Byte encodings for the Battle.net `BnetSRP6v2` SRP variant.
//!
//! All encodings are big-endian (the reference implementation passes `littleEndian = false`
//! everywhere in `BnetSRP6`; the little-endian default is never used). Two
//! distinct paddings are in play:
//!
//! - `pad_be_fixed`: a fixed-width, zero-left-padded big-endian encoding,
//!   used for the `k` and `u` padded-pair hashes (width = modulus size).
//! - `signed_be`: the minimal two's-complement big-endian encoding with a
//!   leading `0x00` when the high bit is set. This is the "broken evidence
//!   vector" (`GetBrokenEvidenceVector` / jsbn `toByteArray`) used for the
//!   `M1`/`M2` evidence hashes.

use num_bigint::BigInt;
use num_traits::Signed;
use sha2::{Digest, Sha256};

/// Modulus byte width for the 2048-bit Battle.net group.
pub(crate) const FIELD_WIDTH: usize = 256;

/// Uppercase the ASCII (Latin) characters of `s`, leaving all other code points
/// unchanged.
///
/// Mirrors the reference implementation's `Utf8ToUpperOnlyLatin` for ASCII input. Battle.net
/// account logins are ASCII email addresses, so this is equivalent for the
/// inputs Realmforge handles.
pub(crate) fn upper_latin(s: &str) -> String {
    s.chars()
        .map(|c| {
            if c.is_ascii() {
                c.to_ascii_uppercase()
            } else {
                c
            }
        })
        .collect()
}

/// The SRP username identity: uppercase-hex SHA-256 of the uppercased login.
///
/// This is the 64-character string the server sends as the challenge `username`
/// field and feeds verbatim into the PBKDF2 input `"username:password"`. It is
/// uppercase hex because the reference implementation's `ByteArrayToHexStr` formats with
/// `{::02X}`.
pub fn srp_username(login: &str) -> String {
    let upper = upper_latin(login);
    let digest = Sha256::digest(upper.as_bytes());
    hex::encode_upper(digest)
}

/// Fixed-width big-endian encoding, zero-left-padded to `width` bytes.
///
/// Equivalent to the reference implementation `ToByteArray<width>(false)` / OpenSSL
/// `BN_bn2binpad`. Panics if the value needs more than `width` bytes (it must
/// be strictly less than the modulus).
pub(crate) fn pad_be_fixed(n: &BigInt, width: usize) -> Vec<u8> {
    let bytes = n.to_bytes_be().1;
    assert!(
        bytes.len() <= width,
        "value ({}) exceeds fixed width {width}",
        bytes.len()
    );
    let mut out = vec![0u8; width];
    out[width - bytes.len()..].copy_from_slice(&bytes);
    out
}

/// Minimal big-endian two's-complement encoding with a leading `0x00` when the
/// high bit is set (the jsbn `toByteArray` / `GetBrokenEvidenceVector` form).
///
/// For a non-negative value this prepends a single zero byte exactly when the
/// most significant byte has its high bit set, so the result stays
/// non-negative under signed interpretation. The length is therefore
/// `(num_bits + 8) / 8` bytes.
pub(crate) fn signed_be(n: &BigInt) -> Vec<u8> {
    // Only called on non-negative SRP field values (A, B, S, M1, K).
    debug_assert!(
        !n.is_negative(),
        "signed_be is only defined for non-negative values"
    );
    let mut bytes = n.to_bytes_be().1;
    if bytes.is_empty() {
        return vec![0];
    }
    if bytes[0] & 0x80 != 0 {
        let mut padded = Vec::with_capacity(bytes.len() + 1);
        padded.push(0x00);
        padded.extend_from_slice(&bytes);
        bytes = padded;
    }
    bytes
}

/// Interpret a digest as an unsigned big-endian integer.
pub(crate) fn unsigned_be(bytes: &[u8]) -> BigInt {
    BigInt::from_bytes_be(num_bigint::Sign::Plus, bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn srp_username_is_uppercase_hex_of_sha256_of_upper_login() {
        // login uppercased -> SHA-256 -> uppercase hex
        let u = srp_username("danielsreichenbach@tuta.com");
        assert_eq!(u.len(), 64);
        assert!(
            u.chars()
                .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit())
        );
        // Independent recompute.
        let mut h = Sha256::new();
        h.update(b"DANIELSREICHENBACH@TUTA.COM");
        let expect = hex::encode_upper(h.finalize());
        assert_eq!(u, expect);
    }

    #[test]
    fn pad_be_fixed_left_pads_to_width() {
        let two = BigInt::from(2u32);
        let bytes = pad_be_fixed(&two, FIELD_WIDTH);
        assert_eq!(bytes.len(), FIELD_WIDTH);
        assert!(bytes[..FIELD_WIDTH - 1].iter().all(|b| *b == 0));
        assert_eq!(bytes[FIELD_WIDTH - 1], 2);
    }

    #[test]
    fn signed_be_prepends_zero_when_high_bit_set() {
        // 0x80 needs a leading zero under signed interpretation.
        let v = BigInt::from(0x80u32);
        assert_eq!(signed_be(&v), vec![0x00, 0x80]);

        // 0x7f does not.
        let v = BigInt::from(0x7fu32);
        assert_eq!(signed_be(&v), vec![0x7f]);
    }
}
