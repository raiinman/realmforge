-- SPDX-License-Identifier: AGPL-3.0-only

-- Account privacy settings matching /api/privacy response format.
-- Defaults match the real Battle.net API for a new account.

CREATE TABLE IF NOT EXISTS privacy_settings (
    account_id BIGINT PRIMARY KEY REFERENCES accounts (id) ON DELETE CASCADE,
    enable_text_chat BOOLEAN NOT NULL DEFAULT true,
    enable_private_text_chat BOOLEAN NOT NULL DEFAULT true,
    only_allow_friend_whispers BOOLEAN NOT NULL DEFAULT true,
    enable_voice_chat BOOLEAN NOT NULL DEFAULT false,
    enable_voice_chat_speak BOOLEAN NOT NULL DEFAULT false,
    enable_friends_management BOOLEAN NOT NULL DEFAULT true,
    enable_groups BOOLEAN NOT NULL DEFAULT true,
    enable_real_id BOOLEAN NOT NULL DEFAULT true,
    enable_friends_of_friends BOOLEAN NOT NULL DEFAULT false,
    enable_parental_control BOOLEAN NOT NULL DEFAULT true,
    enable_force_mute BOOLEAN NOT NULL DEFAULT false,
    enable_third_party_sharing BOOLEAN NOT NULL DEFAULT false,
    has_sms BOOLEAN NOT NULL DEFAULT false,
    profile_visibility TEXT NOT NULL DEFAULT 'FRIENDS'
);
