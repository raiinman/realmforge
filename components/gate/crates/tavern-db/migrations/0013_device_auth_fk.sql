-- SPDX-License-Identifier: AGPL-3.0-only

-- Add foreign key constraints that were missed in the original migration.
-- account_id is nullable (device auth can start before login).

ALTER TABLE device_authorizations
  ADD CONSTRAINT fk_device_auth_account
    FOREIGN KEY (account_id) REFERENCES accounts (id) ON DELETE SET NULL;

ALTER TABLE device_authorizations
  ADD CONSTRAINT fk_device_auth_client
    FOREIGN KEY (client_id) REFERENCES oauth_clients (client_id) ON DELETE CASCADE;
