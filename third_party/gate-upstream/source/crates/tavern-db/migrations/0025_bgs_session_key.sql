-- SPDX-License-Identifier: AGPL-3.0-only

-- Persist the BGS session key with the SSO session so that restored
-- sessions can still serve Param_BnetSessionKey in the
-- Command_RealmJoinRequest_v1 response (the game-server TLS material
-- for the world handshake).

ALTER TABLE bgs_sessions ADD COLUMN IF NOT EXISTS session_key BYTEA;
