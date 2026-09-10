-- Enable case-insensitive text so email comparisons are case-insensitive
-- without a separate lower() index.
CREATE EXTENSION IF NOT EXISTS citext;

-- A Battle.net account. The numeric id is the OAuth `sub` claim.
CREATE TABLE accounts (
    id              BIGSERIAL PRIMARY KEY,
    email           CITEXT    NOT NULL UNIQUE,
    email_verified  BOOLEAN   NOT NULL DEFAULT FALSE,
    battletag       TEXT      NOT NULL DEFAULT '',
    country_code    CHAR(3)   NOT NULL DEFAULT '',
    region          SMALLINT  NOT NULL DEFAULT 0,
    locale          TEXT      NOT NULL DEFAULT 'enUS',
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- SRP6a credential material for an account. Stores the verifier, never the
-- password. Separate from accounts so credential rotation never touches the
-- account row.
CREATE TABLE credentials (
    account_id      BIGINT PRIMARY KEY REFERENCES accounts (id) ON DELETE CASCADE,
    srp_salt        BYTEA   NOT NULL,
    srp_verifier    BYTEA   NOT NULL,
    srp_iterations  INTEGER NOT NULL,
    srp_version     SMALLINT NOT NULL,
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
