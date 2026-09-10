-- SPDX-License-Identifier: AGPL-3.0-only

-- Bind licenses to game accounts.
--
-- Legacy Battle.net sub-accounts (pre-Battle.net WoW accounts such as
-- WoW1/WoW2) carry their own license grants. The table supports two
-- scopes:
--   * account-level:  game_account_id IS NULL (fallback for all game
--     accounts of the account)
--   * game-account:   game_account_id set (precedence over fallback)
--
-- The original PRIMARY KEY (account_id, license_id) cannot hold both a
-- NULL-scope and a game-account-scope row for the same license, so it is
-- replaced by two partial unique indexes.

ALTER TABLE account_licenses ADD COLUMN IF NOT EXISTS game_account_id BIGINT;

-- Drop the old composite PK; recreate the account-level uniqueness as a
-- partial index (one account-level row per account+license).
ALTER TABLE account_licenses DROP CONSTRAINT IF EXISTS account_licenses_pkey;

CREATE UNIQUE INDEX IF NOT EXISTS account_licenses_account_uniq
  ON account_licenses (account_id, license_id)
  WHERE game_account_id IS NULL;

-- Per-game-account grants: one row per (account, game_account, license).
CREATE UNIQUE INDEX IF NOT EXISTS account_licenses_game_account_uniq
  ON account_licenses (account_id, game_account_id, license_id)
  WHERE game_account_id IS NOT NULL;

DO $$ BEGIN
  IF NOT EXISTS (
    SELECT 1 FROM pg_constraint WHERE conname = 'fk_account_licenses_game_account'
  ) THEN
    ALTER TABLE account_licenses
      ADD CONSTRAINT fk_account_licenses_game_account
        FOREIGN KEY (game_account_id) REFERENCES game_accounts (id) ON DELETE CASCADE;
  END IF;
END $$;
