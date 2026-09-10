-- SPDX-License-Identifier: AGPL-3.0-only

ALTER TABLE bgs_sessions
ADD COLUMN build INTEGER,
ADD COLUMN platform TEXT,
ADD COLUMN locale TEXT;
