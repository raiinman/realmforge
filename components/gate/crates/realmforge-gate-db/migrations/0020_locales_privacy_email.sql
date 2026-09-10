-- SPDX-License-Identifier: AGPL-3.0-only

-- Locales supported by the account management UI, served by
-- GET /api/communication-preferences/supported-locales.
-- Values from the live account.battle.net capture (2026-05-29).
CREATE TABLE IF NOT EXISTS supported_locales (
    locale  TEXT PRIMARY KEY,
    sort_order INTEGER NOT NULL DEFAULT 0
);

INSERT INTO supported_locales (locale, sort_order) VALUES
    ('de_DE', 1), ('en_GB', 2), ('en_US', 3), ('es_ES', 4), ('es_MX', 5),
    ('fr_FR', 6), ('it_IT', 7), ('ja_JP', 8), ('ko_KR', 9), ('pl_PL', 10),
    ('pt_BR', 11), ('ru_RU', 12), ('th_TH', 13), ('zh_TW', 14)
ON CONFLICT (locale) DO NOTHING;

-- Additional privacy fields exposed by GET/PUT /api/privacy and
-- /api/privacy/social. The 10 base fields were added in migration 0017.
ALTER TABLE privacy_settings
    ADD COLUMN IF NOT EXISTS show_real_id              BOOLEAN NOT NULL DEFAULT true,
    -- PC variants from the GET /api/privacy capture.
    ADD COLUMN IF NOT EXISTS pc_allow_only_friend_whispers BOOLEAN NOT NULL DEFAULT false,
    ADD COLUMN IF NOT EXISTS pc_text_chat_enabled      BOOLEAN NOT NULL DEFAULT true,
    ADD COLUMN IF NOT EXISTS pc_voice_chat_enabled     BOOLEAN NOT NULL DEFAULT false,
    ADD COLUMN IF NOT EXISTS pc_voice_chat_speak_enabled BOOLEAN NOT NULL DEFAULT false,
    ADD COLUMN IF NOT EXISTS pc_real_id_enabled        BOOLEAN NOT NULL DEFAULT true,
    ADD COLUMN IF NOT EXISTS pc_groups_enabled         BOOLEAN NOT NULL DEFAULT true,
    ADD COLUMN IF NOT EXISTS pc_friends_of_friends_enabled BOOLEAN NOT NULL DEFAULT false,
    ADD COLUMN IF NOT EXISTS pc_profile_enabled        BOOLEAN NOT NULL DEFAULT true,
    ADD COLUMN IF NOT EXISTS pc_share_game_data_enabled BOOLEAN NOT NULL DEFAULT false;

-- Email confirmation tokens for POST /api/email/confirm.
CREATE TABLE IF NOT EXISTS email_confirm_tokens (
    token       TEXT PRIMARY KEY,
    account_id  BIGINT NOT NULL REFERENCES accounts (id) ON DELETE CASCADE,
    expires_at  TIMESTAMPTZ NOT NULL,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
