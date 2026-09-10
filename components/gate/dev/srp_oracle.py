#!/usr/bin/env python3
# SPDX-License-Identifier: AGPL-3.0-only
"""Independent golden-vector oracle for Realmforge's BnetSRP6v2 implementation.

Reimplements the algorithm (from the TrinityCore reference) using Python's
stdlib (hashlib = OpenSSL, native int). Produces fixed-input values that the
Rust test cross-checks byte-for-byte. Run: python3 dev/srp_oracle.py
"""

import hashlib

N = int(
    "AC6BDB41324A9A9BF166DE5E1389582FAF72B6651987EE07FC3192943DB56050"
    "A37329CBB4A099ED8193E0757767A13DD52312AB4B03310DCD7F48A9DA04FD50"
    "E8083969EDB767B0CF6095179A163AB3661A05FBD5FAAAE82918A9962F0B93B8"
    "55F97993EC975EEAA80D740ADBF4FF747359D041D5C33EA71D281E446B14773B"
    "CA97B43A23FB801676BD207A436C6481F1D2B9078717461A5B9D32E688F87748"
    "544523B524B0D57D5EA77A2775D2ECFA032CFBDBF52FB3786160279004E57AE6"
    "AF874E7303CE53299CCC041C7BC308D82A5698F3A8D0C38271AE35F8E9DBFBB6"
    "94B5C803D89F7AE435DE236D525F54759B65E372FCD68EF20FA7111F9E4AFF73",
    16,
)
G = 2
ITER = 15000


def pad256(n):
    return n.to_bytes(256, "big")


def signed_be(n):
    size = max(1, (n.bit_length() + 8) // 8)
    return n.to_bytes(size, "big")


def srp_username(login):
    return hashlib.sha256(login.upper().encode()).hexdigest().upper()


def compute_x(login, password, salt):
    user = srp_username(login)
    dk = hashlib.pbkdf2_hmac("sha512", f"{user}:{password}".encode(), salt, ITER, 64)
    x = int.from_bytes(dk, "big", signed=True)  # two's-complement signed
    return x % (N - 1)


def sha256_int(*chunks):
    h = hashlib.sha256()
    for c in chunks:
        h.update(c)
    return int.from_bytes(h.digest(), "big")


def main():
    login = "player@example.com"
    password = "correct horse battery staple"
    salt = bytes(range(32))  # deterministic
    server_b = 0x1111111111111111
    client_a = 0x2222222222222222

    user = srp_username(login)
    x = compute_x(login, password, salt)
    v = pow(G, x, N)
    k = sha256_int(pad256(N), pad256(G))
    B = (pow(G, server_b, N) + v * k) % N
    A = pow(G, client_a, N)
    u = sha256_int(pad256(A), pad256(B))
    S_server = pow(A * pow(v, u, N) % N, server_b, N)
    S_client = pow((B - k * pow(G, x, N)) % N, client_a + u * x, N)
    assert S_server == S_client, "server/client shared secret mismatch"
    M1 = sha256_int(signed_be(A), signed_be(B), signed_be(S_server))

    print(f"SALT_HEX     = {salt.hex()}")
    print(f"SRP_USERNAME = {user}")
    print(f"VERIFIER     = {v}")
    print(f"PUBLIC_B     = {B}")
    print(f"PUBLIC_A     = {A}")
    print(f"SHARED_S     = {S_server}")
    print(f"M1           = {M1}")


if __name__ == "__main__":
    main()
