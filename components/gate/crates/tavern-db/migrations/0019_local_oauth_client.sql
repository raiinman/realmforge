-- SPDX-License-Identifier: AGPL-3.0-only

-- Local development OAuth client for oauth2c / curl testing.
-- The Battle.net client IDs in migration 0003 use production redirect URIs
-- (account.battle.net, battlenet:// scheme), so a local authorization_code
-- + PKCE run against oauth2c needs a client whose redirect points at
-- oauth2c's callback server (default http://localhost:9876/callback).
--
-- Credentials:
--   client_id:     tavern-local
--   client_secret: dev-secret   (SHA-256: 298754db2dbab6ec62605ceb0379eb7ee376580359449efe0caa3aa06cd56736)

INSERT INTO oauth_clients (client_id, redirect_uris, scopes, allowed_grants, require_2fa)
VALUES ('tavern-local',
        ARRAY['http://localhost:9876/callback', 'http://localhost:8080/callback'],
        ARRAY['account.full', 'account.full.privileged', 'account.standard'],
        ARRAY['authorization_code', 'refresh_token', 'client_credentials'],
        FALSE)
ON CONFLICT (client_id) DO NOTHING;

-- Confidential client secret (client_credentials / refresh_token grants).
UPDATE oauth_clients
SET client_secret_hash = '298754db2dbab6ec62605ceb0379eb7ee376580359449efe0caa3aa06cd56736'
WHERE client_id = 'tavern-local'
  AND client_secret_hash IS NULL;
