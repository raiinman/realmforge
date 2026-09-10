-- SPDX-License-Identifier: AGPL-3.0-only

-- Device authorization codes (RFC 8628). Short-lived entries that bridge
-- the POST /device/code flow (device) with the interactive approval (browser)
-- and the POST /token polling (device).

CREATE TABLE IF NOT EXISTS device_authorizations (
    device_code TEXT PRIMARY KEY,
    user_code TEXT NOT NULL UNIQUE,
    client_id TEXT NOT NULL,
    account_id BIGINT,
    scope TEXT NOT NULL DEFAULT '',
    expires_at TIMESTAMPTZ NOT NULL,
    approved BOOL NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
