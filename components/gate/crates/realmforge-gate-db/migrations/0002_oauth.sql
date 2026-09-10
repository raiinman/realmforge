-- SPDX-License-Identifier: AGPL-3.0-only

-- OAuth provider and account-management tables.

-- Registered OAuth clients. Pre-seeded with the client ids observed in real
-- Battle.net traffic. client_secret_hash is NULL for public clients.
CREATE TABLE oauth_clients (
    client_id           TEXT        PRIMARY KEY,
    client_secret_hash  TEXT,
    redirect_uris       TEXT[]      NOT NULL DEFAULT '{}',
    scopes              TEXT[]      NOT NULL DEFAULT '{}',
    allowed_grants      TEXT[]      NOT NULL DEFAULT '{}',
    require_2fa         BOOLEAN     NOT NULL DEFAULT FALSE
);

-- Short-lived, single-use authorization codes minted at the authorize
-- endpoint, exchanged at /token for access/refresh tokens.
CREATE TABLE authorization_codes (
    code            TEXT        PRIMARY KEY,
    client_id       TEXT        NOT NULL REFERENCES oauth_clients (client_id),
    account_id      BIGINT      NOT NULL REFERENCES accounts (id) ON DELETE CASCADE,
    scope           TEXT        NOT NULL DEFAULT '',
    redirect_uri    TEXT        NOT NULL,
    code_challenge  TEXT,
    nonce           TEXT,
    expires_at      TIMESTAMPTZ NOT NULL,
    used            BOOLEAN     NOT NULL DEFAULT FALSE
);

-- Opaque, rotatable, revocable refresh tokens.
CREATE TABLE refresh_tokens (
    token           TEXT        PRIMARY KEY,
    account_id      BIGINT      NOT NULL REFERENCES accounts (id) ON DELETE CASCADE,
    client_id       TEXT        NOT NULL REFERENCES oauth_clients (client_id),
    scope           TEXT        NOT NULL DEFAULT '',
    expires_at      TIMESTAMPTZ NOT NULL,
    rotated_from    TEXT        REFERENCES refresh_tokens (token)
);

-- Web sessions backing the SESSIONID cookie.
CREATE TABLE sessions (
    session_id      UUID        PRIMARY KEY,
    account_id      BIGINT      NOT NULL REFERENCES accounts (id) ON DELETE CASCADE,
    expires_at      TIMESTAMPTZ NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    user_agent      TEXT,
    ip              TEXT
);

-- One-time service tickets bridging SRP login into the authorize flow.
CREATE TABLE service_tickets (
    st              TEXT        PRIMARY KEY,
    account_id      BIGINT     NOT NULL REFERENCES accounts (id) ON DELETE CASCADE,
    region          SMALLINT   NOT NULL DEFAULT 0,
    expires_at      TIMESTAMPTZ NOT NULL,
    used            BOOLEAN     NOT NULL DEFAULT FALSE
);

-- RSA signing key material for RS256 JWTs and JWKS rotation.
CREATE TABLE signing_keys (
    kid             TEXT        PRIMARY KEY,
    private_key_pem TEXT        NOT NULL,
    public_jwk      JSONB       NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    retired_at      TIMESTAMPTZ
);

-- Minimal entitlement model: "account owns game X".
CREATE TABLE account_licenses (
    account_id      BIGINT      NOT NULL REFERENCES accounts (id) ON DELETE CASCADE,
    license_id      BIGINT      NOT NULL,
    level           TEXT        NOT NULL DEFAULT 'exact',
    granted_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (account_id, license_id)
);
