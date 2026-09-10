// SPDX-License-Identifier: AGPL-3.0-only

//! SRP6a group parameters for the Battle.net web-login variant
//! (`BnetSRP6v2<SHA256>`).
//!
//! The 2048-bit prime and generator are the exact values hardcoded in the
//! reference implementation's `BnetSRP6v2Base` and served by the real
//! `account.battle.net` `/login/srp` endpoint. Tavern uses them verbatim so it
//! interoperates with real game clients.

use num_bigint::BigInt;

/// The Battle.net `BnetSRP6v2` SRP group: a 2048-bit prime and generator 2.
#[derive(Debug, Clone)]
pub struct Group {
    /// Modulus N.
    pub n: BigInt,
    /// Generator g.
    pub g: BigInt,
}

impl Group {
    /// The canonical Battle.net `BnetSRP6v2<SHA256>` group.
    ///
    /// N is the 2048-bit prime `AC6BDB41...73` (see `N_HEX`); g is 2.
    pub fn battle_net_v2() -> Self {
        Self {
            n: BigInt::parse_bytes(N_HEX.as_bytes(), 16).expect("valid 2048-bit prime"),
            g: BigInt::from(2u32),
        }
    }
}

/// The 2048-bit SRP modulus used by Battle.net web login (BnetSRP6v2).
///
/// Copied verbatim from the reference implementation `BnetSRP6v2Base::N`.
const N_HEX: &str = "\
AC6BDB41324A9A9BF166DE5E1389582FAF72B6651987EE07FC3192943DB56050\
A37329CBB4A099ED8193E0757767A13DD52312AB4B03310DCD7F48A9DA04FD50\
E8083969EDB767B0CF6095179A163AB3661A05FBD5FAAAE82918A9962F0B93B8\
55F97993EC975EEAA80D740ADBF4FF747359D041D5C33EA71D281E446B14773B\
CA97B43A23FB801676BD207A436C6481F1D2B9078717461A5B9D32E688F87748\
544523B524B0D57D5EA77A2775D2ECFA032CFBDBF52FB3786160279004E57AE6\
AF874E7303CE53299CCC041C7BC308D82A5698F3A8D0C38271AE35F8E9DBFBB6\
94B5C803D89F7AE435DE236D525F54759B65E372FCD68EF20FA7111F9E4AFF73";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn modulus_is_2048_bits() {
        let group = Group::battle_net_v2();
        // A 2048-bit value has 256 bytes and the top bit set.
        let bytes = group.n.to_bytes_be().1;
        assert_eq!(bytes.len(), 256, "N must be exactly 256 bytes");
        assert!(bytes[0] & 0x80 != 0, "N must have its top bit set");
    }

    #[test]
    fn generator_is_two() {
        assert_eq!(Group::battle_net_v2().g, BigInt::from(2u32));
    }
}
