# Test Data

> Full walkthrough of the testing process (fresh DB, web UI, OAuth,
> simulated clients): [Testing End to End](testing.md).

## Quick Start

```bash
# Reset and seed the database
bash dev/db.sh reset
DATABASE_URL=postgres://realmforge:realmforge@localhost:5432/realmforge \
  cargo sqlx migrate run --source crates/realmforge-gate-db/migrations
podman exec -i realmforge-gate-db psql -U realmforge -d realmforge < docs/test-accounts.sql

# Seed BGS service tickets (required for VerifyWebCredentials)
podman exec realmforge-gate-db psql -U realmforge -d realmforge << 'SQL'
INSERT INTO service_tickets (st, account_id, region, expires_at)
SELECT 'BTEST-' || generate_series, 1001, 1, NOW() + interval '24 hours'
FROM generate_series(1, 50);
SQL

# Seed load-test accounts (required for SRP load testing)
uv run python load-test/seed-loadtest-accounts.py --count 200 | \
  podman exec -i realmforge-gate-db psql -U realmforge -d realmforge
```

## Test Accounts

Reference accounts in `docs/test-accounts.sql`:

| Email | Password | Account ID |
|---|---|---|
| `test@example.com` | `password` | 1001 |
| `admin@bnet.local` | `123456` | 1002 |
| `player@example.org` | `Correct Horse Battery Staple` | 1003 |

All use fixed salt (32 zero bytes) and 15000 PBKDF2 iterations for SRP v2.

## Service Tickets

Service tickets bridge the web SRP login to BGS `VerifyWebCredentials`.
The BGS test client uses `BTEST-{N}` format tickets (account 1001).
Seed 50 fresh tickets before running BGS auth tests.

## Licenses

The `account_licenses` table stores game license grants (e.g. WoW = 1,
WoW Classic = 2). `GetLicenses` and `GetAccountState` return these.
`docs/test-accounts.sql` seeds licenses for accounts 1001–1010.
`seed-loadtest-accounts.py` seeds license 1 (WoW) for load-test accounts.

## Load-Test Accounts

`load-test/seed-loadtest-accounts.py --count N` generates accounts
`lt{0..N-1}@loadtest.local` with password `loadtest`. These share the
same fixed salt as the reference accounts.

## Running Tests

```bash
# Start all servers
DATABASE_URL=postgres://realmforge:realmforge@localhost:5432/realmforge \
  cargo run -p realmforge-gate-bgs-server &
DATABASE_URL=postgres://realmforge:realmforge@localhost:5432/realmforge \
  cargo run -p realmforge-gate-account-server &
DATABASE_URL=postgres://realmforge:realmforge@localhost:5432/realmforge \
  cargo run -p realmforge-gate-oauth-server &

# BGS transport test
uv run python load-test/bgs-client.py --flow logon

# BGS full auth cycle
uv run python load-test/bgs-client.py --flow full

# BGS session restore
uv run python load-test/bgs-client.py --flow restore --sso-id <HEX>

# BGS queue test
MAX_BGS_LOGINS=1 QUEUE_CLIENTS=3 QUEUE_HOLD_SECS=3 \
  uv run python load-test/bgs-queue-test.py

# BGS benchmark
MAX_BGS_LOGINS=5000 uv run python load-test/bgs-queue-test.py \
  --count 200 --concurrency 50 --hold-secs 0

# SRP login test
python3 dev/srp_auth_client.py --server http://127.0.0.1:8081 \
  --email test@example.com --password password

# SRP load test
LOADTEST_ACCOUNTS=200 uv run python load-test/login-harness.py \
  --server http://127.0.0.1:8081 --concurrency 25,50,100 --count 200
```
