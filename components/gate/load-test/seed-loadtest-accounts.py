#!/usr/bin/env python3
# SPDX-License-Identifier: AGPL-3.0-only
#
# Seed additional accounts for the load-test harness.
# Uses the same fixed salt (32 zero bytes) and SRP v2 verifiers as the
# existing test-accounts.sql seed.
#
# Usage:
#   python3 load-test/seed-loadtest-accounts.py --count 50 | podman exec -i tavern-db psql -U tavern -d tavern  # noqa: E501

import argparse
import os
import sys

sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", "dev"))
import srp_auth_client  # noqa: E402

FIXED_SALT = bytes(32)  # 32 zero bytes, matching test-accounts.sql
PASSWORD = "loadtest"
BASE_ID = 9000  # Start IDs well above the reference accounts (1001-1010)


def compute_verifier(email: str, password: str) -> str:
    """Compute the BnetSRP6v2 verifier hex for (email, password) with the fixed salt."""
    username = srp_auth_client.srp_username(email)
    x = srp_auth_client.compute_x_v2(username, password, FIXED_SALT)
    v = pow(srp_auth_client.G_V2, x, srp_auth_client.N_V2)
    return srp_auth_client.to_bignum_hex(v)


def generate_sql(count: int) -> list[str]:
    """Generate ON CONFLICT-safe INSERT statements for *count* accounts."""
    stmts = []
    for i in range(count):
        account_id = BASE_ID + i
        email = f"lt{i}@loadtest.local"
        battletag = f"LoadTester#{account_id}"
        verifier = compute_verifier(email, PASSWORD)
        stmts.append(
            f"INSERT INTO accounts (id, email, country_code, battletag)\n"
            f"VALUES ({account_id}, '{email}', 'THA', '{battletag}')\n"
            f"ON CONFLICT (email) DO UPDATE SET id = {account_id};\n"
        )
        stmts.append(
            f"INSERT INTO credentials (account_id, srp_salt, srp_verifier, srp_iterations, srp_version)\n"  # noqa: E501
            f"VALUES ({account_id},\n"
            f"  decode('{FIXED_SALT.hex()}','hex'),\n"
            f"  decode('{verifier}','hex'),\n"
            f"  15000, 2)\n"
            f"ON CONFLICT (account_id) DO UPDATE SET\n"
            f"  srp_salt = EXCLUDED.srp_salt,\n"
            f"  srp_verifier = EXCLUDED.srp_verifier,\n"
            f"  srp_iterations = EXCLUDED.srp_iterations;\n"
        )
        stmts.append(
            f"INSERT INTO game_accounts (account_id, name)\n"
            f"VALUES ({account_id}, '{i}#1')\n"
            f"ON CONFLICT (account_id, name) DO NOTHING;\n"
        )
        stmts.append(
            f"INSERT INTO account_licenses (account_id, license_id, level)\n"
            f"VALUES ({account_id}, 1, 'exact')\n"
            f"ON CONFLICT (account_id, license_id) DO NOTHING;\n"
        )
        # Enable authenticator on the first load-test account.
        if i == 0:
            stmts.append(f"UPDATE accounts SET has_authenticator = TRUE WHERE id = {account_id};\n")
    return stmts


def main() -> int:
    p = argparse.ArgumentParser(
        prog="seed-loadtest-accounts",
        description="Generate SQL to seed load-test accounts with SRP verifiers",
    )
    p.add_argument("--count", "-n", type=int, required=True, help="Number of accounts to generate")
    p.add_argument(
        "--password",
        default=PASSWORD,
        help=f"Password for all generated accounts (default: {PASSWORD})",
    )
    args = p.parse_args()

    stmts = generate_sql(args.count)
    sys.stdout.write("\n".join(stmts))
    sys.stdout.write("\n")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
