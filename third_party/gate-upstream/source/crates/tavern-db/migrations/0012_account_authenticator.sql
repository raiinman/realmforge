-- SPDX-License-Identifier: AGPL-3.0-only

-- Authenticator flag on accounts. When enabled, the login flow returns
-- authentication_state=AUTHENTICATOR instead of DONE, and the client
-- must POST the 8-digit code to /login/authenticator.

ALTER TABLE accounts ADD COLUMN IF NOT EXISTS has_authenticator BOOLEAN NOT NULL DEFAULT FALSE;
