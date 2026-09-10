-- SPDX-License-Identifier: AGPL-3.0-only

-- Dual credential support: the Vanilla plaintext line (sha_pass_hash) alongside
-- the SRP6a verifier. A registered account stores both so any client line can
-- log in against the same account.
ALTER TABLE credentials ADD COLUMN IF NOT EXISTS sha_pass_hash TEXT;

-- Game accounts (WoW account per Battle.net account). Registration creates a
-- default game account; the bnetserver /gameAccounts/ endpoint (M8) lists them.
CREATE TABLE IF NOT EXISTS game_accounts (
    id          BIGSERIAL    PRIMARY KEY,
    account_id  BIGINT       NOT NULL REFERENCES accounts (id) ON DELETE CASCADE,
    name        TEXT         NOT NULL,  -- e.g. "1#1"
    region      SMALLINT     NOT NULL DEFAULT 0,
    created_at  TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    UNIQUE (account_id, name)
);
