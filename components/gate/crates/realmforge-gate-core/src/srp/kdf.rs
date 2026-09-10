// SPDX-License-Identifier: AGPL-3.0-only

//! Key derivation for the Battle.net `BnetSRP6v2` SRP variant.
//!
//! Implements the reference implementation `BnetSRP6v2Base::CalculateX`:
//! PBKDF2-HMAC-SHA-512 over `"username:password"` with the stored salt, 15000
//! iterations, 64 derived bytes; interpret big-endian as a signed
//! two's-complement integer (subtract `2^512` when the high bit is set);
//! reduce modulo `N - 1`.

use num_bigint::BigInt;
use num_traits::Signed;
use pbkdf2::pbkdf2_hmac;
use sha2::Sha512;

/// Default PBKDF2 iteration count for BnetSRP6v2.
pub const ITERATIONS: u32 = 15_000;

/// Length, in bytes, of the PBKDF2-HMAC-SHA-512 output.
const KEY_LEN: usize = 64;

/// Derive the private exponent `x` from the SRP username identity and password.
///
/// `srp_username` is the 64-character uppercase-hex identity produced by
/// [`crate::srp::srp_username`] (not the raw email). The password is used as
/// supplied (BnetSRP6v2 is case-sensitive). `n_minus_one` is `N - 1` for the
/// chosen group; the result is reduced to the range `[0, N - 2]`.
pub(crate) fn compute_x(
    srp_username: &str,
    password: &str,
    salt: &[u8],
    iterations: u32,
    n_minus_one: &BigInt,
) -> BigInt {
    let input = format!("{srp_username}:{password}");

    let mut dk = [0u8; KEY_LEN];
    pbkdf2_hmac::<Sha512>(input.as_bytes(), salt, iterations, &mut dk);

    // Big-endian two's-complement: from_signed_bytes_be treats the high bit of
    // the first byte as the sign, matching the reference implementation's `if (xBytes[0] &
    // 0x80) x -= 2^512`.
    let mut x = BigInt::from_signed_bytes_be(&dk);

    // Reduce modulo N-1 to a non-negative residue. num-bigint's `%` is
    // truncated (may be negative), so normalize.
    x %= n_minus_one;
    if x.is_negative() {
        x += n_minus_one;
    }
    x
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::srp::Group;

    #[test]
    fn compute_x_is_deterministic_and_nonneg() {
        let group = Group::battle_net_v2();
        let n_minus_one = &group.n - BigInt::from(1u32);
        let salt = [0u8; 32];
        let a = compute_x("ABCD", "pw", &salt, ITERATIONS, &n_minus_one);
        let b = compute_x("ABCD", "pw", &salt, ITERATIONS, &n_minus_one);
        assert_eq!(a, b, "compute_x must be deterministic");
        assert!(
            !a.is_negative(),
            "x must be reduced to a non-negative value"
        );
        assert!(a < n_minus_one, "x must be < N-1");
    }

    #[test]
    fn compute_x_differs_on_password() {
        let group = Group::battle_net_v2();
        let n_minus_one = &group.n - BigInt::from(1u32);
        let salt = [0u8; 32];
        let a = compute_x("U", "pw", &salt, ITERATIONS, &n_minus_one);
        let b = compute_x("U", "PW", &salt, ITERATIONS, &n_minus_one);
        assert_ne!(a, b, "v2 is case-sensitive");
    }
}
