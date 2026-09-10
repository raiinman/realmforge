#!/usr/bin/env python3
# SPDX-License-Identifier: AGPL-3.0-only
#
# srp_auth_client.py - pure-Python SRP6a authentication client for the TrinityCore
# bnetserver login REST API.
#
# End-to-end test tool that runs the full web-login handshake:
#   1. POST /bnetserver/login/srp/  →  retrieve SRP challenge (N, g, salt, B)
#   2. Compute client ephemeral A, session key S, evidence M1
#   3. POST /bnetserver/login/      →  submit proof, receive login ticket + M2
#   4. Verify server evidence M2
#
# Uses ONLY the standard library (hashlib, urllib, json). No TrinityCore headers,
# no OpenSSL bindings, no third-party packages.
#
# Requires a running bnetserver with LoginREST enabled (LoginREST.Port in config).
#
# Usage:
#   python3 contrib/srp_auth_client/srp_auth_client.py \
#       --server http://127.0.0.1:8081 \
#       --email user@example.com \
#       --password hunter2
#
#   python3 contrib/srp_auth_client/srp_auth_client.py \
#       --server https://bnetserver.example.com:8081 \
#       --no-verify-ssl \
#       --email admin@bnet.local \
#       --password 123456
#
# Reference: the TrinityCore BnetSRP6v2 reference implementation
#           (SRP6 crypto module and bnetserver LoginRESTService).
#           Verifier generation cross-checked against the C++ reference
#           tool and a pure-Python reproduction.

from __future__ import annotations

import argparse
import hashlib
import http.client
import json
import secrets
import ssl
import sys
import urllib.parse

# ---------------------------------------------------------------------------
# SRP group parameters (TrinityCore BnetSRP6v2 reference)
# ---------------------------------------------------------------------------

# BnetSRP6v1Base::N, BnetSRP6v1Base::g  (1024-bit, legacy srp_version = 1)
N_V1 = int(
    "86A7F6DEEB306CE519770FE37D556F29944132554DED0BD68205E27F3231FEF5"
    "A10108238A3150C59CAF7B0B6478691C13A6ACF5E1B5ADAFD4A943D4A21A142B"
    "800E8A55F8BFBAC700EB77A7235EE5A609E350EA9FC19F10D921C2FA832E4461"
    "B7125D38D254A0BE873DFC27858ACB3F8B9F258461E4373BC3A6C2A9634324AB",
    16,
)
G_V1 = 2

# BnetSRP6v2Base::N, BnetSRP6v2Base::g  (2048-bit, default srp_version = 2)
N_V2 = int(
    "AC6BDB41324A9A9BF166DE5E1389582FAF72B6651987EE07FC3192943DB56050"
    "A37329CBB4A099ED8193E0757767A13DD52312AB4B03310DCD7F48A9DA04FD50"
    "E8083969EDB767B0CF6095179A163AB3661A05FBD5FAAAE82918A9962F0B93B8"
    "55F97993EC975EEAA80D740ADBF4FF747359D041D5C33EA71D281E446B14773B"
    "CA97B43A23FB801676BD207A436C6481F1D2B9078717461A5B9D32E688F87748"
    "544523B524B0D57D5EA77A2775D2ECFA032CFBDBF52FB3786160279004E57AE6A"
    "F874E7303CE53299CCC041C7BC308D82A5698F3A8D0C38271AE35F8E9DBFBB69"
    "4B5C803D89F7AE435DE236D525F54759B65E372FCD68EF20FA7111F9E4AFF73",
    16,
)
G_V2 = 2

# v2 PBKDF2 iteration count (BnetSRP6v2Base::GetXIterations).
V2_ITERATIONS = 15000

# Padding lengths for k and u (ToByteArray<N>(false)).
# v1: 1024-bit N → 128 bytes.  v2: 2048-bit N → 256 bytes.
PAD_LEN_V1 = 128
PAD_LEN_V2 = 256


# ---------------------------------------------------------------------------
# Client-side SRP6a session
# ---------------------------------------------------------------------------


class SrpClientSession:
    """Holds the client-side ephemeral state for one login attempt."""

    def __init__(self) -> None:
        self.a: int = 0  # client private exponent
        self.A: int = 0  # client public ephemeral A = g^a mod N
        self.N: int = 0  # modulus from server challenge
        self.g: int = 0  # generator from server challenge
        self.salt: bytes = b""  # salt from server challenge
        self.B: int = 0  # server public ephemeral B
        self.srp_username: str = ""  # hex(SHA256(UpperLatin(email)))
        self.version: int = 0  # 1 or 2
        self.hash_name: str = "SHA-256"  # "SHA-256" or "SHA-512"

        # Computed during handshake
        self.k: int = 0  # multiplier k = SHA256(N_pad || g_pad)
        self.x: int = 0  # private key x
        self.u: int = 0  # scrambling parameter
        self.v: int = 0  # verifier = g^x mod N
        self.S: int = 0  # session key
        self.M1_hex: str = ""  # client evidence (hex, uppercase)
        self.M1_int: int = 0  # client evidence as integer (for M2)

    @property
    def pad_len(self) -> int:
        return PAD_LEN_V1 if self.version == 1 else PAD_LEN_V2


# ---------------------------------------------------------------------------
# Algorithm (mirrors Trinity::Crypto::SRP and LoginRESTService)
# ---------------------------------------------------------------------------


def upper_only_latin(text: str) -> str:
    """Utf8ToUpperOnlyLatin: uppercase basic-Latin a-z, leave all else unchanged."""
    return "".join(chr(ord(c) - 0x20) if "a" <= c <= "z" else c for c in text)


def srp_username(email: str) -> str:
    """GetSrpUsername: HEX( SHA256( UpperLatinOnly(email) ) ), uppercase, no separators."""
    return hashlib.sha256(upper_only_latin(email).encode("utf-8")).hexdigest().upper()


def to_bignum_hex(val: int) -> str:
    """BigNumber::AsHexStr: uppercase hex without 0x prefix, padded to even length."""
    if val == 0:
        return "0"
    h = hex(val)[2:].upper()
    return f"0{h}" if len(h) % 2 else h


def evidence_bytes(bn: int) -> bytes:
    """GetBrokenEvidenceVector: exact significant-byte-length, big-endian.

    Mirrors bn.ToByteVector(bytes, false) where:
      bytes = (bn.GetNumBits() + 8) >> 3  (C++ right-shift, equivalent to integer division)
    This is NOT the same as (bit_length + 7) // 8 when bit_length is a multiple of 8
    (the C++ formula gives one more byte when bits % 8 == 0, accounting for the
    possibility of a leading zero byte needed to avoid sign-extension).

    BigNumber::GetBytes → BN_bn2binpad.
    """
    if bn == 0:
        return b"\x00"
    nbytes = (bn.bit_length() + 8) >> 3
    return bn.to_bytes(nbytes, "big")


def pad_be(bn: int, length: int) -> bytes:
    """ToByteArray<N>(false): N bytes big-endian, zero-padded.

    Mirrors bn.ToByteArray<N>(false) where false = big-endian
    (BigNumber::GetBytes → BN_bn2binpad).
    """
    return bn.to_bytes(length, "big")


def _hash_func(hash_name: str):
    """Return hashlib constructor for the hash function name used by the server."""
    if hash_name == "SHA-512":
        return hashlib.sha512
    return hashlib.sha256


def compute_x_v1(username: str, password: str, salt: bytes) -> int:
    """BnetSRP6v1Base::CalculateX: x = int( SHA256(salt || SHA256(u:p)), little-endian )."""
    inner = hashlib.sha256((username + ":" + password).encode("utf-8")).digest()
    outer = hashlib.sha256(salt + inner).digest()
    return int.from_bytes(outer, "little")


def compute_x_v2(username: str, password: str, salt: bytes) -> int:
    """BnetSRP6v2Base::CalculateX: signed big-endian PBKDF2-SHA512 x, reduced mod (N-1)."""
    dk = hashlib.pbkdf2_hmac(
        "sha512",
        (username + ":" + password).encode("utf-8"),
        salt,
        V2_ITERATIONS,
        dklen=64,
    )
    x = int.from_bytes(dk, "big", signed=True)
    return x % (N_V2 - 1)


def compute_k(N: int, g: int, pad_len: int, hash_name: str) -> int:
    """k = H( N.ToByteArray<pad_len>(false) || g.ToByteArray<pad_len>(false) ).

    Both pad and final BigNumber construction are big-endian.
    """
    h = _hash_func(hash_name)
    data = pad_be(N, pad_len) + pad_be(g, pad_len)
    return int.from_bytes(h(data).digest(), "big")


def compute_u(A: int, B: int, pad_len: int, hash_name: str) -> int:
    """u = H( A.ToByteArray<pad_len>(false) || B.ToByteArray<pad_len>(false) ).

    Both pad and final BigNumber construction are big-endian.
    """
    h = _hash_func(hash_name)
    data = pad_be(A, pad_len) + pad_be(B, pad_len)
    return int.from_bytes(h(data).digest(), "big")


def compute_evidence(A: int, B: int, S: int, hash_name: str) -> int:
    """M1 = H( ev(A) || ev(B) || ev(S) ), returned as integer.

    Mirrors BnetSRP6Base::DoCalculateEvidence.
    """
    h = _hash_func(hash_name)
    data = evidence_bytes(A) + evidence_bytes(B) + evidence_bytes(S)
    return int.from_bytes(h(data).digest(), "big")


def compute_client_S(B: int, k: int, v: int, a: int, u: int, x: int, N: int) -> int:
    """Client-side session key: S = (B - k*v)^(a + u*x) mod N."""
    # Ensure non-negative base
    base = (B - k * v) % N
    exponent = a + u * x
    return pow(base, exponent, N)


def compute_client_M2(A: int, M1_int: int, S: int, hash_name: str) -> int:
    """M2 = H( ev(A) || ev(M1) || ev(S) ), returned as integer.

    Mirrors BnetSRP6Base::CalculateServerEvidence.
    """
    h = _hash_func(hash_name)
    data = evidence_bytes(A) + evidence_bytes(M1_int) + evidence_bytes(S)
    return int.from_bytes(h(data).digest(), "big")


# ---------------------------------------------------------------------------
# Client handshake
# ---------------------------------------------------------------------------


def generate_client_ephemeral(N: int, g: int) -> tuple[int, int]:
    """Generate random private a, compute public A = g^a mod N.

    Returns (a, A).
    """
    # a must be random mod (N-1), same bit-length as N
    nbits = N.bit_length()
    a = secrets.randbits(nbits) % (N - 1)
    # Ensure a > 0 (modular exponent with a=0 gives A=1, rejected by some servers)
    if a == 0:
        a = 1
    A = pow(g, a, N)
    return a, A


def perform_handshake(
    email: str,
    password: str,
    server_url: str,
    *,
    ssl_context: ssl.SSLContext | None = None,
    debug: bool = False,
    forced_version: int | None = None,
) -> tuple[SrpClientSession, dict[str, object]]:
    """Run the full SRP6a login handshake against a bnetserver.

    Uses a persistent HTTP connection so the server's per-connection
    session state carries the SRP6 object from the challenge phase
    to the login proof phase.

    If forced_version is set (1 or 2), the server's challenge version must
    match or SrpAuthError is raised. When None, the version is auto-detected.

    Returns (session, login_result_dict).
    Raises SrpAuthError on failure.
    """
    session = SrpClientSession()
    session.srp_username = srp_username(email)

    # Parse server URL to get host, port, and scheme
    parsed = urllib.parse.urlparse(server_url)
    host = parsed.hostname or "127.0.0.1"
    port = parsed.port or (443 if parsed.scheme == "https" else 80)
    use_tls = parsed.scheme == "https"

    # Create persistent HTTP connection
    if use_tls:
        ctx = ssl_context if ssl_context else ssl.create_default_context()
        conn = http.client.HTTPSConnection(host, port, context=ctx, timeout=30)
    else:
        conn = http.client.HTTPConnection(host, port, timeout=30)

    # Build the path prefix (e.g., "" or "/subdir")
    base_path = parsed.path.rstrip("/") if parsed.path else ""

    try:
        # ---- Step 1: POST /bnetserver/login/srp/ (challenge) ----
        challenge_path = f"{base_path}/bnetserver/login/srp/"
        challenge_body = json.dumps({"inputs": [{"input_id": "account_name", "value": email}]}).encode(
            "utf-8"
        )

        if debug:
            print(f"[DEBUG] POST {host}:{port}{challenge_path}", file=sys.stderr)
            print(f"[DEBUG] Request body: {challenge_body.decode()}", file=sys.stderr)

        conn.request(
            "POST",
            challenge_path,
            body=challenge_body,
            headers={"Content-Type": "application/json;charset=utf-8"},
        )
        challenge_resp = conn.getresponse().read().decode("utf-8")

        if debug:
            print(f"[DEBUG] Challenge response: {challenge_resp[:500]}", file=sys.stderr)

        challenge = json.loads(challenge_resp)

        if challenge.get("authentication_state") == "DONE":
            raise SrpAuthError(
                "Account not found or challenge rejected by server.",
                stage="challenge",
            )

        session.version = challenge["version"]
        if forced_version is not None and session.version != forced_version:
            raise SrpAuthError(
                f"Server returned SRP version {session.version} "
                f"but --version {forced_version} was requested.",
                stage="challenge",
            )

        session.N = int(challenge["modulus"], 16)
        session.g = int(challenge["generator"], 16)
        session.hash_name = challenge.get("hash_function", "SHA-256")
        session.salt = bytes.fromhex(challenge["salt"])
        session.B = int(challenge["public_B"], 16)

        if debug:
            print(f"[DEBUG] version       = {session.version}", file=sys.stderr)
            print(f"[DEBUG] hashFunction  = {session.hash_name}", file=sys.stderr)
            print(
                f"[DEBUG] N (hex)       = {to_bignum_hex(session.N)[:32]}...",
                file=sys.stderr,
            )
            print(f"[DEBUG] salt          = {challenge['salt']}", file=sys.stderr)
            print(
                f"[DEBUG] B (hex)       = {to_bignum_hex(session.B)[:32]}...",
                file=sys.stderr,
            )
            print(
                f"[DEBUG] srpUsername   = {challenge.get('username', '')}",
                file=sys.stderr,
            )

        # ---- Step 2: Compute client ephemeral and evidence ----
        while True:
            session.a, session.A = generate_client_ephemeral(session.N, session.g)
            if (session.A % session.N) != 0:
                break

        session.k = compute_k(session.N, session.g, session.pad_len, session.hash_name)
        session.u = compute_u(session.A, session.B, session.pad_len, session.hash_name)

        if (session.u % session.N) == 0:
            raise SrpAuthError(
                "Scrambling parameter u is 0 mod N (extremely unlikely). Regenerate A.",
                stage="compute",
            )

        if session.version == 1:
            pwd = upper_only_latin(password)
            session.x = compute_x_v1(session.srp_username, pwd, session.salt)
        else:
            session.x = compute_x_v2(session.srp_username, password, session.salt)

        session.v = pow(session.g, session.x, session.N)
        session.S = compute_client_S(
            session.B,
            session.k,
            session.v,
            session.a,
            session.u,
            session.x,
            session.N,
        )
        session.M1_int = compute_evidence(session.A, session.B, session.S, session.hash_name)
        session.M1_hex = to_bignum_hex(session.M1_int)

        if debug:
            print(f"[DEBUG] a  = {to_bignum_hex(session.a)}", file=sys.stderr)
            print(f"[DEBUG] A  = {to_bignum_hex(session.A)}", file=sys.stderr)
            print(f"[DEBUG] k  = {to_bignum_hex(session.k)}", file=sys.stderr)
            print(f"[DEBUG] u  = {to_bignum_hex(session.u)}", file=sys.stderr)
            print(f"[DEBUG] x  = {to_bignum_hex(session.x)}", file=sys.stderr)
            print(f"[DEBUG] v  = {to_bignum_hex(session.v)}", file=sys.stderr)
            print(f"[DEBUG] S  = {to_bignum_hex(session.S)}", file=sys.stderr)
            print(f"[DEBUG] M1 = {session.M1_hex}", file=sys.stderr)
            print(
                f"[DEBUG] ev(A) len={len(evidence_bytes(session.A))} = {evidence_bytes(session.A).hex().upper()[:40]}...",  # noqa: E501
                file=sys.stderr,
            )
            print(
                f"[DEBUG] ev(B) len={len(evidence_bytes(session.B))} = {evidence_bytes(session.B).hex().upper()[:40]}...",  # noqa: E501
                file=sys.stderr,
            )
            print(
                f"[DEBUG] ev(S) len={len(evidence_bytes(session.S))} = {evidence_bytes(session.S).hex().upper()[:40]}...",  # noqa: E501
                file=sys.stderr,
            )

        # ---- Step 3: POST /bnetserver/login/ (proof) ----
        login_path = f"{base_path}/bnetserver/login/"
        login_body = json.dumps(
            {
                "inputs": [
                    {"input_id": "account_name", "value": email},
                    {"input_id": "public_A", "value": to_bignum_hex(session.A)},
                    {"input_id": "client_evidence_M1", "value": session.M1_hex},
                ]
            }
        ).encode("utf-8")

        if debug:
            print(f"[DEBUG] POST {host}:{port}{login_path}", file=sys.stderr)
            print(f"[DEBUG] Request body: {login_body.decode()}", file=sys.stderr)

        conn.request(
            "POST",
            login_path,
            body=login_body,
            headers={"Content-Type": "application/json;charset=utf-8"},
        )
        login_response = conn.getresponse()
        login_cookies = login_response.getheader("Set-Cookie") or ""
        login_resp = login_response.read().decode("utf-8")

        if debug:
            print(f"[DEBUG] Login response: {login_resp}", file=sys.stderr)
            if login_cookies:
                print(f"[DEBUG] Set-Cookie: {login_cookies}", file=sys.stderr)

        login_result = json.loads(login_resp)

        # Check for authentication failure
        state = login_result.get("authentication_state", "")

        # ---- Step 3b: Handle authenticator challenge ----
        if state == "AUTHENTICATOR":
            next_url = login_result.get("next_url", "")
            if not next_url:
                raise SrpAuthError(
                    "Server requested authenticator but no next_url provided.",
                    stage="authenticator",
                )
            auth_parsed = urllib.parse.urlparse(next_url)
            auth_path = auth_parsed.path
            if auth_parsed.query:
                auth_path += "?" + auth_parsed.query

            code = "12345678"
            auth_body = json.dumps(
                {
                    "version": "1.0",
                    "program_id": "WoW",
                    "platform_id": "Wn64",
                    "inputs": [
                        {"input_id": "authenticator_input", "value": code},
                        {"input_id": "remember_authenticator", "value": "true"},
                    ],
                }
            ).encode("utf-8")

            auth_headers = {"Content-Type": "application/json;charset=utf-8"}
            if login_cookies:
                auth_headers["Cookie"] = login_cookies

            if debug:
                print(f"[DEBUG] POST {auth_path} (authenticator)", file=sys.stderr)
            conn.request("POST", auth_path, body=auth_body, headers=auth_headers)
            auth_resp = conn.getresponse().read().decode("utf-8")
            if debug:
                print(f"[DEBUG] Authenticator response: {auth_resp}", file=sys.stderr)
            login_result = json.loads(auth_resp)
            state = login_result.get("authentication_state", "")

        if state == "DONE" and "login_ticket" not in login_result:
            raise SrpAuthError(
                f"Authentication failed. Server returned state={state}, "
                f"error_code={login_result.get('error_code', 'none')}",
                stage="login",
            )

        # ---- Step 4: Verify server evidence M2 ----
        server_m2_hex = login_result.get("server_evidence_M2", "")
        login_ticket = login_result.get("login_ticket", "")

        if server_m2_hex:
            expected_m2_int = compute_client_M2(
                session.A,
                session.M1_int,
                session.S,
                session.hash_name,
            )
            expected_m2_hex = to_bignum_hex(expected_m2_int)

            if debug:
                print(f"[DEBUG] Server M2  = {server_m2_hex}", file=sys.stderr)
                print(f"[DEBUG] Expected M2 = {expected_m2_hex}", file=sys.stderr)

            if expected_m2_int != int(server_m2_hex, 16):
                raise SrpAuthError(
                    f"Server evidence M2 mismatch! Got {server_m2_hex}, expected {expected_m2_hex}",
                    stage="verify_m2",
                )

        if not login_ticket:
            raise SrpAuthError(
                "No login ticket returned despite successful authentication.",
                stage="login",
            )

    except (ConnectionRefusedError, OSError, http.client.HTTPException) as e:
        raise SrpAuthError(
            f"HTTP connection error: {e}",
            stage="http",
        ) from e
    finally:
        conn.close()

    return session, login_result


# ---------------------------------------------------------------------------
# Error type
# ---------------------------------------------------------------------------


class SrpAuthError(Exception):
    """Raised when the SRP authentication handshake fails."""

    def __init__(self, message: str, stage: str = "unknown") -> None:
        super().__init__(message)
        self.stage = stage


# ---------------------------------------------------------------------------
# CLI
# ---------------------------------------------------------------------------


def build_arg_parser() -> argparse.ArgumentParser:
    p = argparse.ArgumentParser(
        prog="srp_auth_client.py",
        description="SRP6a authentication client for TrinityCore bnetserver login REST API",
    )
    p.add_argument(
        "--server",
        "-s",
        required=True,
        help="bnetserver base URL (e.g. http://127.0.0.1:8081)",
    )
    p.add_argument(
        "--email",
        "-e",
        required=True,
        help="Account email (login)",
    )
    p.add_argument(
        "--password",
        "-p",
        required=True,
        help="Account password",
    )
    p.add_argument(
        "--debug",
        "-d",
        action="store_true",
        help="Print verbose handshake details to stderr",
    )
    p.add_argument(
        "--no-verify-ssl",
        "-k",
        action="store_true",
        help="Skip TLS certificate verification (INSECURE: dev only)",
    )
    p.add_argument(
        "--json",
        action="store_true",
        help="Emit machine-readable JSON instead of human-readable summary",
    )
    p.add_argument(
        "--version",
        "-V",
        type=int,
        choices=[1, 2],
        default=None,
        help="Force SRP version (1 or 2). If not set, auto-detected from server challenge.",
    )
    return p


def main(argv: list[str]) -> int:
    args = build_arg_parser().parse_args(argv)

    ssl_context: ssl.SSLContext | None = None
    if args.server.startswith("https://"):
        if args.no_verify_ssl:
            ssl_context = ssl.create_default_context()
            ssl_context.check_hostname = False
            ssl_context.verify_mode = ssl.CERT_NONE
        else:
            ssl_context = ssl.create_default_context()

    try:
        session, login_result = perform_handshake(
            email=args.email,
            password=args.password,
            server_url=args.server,
            ssl_context=ssl_context,
            debug=args.debug,
            forced_version=args.version,
        )
    except SrpAuthError as e:
        if args.json:
            json.dump(
                {
                    "success": False,
                    "stage": e.stage,
                    "error": str(e),
                },
                sys.stdout,
                indent=2,
            )
            sys.stdout.write("\n")
        else:
            print(f"Authentication failed [{e.stage}]: {e}", file=sys.stderr)
        return 1
    except Exception as e:
        if args.json:
            json.dump(
                {
                    "success": False,
                    "stage": "exception",
                    "error": str(e),
                },
                sys.stdout,
                indent=2,
            )
            sys.stdout.write("\n")
        else:
            print(f"Unexpected error: {e}", file=sys.stderr)
        return 1

    if args.json:
        json.dump(
            {
                "success": True,
                "login_ticket": login_result.get("login_ticket", ""),
                "server_evidence_M2": login_result.get("server_evidence_M2", ""),
                "srp_username": session.srp_username,
                "version": session.version,
                "M1": session.M1_hex,
            },
            sys.stdout,
            indent=2,
        )
        sys.stdout.write("\n")
    else:
        print("Authentication successful!")
        print(f"  Login ticket: {login_result.get('login_ticket', 'N/A')}")
        print(f"  Server M2:    {login_result.get('server_evidence_M2', 'N/A')}")
        print(f"  srpUsername:  {session.srp_username}")
        print(f"  Version:      v{session.version}")
        print(f"  Client M1:    {session.M1_hex}")

    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
