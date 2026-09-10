-- SPDX-License-Identifier: AGPL-3.0-only

-- Account-specific communication preferences, matching the real
-- Battle.net /api/communication-preferences response format.

CREATE TABLE IF NOT EXISTS communication_preferences (
    account_id BIGINT PRIMARY KEY REFERENCES accounts (id) ON DELETE CASCADE,
    receive_targeted_ads BOOLEAN NOT NULL DEFAULT false,
    receive_blizzard_offers BOOLEAN NOT NULL DEFAULT false,
    enable_personalized_recommendations BOOLEAN NOT NULL DEFAULT false,
    limited_descriptions BOOLEAN NOT NULL DEFAULT false,
    world_of_warcraft_communications BOOLEAN NOT NULL DEFAULT true,
    warcraft_rumble_communications BOOLEAN NOT NULL DEFAULT true,
    diablo_communications BOOLEAN NOT NULL DEFAULT false,
    overwatch_communications BOOLEAN NOT NULL DEFAULT false,
    hearthstone_communications BOOLEAN NOT NULL DEFAULT false,
    call_of_duty_communications BOOLEAN NOT NULL DEFAULT false,
    blizzard_classics_communications BOOLEAN NOT NULL DEFAULT false,
    surveys_communication BOOLEAN NOT NULL DEFAULT true,
    xbox_communications BOOLEAN NOT NULL DEFAULT false
);
