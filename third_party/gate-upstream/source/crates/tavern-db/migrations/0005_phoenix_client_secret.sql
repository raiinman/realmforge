-- SPDX-License-Identifier: AGPL-3.0-only

-- Enable pgcrypto for digest() function.
CREATE EXTENSION IF NOT EXISTS pgcrypto;

-- Add the Phoenix desktop app's hardcoded client secret to the seed.
-- The secret is plaintext in the binary (C++ string literal); it's a
-- first-party confidential client. Stored as a SHA-256 hex hash.
UPDATE oauth_clients
SET client_secret_hash = encode(digest(
    '2Lu0QbF5FLrQoLmIDjcPjojlEu4zirF2', 'sha256'), 'hex')
WHERE client_id = 'a7f9b73e4e9c4e01a9c8056cddabff71';
