-- SPDX-License-Identifier: AGPL-3.0-only

-- Seed the known OAuth clients observed in real Battle.net traffic.
-- These are the client_ids the desktop app, web account UI, shop, and
-- developer portal send to /authorize. A preservation server recognizes
-- them rather than inventing its own.

INSERT INTO oauth_clients (client_id, redirect_uris, scopes, allowed_grants, require_2fa) VALUES
    -- Account management web frontend (account.battle.net).
    ('057adb2af62a4d59904f74754838c4c8',
     ARRAY['https://account.battle.net/callback/oauth2/code/account-settings'],
     ARRAY['account.full', 'account-settings.full'],
     ARRAY['authorization_code', 'refresh_token', 'client_credentials'],
     FALSE),
    -- Shop / storefront.
    ('a1645ac274514d8ebe9b5ffab1a30660',
     ARRAY['https://eu.shop.battle.net/login/oauth2/code/storefront',
           'https://us.shop.battle.net/login/oauth2/code/storefront'],
     ARRAY['account.full', 'commerce.catalog.basic'],
     ARRAY['authorization_code', 'refresh_token'],
     FALSE),
    -- Battle.net desktop app (Phoenix) — used for the RFC 8693 token-exchange
    -- to obtain DPLT tokens. No redirect URI (it uses the token endpoint
    -- directly, not the browser flow).
    ('a7f9b73e4e9c4e01a9c8056cddabff71',
     ARRAY['battlenet://oauth/callback'],
     ARRAY['account.standard', 'account.standard:modify', 'commerce.entitlements.basic',
           'commerce.inventory.full'],
     ARRAY['urn:ietf:params:oauth:grant-type:token-exchange', 'refresh_token', 'client_credentials'],
     FALSE),
    -- Developer portal.
    ('d24c87becefa45e788e556ed1f2386e3',
     ARRAY['http://localhost:8080/callback'],
     ARRAY['account.full.privileged'],
     ARRAY['authorization_code', 'refresh_token'],
     TRUE)
ON CONFLICT (client_id) DO NOTHING;
