-- SPDX-License-Identifier: AGPL-3.0-only

-- Add address fields to accounts table for /api/details/address (GET + PUT).

ALTER TABLE accounts ADD COLUMN IF NOT EXISTS street1    TEXT;
ALTER TABLE accounts ADD COLUMN IF NOT EXISTS street2    TEXT;
ALTER TABLE accounts ADD COLUMN IF NOT EXISTS city       TEXT;
ALTER TABLE accounts ADD COLUMN IF NOT EXISTS state      TEXT;
ALTER TABLE accounts ADD COLUMN IF NOT EXISTS postal_code TEXT;
