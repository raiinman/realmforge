#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-only
#
# Mint a fresh, unused BGS service ticket and print its name.
#
# Service tickets are single-use: VerifyWebCredentials marks them used,
# so every bgs-client.py run needs a new one. This script inserts a
# unique ticket and echoes the name for --ticket:
#
#   uv run python bgs-client.py --flow full --ticket "$(bash dev/ticket.sh 1001)"
#
# Usage: dev/ticket.sh [account_id] [region]   (defaults: 1001, 1)

set -euo pipefail

ACCOUNT_ID="${1:-1001}"
REGION="${2:-1}"
NAME="BTEST-$(date +%s)-$$"

podman exec -i realmforge-gate-db psql -U realmforge -d realmforge -v ON_ERROR_STOP=1 >/dev/null <<SQL
INSERT INTO service_tickets (st, account_id, region, expires_at)
VALUES ('$NAME', $ACCOUNT_ID, $REGION, NOW() + interval '24 hours');
SQL

echo "$NAME"
