#!/usr/bin/env python3
# SPDX-License-Identifier: AGPL-3.0-only
#
# Tavern SRP login load-test harness.
#
# Replays a configurable login burst against the account-server and reports
# throughput, latency percentiles, and error rates. Uses the existing SRP
# handshake in dev/srp_auth_client.py.
#
# Usage:
#   python3 load-test/login-harness.py --server http://127.0.0.1:8081 \
#       --concurrency 50 --count 200
#
#   python3 load-test/login-harness.py --server http://127.0.0.1:8081 \
#       --concurrency 10,50,100,200 --count 500 --json
#
# Prerequisites:
#   A running account-server with seeded test accounts (docs/test-accounts.sql).

import argparse
import concurrent.futures
import json
import math
import os
import sys
import time
from dataclasses import dataclass, field

# Import the existing SRP handshake from the dev directory.
sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", "dev"))
import srp_auth_client  # noqa: E402

# ---------------------------------------------------------------------------
# Test accounts — seeded by docs/test-accounts.sql
# ---------------------------------------------------------------------------
TEST_ACCOUNTS = [
    ("test@example.com", "password"),
    ("admin@bnet.local", "123456"),
    ("player@example.org", "Correct Horse Battery Staple"),
    ("user@example.com", "MixedCasePassword"),
    ("guest@bnet.local", "p@ssw0rd!#$%^&*()"),
    ("numeric@example.com", "01234567890123456789"),
    ("longpass@example.com", "Q" * 128),
]
# These accounts fail SRP because they have edge-case passwords that the
# seeded verifier may not handle identically. Exclude them from load tests:
#   ("a@b.cd", "x")
#   ("empty@example.com", "")
#   ("utf8@example.com", "üñîçøé")


@dataclass
class LoginResult:
    ok: bool
    duration_ms: float
    error: str | None = None


@dataclass
class RunReport:
    concurrency: int
    count: int
    ok: int
    fail: int
    duration_ms: float
    latencies_ms: list[float] = field(default_factory=list)

    @property
    def throughput_rps(self) -> float:
        return self.count / (self.duration_ms / 1000) if self.duration_ms > 0 else 0.0

    def percentile(self, p: float) -> float:
        if not self.latencies_ms:
            return 0.0
        return _percentile(sorted(self.latencies_ms), p)

    def summary(self) -> str:
        if not self.latencies_ms:
            p50 = p95 = p99 = 0.0
        else:
            s = sorted(self.latencies_ms)
            p50 = _percentile(s, 50)
            p95 = _percentile(s, 95)
            p99 = _percentile(s, 99)
        return (
            f"concurrency={self.concurrency:>4}  "
            f"ok={self.ok}/{self.count}  "
            f"fail={self.fail}  "
            f"duration={self.duration_ms:.1f}s  "
            f"rps={self.throughput_rps:.1f}  "
            f"p50={p50:.1f}ms  p95={p95:.1f}ms  p99={p99:.1f}ms"
        )


def _percentile(sorted_data: list[float], p: float) -> float:
    if not sorted_data:
        return 0.0
    k = (p / 100.0) * (len(sorted_data) - 1)
    f = math.floor(k)
    c = math.ceil(k)
    if f == c:
        return sorted_data[int(k)]
    d0 = sorted_data[int(f)] * (c - k)
    d1 = sorted_data[int(c)] * (k - f)
    return d0 + d1


def login_once(server: str, email: str, password: str) -> LoginResult:  # noqa: D103
    start = time.perf_counter()
    try:
        _, result = srp_auth_client.perform_handshake(
            email=email,
            password=password,
            server_url=server,
        )
        duration = (time.perf_counter() - start) * 1000.0
        if result.get("login_ticket"):
            return LoginResult(ok=True, duration_ms=duration)
        return LoginResult(ok=False, duration_ms=duration, error="no login ticket")
    except Exception as e:
        duration = (time.perf_counter() - start) * 1000.0
        return LoginResult(ok=False, duration_ms=duration, error=str(e))


def _build_accounts() -> list[tuple[str, str]]:
    """Expand the account list with seeded load-test accounts if present."""
    accounts = list(TEST_ACCOUNTS)
    # Auto-detect seeded lt*@loadtest.local accounts via env or SQL.
    # For simplicity, just expand from an env variable or default count.
    lt_count = int(os.environ.get("LOADTEST_ACCOUNTS", "0"))
    for i in range(lt_count):
        accounts.append((f"lt{i}@loadtest.local", "loadtest"))
    return accounts


def run_one(server: str, concurrency: int, count: int) -> RunReport:
    """Run *count* logins with *concurrency* worker threads."""
    all_accounts = _build_accounts()
    # Assign each worker a unique slice to avoid SRP session collisions.
    per_worker = -(count // -concurrency)  # ceiling division
    results: list[LoginResult] = []

    start = time.perf_counter()
    with concurrent.futures.ThreadPoolExecutor(max_workers=concurrency) as pool:
        futs = []
        for w in range(concurrency):
            base = (w * per_worker) % len(all_accounts)
            for i in range(per_worker):
                email, pw = all_accounts[(base + i) % len(all_accounts)]
                futs.append(pool.submit(login_once, server, email, pw))
        for future in concurrent.futures.as_completed(futs):
            results.append(future.result())
    wall = (time.perf_counter() - start) * 1000.0

    ok = sum(1 for r in results if r.ok)
    fail = len(results) - ok
    latencies = [r.duration_ms for r in results if r.ok]
    return RunReport(
        concurrency=concurrency,
        count=len(results),
        ok=ok,
        fail=fail,
        duration_ms=wall,
        latencies_ms=latencies,
    )


def main() -> int:  # noqa: D103
    p = argparse.ArgumentParser(
        prog="login-harness",
        description="Tavern SRP login load-test harness",
    )
    p.add_argument(
        "--server",
        "-s",
        required=True,
        help="Account-server base URL (e.g. http://127.0.0.1:8081)",
    )
    p.add_argument(
        "--concurrency",
        "-c",
        help="Comma-separated concurrency levels (default: 10,50,100)",
    )
    p.add_argument(
        "--count",
        "-n",
        type=int,
        default=200,
        help="Total login attempts per concurrency level (default: 200)",
    )
    p.add_argument("--json", action="store_true", help="Emit machine-readable JSON")
    args = p.parse_args()

    if args.concurrency:
        levels = [int(x.strip()) for x in args.concurrency.split(",")]
    else:
        levels = [10, 50, 100]

    reports = []
    for level in levels:
        print(f"--- concurrency={level}, count={args.count} ---", file=sys.stderr)
        report = run_one(args.server, level, args.count)
        print(report.summary(), file=sys.stderr)
        reports.append(report)

        # Brief cooldown between runs so the server's SRP-session maps stabilize.
        if level != levels[-1]:
            time.sleep(2)

    if args.json:
        payload = []
        for r in reports:
            s = sorted(r.latencies_ms) if r.latencies_ms else []
            payload.append(
                {
                    "concurrency": r.concurrency,
                    "count": r.count,
                    "ok": r.ok,
                    "fail": r.fail,
                    "duration_ms": round(r.duration_ms, 2),
                    "throughput_rps": round(r.throughput_rps, 1),
                    "latency_p50_ms": round(_percentile(s, 50), 2) if s else None,
                    "latency_p95_ms": round(_percentile(s, 95), 2) if s else None,
                    "latency_p99_ms": round(_percentile(s, 99), 2) if s else None,
                }
            )
        json.dump(payload, sys.stdout, indent=2)
        sys.stdout.write("\n")

    total_fail = sum(r.fail for r in reports)
    return 1 if total_fail > 0 else 0


if __name__ == "__main__":
    raise SystemExit(main())
