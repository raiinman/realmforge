#!/usr/bin/env python3
# SPDX-License-Identifier: AGPL-3.0-only
"""Input fuzzer for Realmforge account server.

Hits every endpoint with malformed, oversized, and edge-case inputs to
surface panics and 500 errors. Run against a local dev server:

    python3 dev/fuzz.py --server http://127.0.0.1:8081
"""

from __future__ import annotations

import argparse
import contextlib
import hashlib
import http.client
import json
import random
import secrets
import sys
import time

FUZZ_COUNT = 200
TIMEOUT = 5

MALFORMED_JSON = [
    "",
    "{",
    "}",
    "[",
    "null",
    "undefined",
    "true",
    "false",
    '{"inputs": [',
    '{"inputs": null}',
    '{"inputs": [{}]}',
    '{"inputs": [{"input_id": null}]}',
    '{"inputs": [{"input_id": ""}]}',
    '{"inputs": [{"input_id": "' + "a" * 10000 + '"}]}',
    '{"inputs": [{"input_id": "account_name", "value": "' + "\x00" * 100 + '"}]}',
]

OVERSIZED_VALUES = [
    "a" * 100_000,
    "\x00" * 65_000,
    "\xff" * 10_000,
    b"{{}",
    b"\x00\x01\x02\x03" * 1000,
]


def _post(
    host: str,
    port: int,
    path: str,
    body: str | bytes,
    content_type: str = "application/json",
) -> tuple[int, str]:
    conn = http.client.HTTPConnection(host, port, timeout=TIMEOUT)
    try:
        if isinstance(body, str):
            body = body.encode("utf-8")
        conn.request(
            "POST",
            path,
            body=body,
            headers={"Content-Type": content_type, "Connection": "close"},
        )
        resp = conn.getresponse()
        status = resp.status
        body_text = resp.read().decode(errors="replace")
        return status, body_text[:500]
    except Exception as e:
        return 0, f"NETWORK_ERROR: {e}"
    finally:
        conn.close()


def _get(host: str, port: int, path: str) -> tuple[int, str]:
    conn = http.client.HTTPConnection(host, port, timeout=TIMEOUT)
    try:
        conn.request("GET", path, headers={"Connection": "close"})
        resp = conn.getresponse()
        return resp.status, resp.read().decode(errors="replace")[:500]
    except Exception as e:
        return 0, f"NETWORK_ERROR: {e}"
    finally:
        conn.close()


def fuzz_api(args) -> list[str]:
    failures = []
    host, port = args.server.split(":")[1].lstrip("/"), int(args.server.split(":")[2])
    endpoints = [
        "/api/",
        "/api/user",
        "/api/details",
        "/api/env",
        "/api/overview",
        "/api/security",
        "/api/wallet",
        "/api/privacy",
        "/api/games-and-subs",
        "/bnetserver/login/",
        "/bnetserver/login/srp/",
        "/login/en/",
        "/login/en/password",
        "/login/srp",
    ]

    print(
        f"[fuzz] {FUZZ_COUNT} iterations across {len(endpoints)} endpoints",
        file=sys.stderr,
    )

    for i in range(FUZZ_COUNT):
        path = random.choice(endpoints)
        method = random.choice(["GET", "POST"])

        if method == "GET":
            try:
                status, body = _get(host, port, path)
            except Exception as e:
                status, body = 0, f"EXCEPTION: {e}"

            if status == 0:
                failures.append(f"GET {path} → connection refused/crash")
        else:
            # POST: pick random payload
            payload_type = random.choice(["json", "raw", "empty", "oversized", "malformed"])
            if payload_type == "json":
                body = json.dumps(
                    {
                        "inputs": [
                            {"input_id": "account_name", "value": "test@example.com"},
                        ]
                    }
                )
                content_type = "application/json"
            elif payload_type == "raw":
                body = secrets.token_bytes(random.randint(0, 5000))
                content_type = "application/octet-stream"
            elif payload_type == "empty":
                body = ""
                content_type = "application/json"
            elif payload_type == "oversized":
                body = "x" * random.randint(50_000, 200_000)
                content_type = "text/plain"
            else:
                body = random.choice(MALFORMED_JSON)
                content_type = "application/json"

            try:
                status, body_resp = _post(host, port, path, body, content_type)
            except Exception as e:
                status, body_resp = 0, f"EXCEPTION: {e}"

            if status == 0:
                failures.append(f"POST {path} ({payload_type}) → connection refused/crash")
            elif status >= 500:
                failures.append(f"POST {path} ({payload_type}) → {status}: {body_resp[:200]}")

        if (i + 1) % 50 == 0:
            print(
                f"[fuzz] {i + 1}/{FUZZ_COUNT} iterations, {len(failures)} failures",
                file=sys.stderr,
            )

    return failures


def fuzz_srp(args) -> list[str]:
    """Fuzz the SRP login flow with malformed proofs."""
    failures = []
    host, port = args.server.split(":")[1].lstrip("/"), int(args.server.split(":")[2])

    valid_proof = None

    # First, get a valid challenge and proof for comparison
    try:
        conn = http.client.HTTPConnection(host, port, timeout=TIMEOUT)
        conn.request(
            "POST",
            "/bnetserver/login/srp/",
            json.dumps({"inputs": [{"input_id": "account_name", "value": "test@example.com"}]}),
            {"Content-Type": "application/json", "Connection": "close"},
        )
        chal = json.loads(conn.getresponse().read())
        conn.close()

        salt = bytes.fromhex(chal["salt"])
        N = int(chal["modulus"], 16)
        B = int.from_bytes(bytes.fromhex(chal["public_B"]), "big")

        dk = hashlib.pbkdf2_hmac("sha512", (chal["username"] + ":password").encode(), salt, 15000, dklen=64)
        x_raw = int.from_bytes(dk, "big")
        if x_raw >= (1 << 511):
            x_raw -= 1 << 512
        x = x_raw % (N - 1)
        a = int.from_bytes(secrets.token_bytes(32), "big")
        A = pow(2, a, N)
        n_bytes = N.to_bytes(256, "big")
        k = int.from_bytes(hashlib.sha256(n_bytes + (2).to_bytes(256, "big")).digest(), "big")
        u = int.from_bytes(
            hashlib.sha256(A.to_bytes(256, "big") + B.to_bytes(256, "big")).digest(),
            "big",
        )
        gx = pow(2, x, N)
        base = (B - k * gx) % N
        S = pow(base, a + u * x, N)

        def sb(v):
            bv = v.to_bytes((v.bit_length() + 7) // 8, "big")
            return b"\x00" + bv if bv[0] & 0x80 else bv

        m1 = hashlib.sha256(sb(A) + sb(B) + sb(S)).hexdigest().upper()
        pubA = hex(A)[2:].upper()

        valid_proof = {
            "public_A": pubA,
            "m1": m1,
        }
    except Exception:
        pass

    # Now try submitting valid proof first, then malicious ones
    malicious_proofs = []
    if valid_proof:
        # Valid
        malicious_proofs.append(valid_proof.copy())
        # Truncated
        if len(valid_proof["public_A"]) > 2:
            malicious_proofs.append({"public_A": valid_proof["public_A"][:-1], "m1": valid_proof["m1"]})
        # Invalid chars
        malicious_proofs.append({"public_A": "Z" * 512, "m1": valid_proof["m1"]})
        # Wrong M1
        malicious_proofs.append({"public_A": valid_proof["public_A"], "m1": "0" * 64})
        # Empty
        malicious_proofs.append({"public_A": "", "m1": ""})
        # Massive
        malicious_proofs.append({"public_A": "A" * 2000, "m1": "B" * 2000})
        # Negative values
        malicious_proofs.append({"public_A": "-" + valid_proof["public_A"][1:], "m1": valid_proof["m1"]})

    for proof in malicious_proofs:
        body = json.dumps(
            {
                "inputs": [
                    {"input_id": "account_name", "value": "test@example.com"},
                    {"input_id": "public_A", "value": proof["public_A"]},
                    {"input_id": "client_evidence_M1", "value": proof["m1"]},
                ]
            }
        )
        status, resp_body = _post(host, port, "/bnetserver/login/", body)
        if status == 0:
            failures.append("SRP proof fuzz → connection refused/crash")
        elif status >= 500:
            failures.append(f"SRP proof fuzz → {status}: {resp_body[:200]}")

    return failures


def fuzz_http_raw(args) -> list[str]:
    """Send raw garbage at HTTP level."""
    failures = []
    host, port = args.server.split(":")[1].lstrip("/"), int(args.server.split(":")[2])

    garbage_payloads = [
        b"\x00" * 4,
        b"GET /\x00HTTP/1.1\r\n\r\n",
        b"POST / HTTP/1.1\r\nContent-Length: -1\r\n\r\n",
        b"POST / HTTP/1.1\r\nContent-Length: 999999999\r\n\r\n",
        b"\xff" * 8192,
        b"GARBAGE\r\n\r\n",
        "GET /" + "\u0000" * 100 + " HTTP/1.1\r\n\r\n",
    ]

    import socket

    for payload in garbage_payloads:
        try:
            sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
            sock.settimeout(TIMEOUT)
            sock.connect((host, port))
            if isinstance(payload, str):
                payload = payload.encode("utf-8")
            sock.sendall(payload)
            with contextlib.suppress(OSError):
                sock.recv(4096)
            sock.close()
            time.sleep(0.05)  # small delay to let server recover
        except Exception as e:
            failures.append(f"raw HTTP {payload[:50]!r} → {e}")

    return failures


def main():
    parser = argparse.ArgumentParser(description="Realmforge account server fuzzer")
    parser.add_argument(
        "--server",
        default="http://127.0.0.1:8081",
        help="Server URL (default: http://127.0.0.1:8081)",
    )
    parser.add_argument("--all", action="store_true", help="Run all fuzzers")
    parser.add_argument("--api", action="store_true", help="API endpoint fuzzer")
    parser.add_argument("--srp", action="store_true", help="SRP login fuzzer")
    parser.add_argument("--raw", action="store_true", help="Raw HTTP fuzzer")
    args = parser.parse_args()

    run_all = args.all or not (args.api or args.srp or args.raw)

    failures = []

    if run_all or args.api:
        print("[fuzz] API endpoint fuzzer", file=sys.stderr)
        failures.extend(fuzz_api(args))

    if run_all or args.srp:
        print("[fuzz] SRP login fuzzer", file=sys.stderr)
        failures.extend(fuzz_srp(args))

    if run_all or args.raw:
        print("[fuzz] Raw HTTP fuzzer", file=sys.stderr)
        failures.extend(fuzz_http_raw(args))

    if failures:
        print(f"\n{failures} failures found:", file=sys.stderr)
        for f in failures:
            print(f"  {f}", file=sys.stderr)
        sys.exit(1)
    else:
        print("\nNo failures.", file=sys.stderr)
        sys.exit(0)


if __name__ == "__main__":
    main()
