-- SPDX-License-Identifier: AGPL-3.0-only

-- Add profile fields to accounts table (first_name, last_name, birth_date,
-- mobile_number, country_id) matching the captured /api/details and /userinfo
-- responses from account.battle.net.

ALTER TABLE accounts ADD COLUMN IF NOT EXISTS first_name    TEXT;
ALTER TABLE accounts ADD COLUMN IF NOT EXISTS last_name     TEXT;
ALTER TABLE accounts ADD COLUMN IF NOT EXISTS birth_date    TEXT;
ALTER TABLE accounts ADD COLUMN IF NOT EXISTS mobile_number TEXT;
ALTER TABLE accounts ADD COLUMN IF NOT EXISTS country_id    INTEGER;
