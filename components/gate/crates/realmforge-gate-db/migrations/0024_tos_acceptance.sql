-- SPDX-License-Identifier: AGPL-3.0-only

-- Per-account Terms-of-Service acceptance state.
--
-- The 1.13.2 in-client login flow (realmforge-interop-analysis.md §4.11a) has a
-- LEGAL gate: until the account has accepted the current ToS version, the
-- server returns authentication_state="LEGAL" and must not hand out a
-- login_ticket. tos_version holds the latest agreement version the account
-- accepted ('' = never); bumping the server's current version re-gates every
-- account, matching real ToS-change behavior.

ALTER TABLE accounts ADD COLUMN IF NOT EXISTS tos_version    TEXT        NOT NULL DEFAULT '';
ALTER TABLE accounts ADD COLUMN IF NOT EXISTS tos_accepted_at TIMESTAMPTZ;
