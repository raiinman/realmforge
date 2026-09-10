-- SPDX-License-Identifier: AGPL-3.0-only

-- Game account status fields for suspension/ban tracking and
-- subscription expiration.  Used by AccountService to report
-- game-level info and game-status to the client.

ALTER TABLE game_accounts
  ADD COLUMN IF NOT EXISTS is_suspended       BOOLEAN     NOT NULL DEFAULT FALSE,
  ADD COLUMN IF NOT EXISTS is_banned          BOOLEAN     NOT NULL DEFAULT FALSE,
  ADD COLUMN IF NOT EXISTS suspension_expires TIMESTAMPTZ,
  ADD COLUMN IF NOT EXISTS game_time_expires  TIMESTAMPTZ;
