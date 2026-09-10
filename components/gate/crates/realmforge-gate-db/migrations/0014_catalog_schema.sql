-- SPDX-License-Identifier: AGPL-3.0-only

-- Product catalog schema matching cascette-py/Blizzard TPR format.
-- Populated with production WoW data only (no PTR/dev/beta/alpha).

-- License IDs referenced by entitlement rules.
CREATE TABLE IF NOT EXISTS catalog_licenses (
    license_id  INTEGER PRIMARY KEY,
    program_id  TEXT    NOT NULL DEFAULT 'WoW',
    description TEXT    NOT NULL
);

-- Programs and their game-account requirements.
CREATE TABLE IF NOT EXISTS catalog_programs (
    program_id              TEXT PRIMARY KEY,
    is_game_account_level   BOOLEAN NOT NULL DEFAULT FALSE,
    config_json             JSONB   NOT NULL DEFAULT '{}'
);

-- Entitlement rules per program.
CREATE TABLE IF NOT EXISTS catalog_rules (
    id          SERIAL PRIMARY KEY,
    program_id  TEXT    NOT NULL REFERENCES catalog_programs (program_id),
    rule_seq    INTEGER NOT NULL,
    level       TEXT,
    match_json  JSONB,
    actions_json JSONB   NOT NULL,
    UNIQUE (program_id, rule_seq)
);

-- Products linked to license IDs or game-account rules.
CREATE TABLE IF NOT EXISTS catalog_products (
    product_id  TEXT PRIMARY KEY,
    program_id  TEXT NOT NULL REFERENCES catalog_programs (program_id),
    name        TEXT NOT NULL,
    UNIQUE (product_id)
);

-- License-to-product mappings (from entitlement rules).
CREATE TABLE IF NOT EXISTS catalog_product_licenses (
    product_id  TEXT    NOT NULL REFERENCES catalog_products (product_id) ON DELETE CASCADE,
    license_id  INTEGER NOT NULL REFERENCES catalog_licenses (license_id) ON DELETE CASCADE,
    PRIMARY KEY (product_id, license_id)
);

-- FK from account_licenses to catalog_licenses for referential integrity.
DO $$ BEGIN
  IF NOT EXISTS (
    SELECT 1 FROM pg_constraint WHERE conname = 'fk_account_licenses_catalog'
  ) THEN
    ALTER TABLE account_licenses
      ADD CONSTRAINT fk_account_licenses_catalog
        FOREIGN KEY (license_id) REFERENCES catalog_licenses (license_id);
  END IF;
END $$;

-- ============================================================

-- ============================================================
-- WoW production data (no PTR/dev/beta/alpha)
-- Sourced from cascette-py (cascette catalog show WoW).
-- ============================================================

-- Program
INSERT INTO catalog_programs (program_id, is_game_account_level, config_json) VALUES
    ('WoW', TRUE, '{}'::jsonb)
ON CONFLICT (program_id) DO NOTHING;

-- License IDs with descriptions
INSERT INTO catalog_licenses (license_id, program_id, description) VALUES
    (179712, 'WoW', 'Shadowlands / WoWX9'),
    (813186, 'WoW', 'Dragonflight / WoWX10'),
    (1106089, 'WoW', 'The War Within / WoWX12 (current retail)')
ON CONFLICT (license_id) DO NOTHING;

-- Products
INSERT INTO catalog_products (product_id, program_id, name) VALUES
    -- Base game (requires a game account, no license)
    ('WoW/retail', 'WoW', 'World of Warcraft'),
    ('WoW/wow_classic', 'WoW', 'WoW Classic'),
    ('WoW/wow_classic_era', 'WoW', 'WoW Classic Era'),
    ('WoW/wow_classic_anniversary', 'WoW', 'WoW Classic Anniversary'),
    -- Retail expansions (require specific licenses)
    ('WoWX12/retail', 'WoW', 'WoW: The War Within'),
    ('WoWX10/retail', 'WoW', 'WoW: Dragonflight'),
    ('WoWX9/retail', 'WoW', 'WoW: Shadowlands')
ON CONFLICT (product_id) DO NOTHING;

-- License-to-product mappings
INSERT INTO catalog_product_licenses (product_id, license_id) VALUES
    ('WoWX12/retail', 1106089),
    ('WoWX10/retail', 813186),
    ('WoWX9/retail', 179712)
ON CONFLICT (product_id, license_id) DO NOTHING;

-- Entitlement rules (readable form for the web UI)
INSERT INTO catalog_rules (program_id, rule_seq, level, match_json, actions_json) VALUES
    ('WoW', 0, NULL,
     '{"type":"match","conditions":[{"type":"game_account","program_id":"WoW","region":["US","EU","KR","TW","CN"]}]}'::jsonb,
     '{"add_products":["WoW/wow_classic","WoW/wow_classic_era","WoW/wow_classic_anniversary"]}'::jsonb),
    ('WoW', 1, NULL,
     '{"type":"run_first_rule","rules":[{"type":"match","conditions":[{"type":"license","license_id":1106089}],"add_products":["WoWX12/retail"]},{"type":"match","conditions":[{"type":"license","license_id":813186}],"add_products":["WoWX10/retail"]},{"type":"match","conditions":[{"type":"license","license_id":179712}],"add_products":["WoWX9/retail"]},{"type":"always","add_products":["WoW/retail"]}]}'::jsonb,
     '[]'::jsonb)
ON CONFLICT (program_id, rule_seq) DO NOTHING;
