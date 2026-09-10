-- SPDX-License-Identifier: AGPL-3.0-only

-- Make birth_date mandatory.
--
-- The account-creation flow collects a date of birth and the seed data
-- provides one; every account must carry a valid birth date (the
-- age-of-majority check depends on it). Existing rows get a fixed adult
-- backfill; new inserts without an explicit value default to the same
-- adult date so bare INSERT (email-only) paths keep working.

ALTER TABLE accounts ALTER COLUMN birth_date SET DEFAULT '1990-01-01';
UPDATE accounts SET birth_date = '1990-01-01' WHERE birth_date IS NULL;
ALTER TABLE accounts ALTER COLUMN birth_date SET NOT NULL;
