-- SPDX-License-Identifier: AGPL-3.0-only

-- Connected third-party accounts matching /api/account-connections format.
-- Providers: apple, nintendo, twitch, discord, facebook, ubisoft, google, psn, steam, live.

CREATE TABLE IF NOT EXISTS account_connections (
    account_id BIGINT NOT NULL REFERENCES accounts (id) ON DELETE CASCADE,
    provider TEXT NOT NULL,
    provider_user_id TEXT,
    connected_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (account_id, provider)
);
