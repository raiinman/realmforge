// SPDX-License-Identifier: AGPL-3.0-only

//! Battle.net web-login SRP6a (`BnetSRP6v2<SHA256>`), server side.
//!
//! This is a pure-Rust implementation of the modern Battle.net password proof,
//! matching the reference implementation's `BnetSRP6v2` byte-for-byte (see
//! `docs/srp.md`). It is distinct from the legacy 256-bit/SHA-1 realm-auth
//! SRP used by 3.3.5a-era clients.
//!
//! The server stores `(salt, verifier, iterations, version)` and never the
//! password. On login it generates a private ephemeral `b`, publishes
//! `B = (g^b + v*k) mod N`, then verifies the client's evidence `M1` against
//! its own recomputation.
//!
//! # References
//!
//! - The reference implementation's `SRP6.{h,cpp}` (`BnetSRP6v2<SHA256>`).
//! - The bnetserver `LoginRESTService` `/bnetserver/login/srp/` challenge.
//! - Captured `account.battle.net` SRP login flow.

pub mod encoding;
mod groups;
mod kdf;

pub use encoding::srp_username;
pub use groups::Group;
pub use kdf::ITERATIONS;

use num_bigint::BigInt;
use num_traits::Zero;
use rand::Rng;
use sha2::{Digest, Sha256};

pub(crate) use encoding::{FIELD_WIDTH, pad_be_fixed, signed_be, unsigned_be};

/// A fresh random salt (32 bytes).
pub fn generate_salt() -> [u8; 32] {
    rand::random()
}

/// Compute the SRP6a verifier `v = g^x mod N` for registration.
///
/// `login` is the account email. The password is used as supplied (BnetSRP6v2
/// is case-sensitive). The returned verifier is stored alongside the salt and
/// iteration count; it is the only credential material the server keeps.
pub fn compute_verifier(
    group: &Group,
    login: &str,
    password: &str,
    salt: &[u8],
    iterations: u32,
) -> BigInt {
    let srp_user = srp_username(login);
    let n_minus_one = &group.n - BigInt::from(1u32);
    let x = kdf::compute_x(&srp_user, password, salt, iterations, &n_minus_one);
    group.g.modpow(&x, &group.n)
}

/// Compute the plaintext-line credential hash used by 1.13/1.14 Vanilla
/// clients (`sha_pass_hash`): `SHA256(hex(SHA256(UPPER(login))) + ":" +
/// UPPER(password))`. This is NOT SRP — it is the plaintext-over-TLS credential
/// proof the Vanilla bnetserver verifies directly.
pub fn compute_sha_pass_hash(login: &str, password: &str) -> String {
    let email_hash = {
        let mut h = Sha256::new();
        h.update(encoding::upper_latin(login).as_bytes());
        hex::encode(h.finalize())
    };
    let mut h = Sha256::new();
    h.update(email_hash.as_bytes());
    h.update(b":");
    h.update(encoding::upper_latin(password).as_bytes());
    hex::encode(h.finalize())
}

/// Compute the client-side SRP proof: `(publicA, clientEvidenceM1)`.
///
/// Given the server challenge parameters (`public_b`, `salt`, `iterations`)
/// and a client private ephemeral `a`, returns the client's public ephemeral
/// `A` and the evidence `M1` the server verifies. This mirrors what the
/// browser's `srp6a-routines.worker.js` computes; it exists so integration
/// tests can simulate a client.
pub fn client_proof(
    group: &Group,
    login: &str,
    password: &str,
    salt: &[u8],
    iterations: u32,
    public_b: &BigInt,
    client_a: &BigInt,
) -> (BigInt, BigInt) {
    let n = &group.n;
    let n_minus_one = n - BigInt::from(1u32);
    let srp_user = srp_username(login);
    let x = kdf::compute_x(&srp_user, password, salt, iterations, &n_minus_one);

    let big_a = group.g.modpow(client_a, n);
    let k = multiplier(group);
    let u = scrambling_parameter(group, &big_a, public_b);
    let g_x = group.g.modpow(&x, n);

    let mut base = public_b - &k * &g_x;
    base %= n;
    if base.sign() == num_bigint::Sign::Minus {
        base += n;
    }

    let exp = client_a + &u * &x;
    let s = base.modpow(&exp, n);

    let mut hasher = Sha256::new();
    hasher.update(signed_be(&big_a));
    hasher.update(signed_be(public_b));
    hasher.update(signed_be(&s));
    let m1 = unsigned_be(&hasher.finalize());

    (big_a, m1)
}

/// An SRP6a server session, single-use.
///
/// Construct it with the stored verifier to obtain the server public ephemeral
/// [`ServerSession::public_b`], then call [`ServerSession::verify`] with the
/// client's `A` and `M1`. A session verifies at most once.
pub struct ServerSession {
    group: Group,
    v: BigInt,
    b: BigInt,
    pub_b: BigInt,
    used: bool,
}

impl ServerSession {
    /// Create a session with a caller-supplied private ephemeral `b`.
    ///
    /// For production logins prefer [`ServerSession::new`], which draws `b`
    /// from an RNG. This constructor exists for deterministic golden-vector
    /// tests.
    pub fn with_private_b(group: &Group, verifier: &BigInt, b: BigInt) -> Self {
        let n_minus_one = &group.n - BigInt::from(1u32);
        let k = multiplier(group);
        // B = (g^b + v*k) mod N
        let g_b = group.g.modpow(&b, &group.n);
        let pub_b = (g_b + &k * verifier) % &group.n;
        let _ = n_minus_one; // b is assumed already reduced by the caller
        Self {
            group: group.clone(),
            v: verifier.clone(),
            b,
            pub_b,
            used: false,
        }
    }

    /// Create a session with a cryptographically random private ephemeral `b`,
    /// uniform in `[0, N - 2]`.
    pub fn new(group: &Group, verifier: &BigInt, rng: &mut impl Rng) -> Self {
        let n_minus_one = &group.n - BigInt::from(1u32);
        let b = gen_below(rng, &n_minus_one);
        Self::with_private_b(group, verifier, b)
    }

    /// The server public ephemeral `B`, to send in the `/login/srp` challenge.
    pub fn public_b(&self) -> &BigInt {
        &self.pub_b
    }

    /// Verify the client's evidence `M1` against `A`. On success returns the
    /// shared secret `S`; on failure (bad proof, `A mod N == 0`, `u mod N ==
    /// 0`, or a reused session) returns `None`.
    pub fn verify(&mut self, a: &BigInt, client_m1: &BigInt) -> Option<BigInt> {
        if self.used {
            return None;
        }
        self.used = true;

        let n = &self.group.n;

        // Reject degenerate values.
        if (a % n).is_zero() {
            return None;
        }
        let u = scrambling_parameter(&self.group, a, &self.pub_b);
        if (&u % n).is_zero() {
            return None;
        }

        // S = (A * v^u)^b mod N
        let v_u = self.v.modpow(&u, n);
        let base = (a * v_u) % n;
        let s = base.modpow(&self.b, n);

        // M1 = SHA256( signed_be(A) || signed_be(B) || signed_be(S) )
        let mut hasher = Sha256::new();
        hasher.update(signed_be(a));
        hasher.update(signed_be(&self.pub_b));
        hasher.update(signed_be(&s));
        let our_m1 = unsigned_be(&hasher.finalize());

        if &our_m1 == client_m1 { Some(s) } else { None }
    }

    /// Compute the server evidence `M2 = SHA256( signed_be(A) || signed_be(M1)
    /// || signed_be(S) )`, returned to the client so it can authenticate the
    /// server.
    pub fn server_evidence(a: &BigInt, m1: &BigInt, s: &BigInt) -> BigInt {
        let mut hasher = Sha256::new();
        hasher.update(signed_be(a));
        hasher.update(signed_be(m1));
        hasher.update(signed_be(s));
        unsigned_be(&hasher.finalize())
    }
}

/// `k = SHA256( pad256(N) || pad256(g) )`, the SRP-6a multiplier.
fn multiplier(group: &Group) -> BigInt {
    let mut hasher = Sha256::new();
    hasher.update(pad_be_fixed(&group.n, FIELD_WIDTH));
    hasher.update(pad_be_fixed(&group.g, FIELD_WIDTH));
    unsigned_be(&hasher.finalize())
}

/// `u = SHA256( pad256(A) || pad256(B) )`, the scrambling parameter.
fn scrambling_parameter(group: &Group, a: &BigInt, b: &BigInt) -> BigInt {
    let _ = group;
    let mut hasher = Sha256::new();
    hasher.update(pad_be_fixed(a, FIELD_WIDTH));
    hasher.update(pad_be_fixed(b, FIELD_WIDTH));
    unsigned_be(&hasher.finalize())
}

/// Uniform random BigInt in `[0, bound)` via rejection sampling on random
/// bytes. Avoids the `RandBigInt` trait so the build stays hermetic.
fn gen_below(rng: &mut impl Rng, bound: &BigInt) -> BigInt {
    use num_bigint::Sign;
    let bits = bound.bits() as usize;
    let byte_len = bits.div_ceil(8);
    loop {
        let mut bytes = vec![0u8; byte_len];
        rng.fill_bytes(&mut bytes);
        let candidate = BigInt::from_bytes_be(Sign::Plus, &bytes);
        if candidate < *bound {
            return candidate;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const LOGIN: &str = "player@example.com";
    const PASSWORD: &str = "correct horse battery staple";
    const ITER: u32 = ITERATIONS;

    fn group() -> Group {
        Group::battle_net_v2()
    }

    /// Client-side SRP computation (an independent code path from the server).
    /// Used by the round-trip test to produce `A`, `S`, and `M1`.
    struct Client {
        group: Group,
        a: BigInt,
        big_a: BigInt,
    }

    impl Client {
        fn new(group: &Group, a: BigInt) -> Self {
            let big_a = group.g.modpow(&a, &group.n);
            Self {
                group: group.clone(),
                a,
                big_a,
            }
        }

        fn derive(
            &self,
            password: &str,
            salt: &[u8],
            public_b: &BigInt,
        ) -> (BigInt, BigInt, BigInt) {
            let n = &self.group.n;
            let n_minus_one = n - BigInt::from(1u32);

            let srp_user = srp_username(LOGIN);
            let x = kdf::compute_x(&srp_user, password, salt, ITER, &n_minus_one);

            let k = {
                let mut h = Sha256::new();
                h.update(pad_be_fixed(n, FIELD_WIDTH));
                h.update(pad_be_fixed(&self.group.g, FIELD_WIDTH));
                unsigned_be(&h.finalize())
            };
            let u = {
                let mut h = Sha256::new();
                h.update(pad_be_fixed(&self.big_a, FIELD_WIDTH));
                h.update(pad_be_fixed(public_b, FIELD_WIDTH));
                unsigned_be(&h.finalize())
            };

            // S = (B - k * g^x) ^ (a + u * x) mod N
            let g_x = self.group.g.modpow(&x, n);
            let mut base = public_b - (&k * &g_x);
            base %= n;
            if base.sign() == num_bigint::Sign::Minus {
                base += n;
            }
            let exp = &self.a + &u * &x;
            let s = base.modpow(&exp, n);

            // M1 = SHA256( signed_be(A) || signed_be(B) || signed_be(S) )
            let mut h = Sha256::new();
            h.update(signed_be(&self.big_a));
            h.update(signed_be(public_b));
            h.update(signed_be(&s));
            let m1 = unsigned_be(&h.finalize());
            (self.big_a.clone(), m1, s)
        }
    }

    #[test]
    fn register_then_login_succeeds() {
        let group = group();
        let salt = generate_salt();
        let v = compute_verifier(&group, LOGIN, PASSWORD, &salt, ITER);

        // Fixed private values for determinism in this test.
        let mut session = ServerSession::with_private_b(&group, &v, BigInt::from(123_456_789u32));
        let public_b = session.public_b().clone();

        let client = Client::new(&group, BigInt::from(987_654_321u32));
        let (big_a, m1, client_s) = client.derive(PASSWORD, &salt, &public_b);

        let server_s = session
            .verify(&big_a, &m1)
            .expect("correct password must verify");

        // The client and server independently arrive at the same shared
        // secret: the core SRP invariant.
        assert_eq!(server_s, client_s, "shared secrets must agree");
        assert!(!server_s.is_zero());
    }

    #[test]
    fn wrong_password_is_rejected() {
        let group = group();
        let salt = generate_salt();
        let v = compute_verifier(&group, LOGIN, PASSWORD, &salt, ITER);

        let server_b = BigInt::from(42u32);
        let mut session = ServerSession::with_private_b(&group, &v, server_b);
        let public_b = session.public_b().clone();

        // Client derives a proof with the WRONG password.
        let client = Client::new(&group, BigInt::from(7u32));
        let (big_a, m1, _s) = client.derive("totally wrong password", &salt, &public_b);

        assert!(
            session.verify(&big_a, &m1).is_none(),
            "wrong password must not verify"
        );
    }

    #[test]
    fn degenerate_a_is_rejected() {
        let group = group();
        let salt = generate_salt();
        let v = compute_verifier(&group, LOGIN, PASSWORD, &salt, ITER);
        let mut session = ServerSession::with_private_b(&group, &v, BigInt::from(1u32));

        // A == 0 (mod N) and A == N must both be rejected.
        let m1 = BigInt::from(0u32);
        assert!(session.verify(&BigInt::from(0u32), &m1).is_none());
        // Note: session is single-use; after the first verify it refuses again.
        let mut session2 = ServerSession::with_private_b(&group, &v, BigInt::from(1u32));
        assert!(session2.verify(&group.n, &m1).is_none());
    }

    #[test]
    fn session_is_single_use() {
        let group = group();
        let salt = generate_salt();
        let v = compute_verifier(&group, LOGIN, PASSWORD, &salt, ITER);
        let server_b = BigInt::from(5u32);
        let client_a = BigInt::from(6u32);

        let mut session = ServerSession::with_private_b(&group, &v, server_b);
        let public_b = session.public_b().clone();
        let client = Client::new(&group, client_a);
        let (big_a, m1, _s) = client.derive(PASSWORD, &salt, &public_b);

        assert!(
            session.verify(&big_a, &m1).is_some(),
            "first verify succeeds"
        );
        // Replaying the same proof must fail: the session is consumed.
        assert!(
            session.verify(&big_a, &m1).is_none(),
            "a session must verify at most once"
        );
    }

    #[test]
    fn random_b_round_trip() {
        // Same as the success path but with a cryptographically random b.
        let group = group();
        let salt = generate_salt();
        let v = compute_verifier(&group, LOGIN, PASSWORD, &salt, ITER);

        let mut rng = rand::thread_rng();
        let mut session = ServerSession::new(&group, &v, &mut rng);
        let public_b = session.public_b().clone();

        let client = Client::new(
            &group,
            gen_below(&mut rng, &(&group.n - BigInt::from(1u32))),
        );
        let (big_a, m1, _s) = client.derive(PASSWORD, &salt, &public_b);
        assert!(session.verify(&big_a, &m1).is_some());
    }

    /// Cross-language golden vector: Rust (RustCrypto) must agree with an
    /// independent Python reimplementation (OpenSSL via `hashlib`) and the
    /// documented the reference implementation algorithm, for identical fixed inputs. Values
    /// generated by `dev/srp_oracle.py`.
    #[test]
    fn golden_vector_matches_python_oracle() {
        let group = group();

        let salt: [u8; 32] = core::array::from_fn(|i| i as u8);
        let server_b = BigInt::parse_bytes(b"1111111111111111", 16).unwrap();
        let client_a = BigInt::parse_bytes(b"2222222222222222", 16).unwrap();

        let expected_v = BigInt::parse_bytes(
            b"3c38f67d9bdb5916aae790a2d4875b53ef715923e9a35fa641ec28c040b8c0cc0ae815d0e24795376f6f7d67b2df6db5950919f0450f8d330d38ce1d7fb7934e20d493c311866ec8353536e2284d074025b8b1758139bf49bf5d8879f9c7ad7c620f519e0d3ccadb04ffbbfa1b5a48729b1d3a11d53dc301e5b4843aba74c5a7ca6ef6597155778b473b25b998d940aed00cc9c88d0b3c3102ad343353ae2fab4af3f146e719a883d7a42d7130e4234c1eac60d7677160c496d141597a6f085ab554135c6c1ca81403c04e1a0fe355701106c19ad28761b9aab98d906569fbbc5e419b82efd5752e1325e329cc797b16368889cb20713eaf6e25e45a645aec04",
            16,
        ).unwrap();
        let expected_b = BigInt::parse_bytes(
            b"90bb2b3f0ce08f4ec68effd1771ec813b9dd71f56f3f00f414839802c26ef308a2ba7b0f8766a49cc24c20881fb8d8d58fec036c57851f7c9fab3852c0559a832a39ac23242345bc15560b58259377bf2f4c4ba3c242c2c4339b3d30235428356c182f7d1756090261037d62057da3c51f07125037c9af4ac84366bdf56da7ce072339acd4edb780f2787676323460df6b81a2f6dec2d0beca662e2db61aaf0d05c52efdaa49686296a7410b4b1ac41024c5f775b85cc6353fb6e8e88312f95861d30a30736587b9bac07c7ec58255a1b365f4bae9223ddc4a34360f4118255dccfe6f9210a1554be6fa487361656ca815628616450a27e31f67eed8dfc8015b",
            16,
        ).unwrap();
        let expected_a = BigInt::parse_bytes(
            b"65846fe086cfd42fb66ab61aab9b64197e12a573ee72c85faedbf11f0936067783d53bbe4a50324a400fa41f21afc8b2dd163451afb698468e8a441f32f8f3846f927749a7cba3dec69a104b2822b5a2580738acbc02942018b2bf1f1dac28d825b155bd201aa6928a50c5b736633ce6763aa30728c0637facc404d15ad518f7dff4c1cfa530ce829f04a4b276cf5980c88d7f5f1182c2accb79bfd8c3d2f089a57efb5789a8fec0c6382fb5213ef3b42ded5e87acc36fef01889600c8ab649e31fea37568342a8c0e5251e4f707f52e6dc2d22c570d1c690189dd464004b672ba8f0c8d38638cb15ea1d1a2929af89ce64e1a409b2877ef7aa3795b11f0f0bf",
            16,
        ).unwrap();
        let expected_s = BigInt::parse_bytes(
            b"2a70667fbe0f8cee11115788ce2e16abcf719f1ed180487846020d75ef20fb78775fcc2dd02aeaea00a353030950fdeb84b1439b1eef286c7018a0ee04e83e403fc693b25a3bc32825f807d5046024e60e1cd1872563746b7e5020cbb64b6d524af48e14129fa3db115511b02b74027f81caf5f84a41161a83129e6a94a1a420e688b1eee22715680f313d1a4cc74d41b52b656581613c0958bb3daaadd547ab660605865151e35f6298bb7c76adfe3a52f4eee48a2e17e038555cceebb752cef6d47e81b69bb79e3366e49fdd6ee9184b7371609e29f6f35b56de8b96f24ce06b5043b0653918a814cfe40e0a8c5b3a2b76f56c4f4374f2a53e3bf28a687fca",
            16,
        ).unwrap();
        let expected_m1 = BigInt::parse_bytes(
            b"5d4dbc75be4c0d069e34d27e571d92e750bef524772838db5190161e31969ac2",
            16,
        )
        .unwrap();

        // Verifier.
        let v = compute_verifier(&group, LOGIN, PASSWORD, &salt, ITER);
        assert_eq!(v, expected_v, "verifier must match the Python oracle");

        // Server public B.
        let mut session = ServerSession::with_private_b(&group, &v, server_b);
        assert_eq!(session.public_b(), &expected_b, "B must match the oracle");

        // Client A and M1 via the independent client path.
        let client = Client::new(&group, client_a);
        let (big_a, m1, client_s) = client.derive(PASSWORD, &salt, session.public_b());
        assert_eq!(big_a, expected_a, "A must match the oracle");
        assert_eq!(m1, expected_m1, "M1 must match the oracle");

        // Server verifies and yields the same shared secret.
        let server_s = session
            .verify(&big_a, &m1)
            .expect("golden M1 must verify on the server");
        assert_eq!(server_s, expected_s, "S must match the oracle");
        assert_eq!(server_s, client_s, "client and server shared secrets agree");
    }

    /// The authoritative reference vectors: the reference implementation's `srp_reference` tool
    /// (the actual `BnetSRP6v2` implementation) emits verifiers for a fixed
    /// set of (email, password) pairs with a fixed all-zero salt. Our
    /// `compute_verifier` must reproduce every `v2/SHA256` verifier byte-for-
    /// byte. Generated by `build/tools-min/.../srp_reference --json` on the
    /// the reference implementation `wotlk_classic` branch.
    #[test]
    fn trinitycore_reference_verifiers_match() {
        // Fixed all-zero salt used by `srp_reference`.
        let salt = [0u8; 32];
        let group = group();

        let cases: &[(&str, &str, &str)] = &[
            (
                "test@example.com",
                "password",
                "14F97166C11B7FEC10B0B9632DFD3ABF0A8098F1980B6A599616FBB0F274DCC8462CD7EB55931BDF5E87B488E4E3920878FCC9E5404814185A2C1E233A5F1E855E7A33C33BD13CBDC95D83C45B848232DCDCA15DFC03C42AF74EDBC2AA7E39DD2198608C8DB20BBDA303144F311999D5149020ECBBE5373A05B3CED82B4CDF25DFD6EAE161EE109D5FE6A01DF595CB1641A0551F6B3A57D23AFF719A868D80091FDE259E0E5B8E3819409D86D5F42AE2ECEE7B7C8F782F45E15A3F75879676CA02B96AE48FDD3C6CE1E7077E653F3EB71307E23DA9114D93C08218A81C3D7A4AB354857D89A5226C4C9C63F119918EBD77D0858E6F824E2DB16DF0EC0D00BB89",
            ),
            (
                "admin@bnet.local",
                "123456",
                "C4E987B13F2CEDA8FAB8E55E9217E1E93B52650B71E6C914E629632E5133D4F864F063B3BBB329494F84C3F185EFF17CD66D5B234A7CFE539478460753EE6467F27B000290A03858B6052AAE8B089CE456C040AA32786DFCA4A62A69B754F8FEDF72285B1862C0ED1603020DA64EA0C4D9766EB2A99259F6C805E93BE2853CB30AEA840C79A85009115651B179DC496C864848484A83B437E2AB33C6E5A9A22D6215AFCF37EB6E23AE21644319423D82CAACF70CF3926444902D28891F11CDCAB6E36050142F520D406F6A46F665E4067EC3584CA20AFDCD550F511A479075B5E53B19FF83F117364896B4277A9939EF982ACD3024D990E55F8176A3716F7468",
            ),
            (
                "player@example.org",
                "Correct Horse Battery Staple",
                "6BFB325644B5B998FFF46B49BAC68CF581B5F52BCF4F2D61250CDF17E4A6FB21831B94B3F06CE8C3781F37A26AD96FADBB3D8B9D9489A7ED2D168C6D772094935FB5A431DA1A17C5A9FE053A4199DD581C1F4CED5E796807D86047796D5BEB9F0B1D12111637E19C5C5CD8F23F5D7C1768388B76615079991161FC00F18DEF0B1781F3240CE57AF6C01C369253DE4EC5010055F55E0A43C9EA6C87FC94D81930BE1A60A5383D737F4FE74E8E309B0FB833D9FAF518BD3DAE5D77B3E65F27550224709555ECE134239CFFE6D22E389124C398939C9795ED35D2903860FF46EB4B303F2E22425233DC2D5F164C76713972E2294968C40990168E1CC6C2E2AA48A5",
            ),
            (
                "user@example.com",
                "MixedCasePassword",
                "092FC0A38F5D442B527CFDACE951B223A893F4A569DA59592F8FF203CC4CD770BA8B7F317775E42FC19B4B2E77798F17DC22A6C025D93769BD438F11E9B34F4015E332474BB112D1BF4A31D1F273BE93E08A52A0F6F53F4D30EA63152D51DE8A0E03322F50DEDBEFF7F1002867C8F8A1E4781D24EBB58465C3D4B47F4C13395087F38039F3819CBFF36FEBD4C7E88BBDCE4D7D6AD89E4B53813AC859C9FA68595BD8DD8BCFAD19C49FB1A7A87AEA2DB2DCA6CF61C0F1D1335766C9B0D7DE6828A207366F2F3629EB561AA015122F2323854B71671109EE9F721802583691775267778A63DFF36BDDF2FF1C64CAC1C7F28C8F6E9A064494F0E31C79A3732C3C6A",
            ),
            (
                "guest@bnet.local",
                "p@ssw0rd!#$%^&*()",
                "1969F799BC8DC352FC3684B831206700CF696FAE84FE7F835B21BAE141C84EED28B79222B8D9A8720A8986ED3DBC2F51F5E5EFC2D564A29EA88E14F2C4EBBBCD7FA84ABE3A63134066E4A2AB7A35E0FF659D3EC076DA6935DABB02697DA95024273B97780DEB4E426C3A3553E6B30E9B2FF77D7306FFFB88335F81947127E4D491AA73DC8690E910B0DCC3C71117C545595D672EE1C1FF20EF0C03F8A616363129785286E772237ED0F0A6842C972E3591531779B8A041893870CDCF2E708F4879D9EE011A1AF50AF49AAACB86E97E8635315B33A441B0AC7982279A2947F19C3969E2B63DB31F8F821D5B2C97405D4AD0CA6F0112AEF6A1D542CAB71DC90298",
            ),
            (
                "a@b.cd",
                "x",
                "D463701E824D1CD89EE2244DAD4CF5EACCDF9A7113D49CCC1AD9351D141604594CD819B92B91F65005E518EA11C689BF9C008CBD6400B4B546F4E408C5D83AD7320A46AA181BB84529FED378AE1AA30E3A5FE05345D64AE226FC683A2C41E5F81C905C159D6054092FAD775BE6E4BB8A89EDC6675E123AC82A15B6859ECBA8FEDD8D314C092892D024F2A0A44289246CEA0B372BAC4132C4898747F14AD846953BEFC3C62B789D47E64C0EDAC245F99BB7E49D9BAE827C39E6792D5E30E51414F4A5B106724E570724324F4A9804A397D51CF6B54F30D27796B540B77E1F674604BC8FE2B82797EA6330A954DE1324F10434900134C0218DB8A510C573E3A83E",
            ),
            (
                "empty@example.com",
                "",
                "821A0328B6A11B8AF7FBA310FC53E8570DBF743A07D6EAFC930049269B1CC1DDB839B241E65C7FA0BCBFFB17A242667D8B92AF89A802C0629A2E198CCC281977257FE8EE3F69810CE8703AC981F53511C5BBDFD68D0E34F701EAD8AA4D051BD37A838BBB9CF1F556641D789A029A69A2BDFDE05A5D2F3D8932D59068703202A2C9C9A94BE42A647ABBA47E3961A575BDAB539FAB9264FDB98B83131DC38E58EB20E89FA944F61F192DD323F7F67D086CF0DF81FCC28109B86F46043ABC1F49442B87649A711130F5289AF9325B60BDDDB0EC2A0288035627ACDA220200AFACCED8F671577DBCB41A5BD4311789E5AC102FFCE485D781454EBD026860A6C7F32E",
            ),
            (
                "numeric@example.com",
                "01234567890123456789",
                "CB521F96716D00FA7A230F19DB824073A8C0AA98E781F1F0291C498BE062723C4DE331553A6744E0C6AB42AB6491C99E0BE496AB2AB5562569E25403F7C70EDBE06DD8D51824DFC77A3480508FDEBFA3BF8AFA90B261AFCB2FE5E6E5E8932EF0FFC99381F6C0EEE0614DE0373486F6B5A297A9D89BF0C30CE4C64E8394ADE58D2A8B8D511991F1BF938B427CB6588376C0E298CE291470514B606BE7180906E5C362925063C751135634DA61E839812320E7EF7D20A94941979AA9161CE1AB28E17F3A17C164B40A599DD694436CAD5CB3213EC75CBB2C1B7D29495A081E91825DBA3E5C2DB23E8AF055C42E0D90CBF814F00D4648A2D93DBDF005323701033C",
            ),
            (
                "utf8@example.com",
                "üñîçøé",
                "A4CBE03321C86DB6D10410EAE3BFB9092D119AE55B5C94F441FBD045A203A9BA4098BE13122FFAC10239BA60E33362650B2E8F14E83102C5F6A5FAF4271CBD73ADF59E91CD108D78193E80209389A927B650211648EF1C334D395005938CB2490FF98A3E48EF3A663EB8655A12D88C34180D608FFCAC3170BB0F64122C53667923A6F1B41C75A3CFAB2DC7E78B324BEA386D1FB9008C2F69AEBD53E3EEE4A7BA793FF004F0FA606DC25F301748BC104687FF350FBC556E44F3EC73F4FD6EEE7AC4336F3FDE71FE3DCE46F907161D9672689D7707B9FF5C8D452980204B0EBEA9B83CF4A7E873427D890D1A32622EB0951CBB45AF1FC9D1602994BD66281FDC30",
            ),
            (
                "longpass@example.com",
                "QQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQ",
                "041873B32871E76471864D102550631E4187710EC9EE01D80C99EB1D1AA0A3DF4A820BAE70A7DA17E23D52A198A3CF3D2055D6CE3016618E0971ACF1FE6684FAC54E9858832245E6CD5B6DF168718562EA8325E6C3F622A71F8B49985028676D71A07DD5664B9A47C635FC705F10DF4FC0C9BD0A9B2F3EB724AF69DCADAE554958EBE7CD9590CEB919017A589CA673180057C88B27081875F358D73DF13209AD7BC50929E40E643B982EED95441F27E654389B70190BF2C8D749352AE52228A33AD62CED79885913CB21C559200DEE7F6D07A53E24923EE37B8E5B9B23577CC14868BE388EADF1CCB2E8A3C7942048F733270F3260C5809FEFC7D2C0B4858F88",
            ),
        ];

        for (email, password, expected_le_hex) in cases {
            let v = compute_verifier(&group, email, password, &salt, ITER);
            // The reference tool emits the verifier little-endian (the
            // reference Python reproduction reverses to little-endian, exact
            // length). Reverse to big-endian to parse as
            // the same integer.
            let expected = parse_le_hex(expected_le_hex);
            assert_eq!(
                v, expected,
                "the reference implementation reference verifier mismatch for email={email:?}"
            );
        }
    }

    /// Parse a little-endian hex string (as emitted by the the reference implementation reference
    /// tools) into a big-endian [`BigInt`].
    fn parse_le_hex(le_hex: &str) -> BigInt {
        let mut bytes = hex::decode(le_hex).expect("valid reference hex");
        bytes.reverse();
        BigInt::from_bytes_be(num_bigint::Sign::Plus, &bytes)
    }
}
