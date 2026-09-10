// SPDX-License-Identifier: AGPL-3.0-only

//! Realmforge-owned Gate projection for the realm catalog.
//!
//! This module deliberately lives inside the covered Gate derivative. It is
//! not Realmforge Core's canonical Realm model. Its job is to translate the
//! product-level idea of a realm into the exact fields this retired client
//! family needs while we build the Core <-> Gate contract.

use std::collections::{HashMap, HashSet};
use std::path::Path;

use anyhow::{Context, bail};
use serde::Deserialize;

pub const DEFAULT_CLIENT_BUILD: i32 = 31_650;

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ClientVersion {
    pub major: u32,
    pub minor: u32,
    pub revision: u32,
    pub build: u32,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RealmDefinition {
    /// Stable Realmforge-side identity for this Gate projection.
    pub id: String,
    pub display_name: String,
    pub public_address: String,
    pub game_port: u16,

    #[serde(default = "default_region")]
    pub region: u8,
    #[serde(default = "default_site")]
    pub site: u8,
    #[serde(default = "default_realm_index")]
    pub realm_index: u8,
    #[serde(default)]
    pub address_flags: u8,

    #[serde(default = "default_one_u32")]
    pub timezone_id: u32,
    #[serde(default = "default_one_u32")]
    pub population_state: u32,
    #[serde(default = "default_one_u32")]
    pub category_id: u32,
    #[serde(default = "default_one_u32")]
    pub config_id: u32,
    #[serde(default = "default_one_u32")]
    pub language_id: u32,
    #[serde(default)]
    pub realm_flags: u32,

    /// Exact client builds this realm is intentionally advertised to.
    #[serde(default = "default_supported_builds")]
    pub supported_builds: Vec<i32>,
}

impl RealmDefinition {
    /// BGS WoW realm address: region | site | realm | address flags.
    pub fn wow_realm_address(&self) -> u32 {
        (u32::from(self.region) << 24)
            | (u32::from(self.site) << 16)
            | (u32::from(self.realm_index) << 8)
            | u32::from(self.address_flags)
    }

    pub fn supports_build(&self, build: i32) -> bool {
        self.supported_builds.contains(&build)
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RegistryDocument {
    realms: Vec<RealmDefinition>,
    #[serde(default = "default_build_profiles")]
    build_profiles: HashMap<i32, ClientVersion>,
    #[serde(default)]
    allow_unknown_builds: bool,
}

#[derive(Debug, Clone)]
pub struct RealmRegistry {
    realms: Vec<RealmDefinition>,
    build_profiles: HashMap<i32, ClientVersion>,
    allow_unknown_builds: bool,
}

impl RealmRegistry {
    /// Load the temporary Gate-side registry.
    ///
    /// Precedence:
    /// 1. REALMFORGE_REALMS_JSON
    /// 2. REALMFORGE_REALMS_FILE
    /// 3. one local compatibility realm built from legacy REALM_ADDRESS /
    ///    REALM_PORT plus REALMFORGE_REALM_NAME
    pub fn from_env() -> anyhow::Result<Self> {
        let inline = std::env::var("REALMFORGE_REALMS_JSON").ok();
        let file = std::env::var("REALMFORGE_REALMS_FILE").ok();

        if inline.is_some() && file.is_some() {
            bail!("set only one of REALMFORGE_REALMS_JSON or REALMFORGE_REALMS_FILE");
        }

        let mut registry = if let Some(json) = inline {
            Self::from_json(&json).context("invalid REALMFORGE_REALMS_JSON")?
        } else if let Some(path) = file {
            Self::from_file(path).context("invalid REALMFORGE_REALMS_FILE")?
        } else {
            Self::legacy_default_from_env()?
        };

        if let Ok(raw) = std::env::var("REALMFORGE_ALLOW_UNKNOWN_BUILDS") {
            registry.allow_unknown_builds = parse_bool(&raw)
                .with_context(|| format!("invalid REALMFORGE_ALLOW_UNKNOWN_BUILDS={raw:?}"))?;
        }

        registry.validate()?;
        Ok(registry)
    }

    pub fn from_json(json: &str) -> anyhow::Result<Self> {
        let document: RegistryDocument = serde_json::from_str(json)?;
        let registry = Self {
            realms: document.realms,
            build_profiles: document.build_profiles,
            allow_unknown_builds: document.allow_unknown_builds,
        };
        registry.validate()?;
        Ok(registry)
    }

    pub fn from_file(path: impl AsRef<Path>) -> anyhow::Result<Self> {
        let path = path.as_ref();
        let json = std::fs::read_to_string(path)
            .with_context(|| format!("read realm registry {}", path.display()))?;
        Self::from_json(&json)
    }

    fn legacy_default_from_env() -> anyhow::Result<Self> {
        let public_address =
            std::env::var("REALM_ADDRESS").unwrap_or_else(|_| "127.0.0.1".to_string());
        let game_port = std::env::var("REALM_PORT")
            .ok()
            .map(|v| v.parse::<u16>().context("invalid REALM_PORT"))
            .transpose()?
            .unwrap_or(8085);
        let display_name =
            std::env::var("REALMFORGE_REALM_NAME").unwrap_or_else(|_| "RealmForge".to_string());

        Ok(Self {
            realms: vec![RealmDefinition {
                id: "default".to_string(),
                display_name,
                public_address,
                game_port,
                region: 1,
                site: 1,
                realm_index: 1,
                address_flags: 0,
                timezone_id: 1,
                population_state: 1,
                category_id: 1,
                config_id: 1,
                language_id: 1,
                realm_flags: 0,
                supported_builds: default_supported_builds(),
            }],
            build_profiles: default_build_profiles(),
            allow_unknown_builds: false,
        })
    }

    pub fn len(&self) -> usize {
        self.realms.len()
    }

    pub fn is_empty(&self) -> bool {
        self.realms.is_empty()
    }

    pub fn effective_build(build: Option<i32>) -> i32 {
        build.unwrap_or(DEFAULT_CLIENT_BUILD)
    }

    pub fn version_for_build(&self, build: Option<i32>) -> Option<ClientVersion> {
        let build = Self::effective_build(build);
        if let Some(version) = self.build_profiles.get(&build).copied() {
            return Some(version);
        }

        self.allow_unknown_builds.then_some(ClientVersion {
            major: 1,
            minor: 13,
            revision: 2,
            build: build.max(0) as u32,
        })
    }

    pub fn advertised_realms(
        &self,
        build: Option<i32>,
    ) -> Vec<(&RealmDefinition, ClientVersion)> {
        let effective = Self::effective_build(build);
        let Some(version) = self.version_for_build(build) else {
            return Vec::new();
        };

        self.realms
            .iter()
            .filter(|realm| realm.supports_build(effective) || self.allow_unknown_builds)
            .map(|realm| (realm, version))
            .collect()
    }

    pub fn find_join_target(
        &self,
        wow_realm_address: u64,
        build: Option<i32>,
    ) -> Option<&RealmDefinition> {
        let effective = Self::effective_build(build);
        let address = u32::try_from(wow_realm_address).ok()?;
        self.realms.iter().find(|realm| {
            realm.wow_realm_address() == address
                && (realm.supports_build(effective) || self.allow_unknown_builds)
        })
    }

    fn validate(&self) -> anyhow::Result<()> {
        if self.realms.is_empty() {
            bail!("Realmforge Gate realm registry must contain at least one realm");
        }

        let mut ids = HashSet::new();
        let mut addresses = HashSet::new();

        for realm in &self.realms {
            if realm.id.trim().is_empty() {
                bail!("realm id may not be empty");
            }
            if realm.display_name.trim().is_empty() {
                bail!("realm {} displayName may not be empty", realm.id);
            }
            if realm.public_address.trim().is_empty() {
                bail!("realm {} publicAddress may not be empty", realm.id);
            }
            if realm.supported_builds.is_empty() {
                bail!("realm {} must declare supportedBuilds explicitly", realm.id);
            }
            if !ids.insert(realm.id.clone()) {
                bail!("duplicate realm id {}", realm.id);
            }
            if !addresses.insert(realm.wow_realm_address()) {
                bail!(
                    "duplicate WoW realm address 0x{:08x}",
                    realm.wow_realm_address()
                );
            }
            if !self.allow_unknown_builds {
                for build in &realm.supported_builds {
                    if !self.build_profiles.contains_key(build) {
                        bail!(
                            "realm {} declares build {} without an exact build profile",
                            realm.id,
                            build
                        );
                    }
                }
            }
        }

        Ok(())
    }
}

impl Default for RealmRegistry {
    fn default() -> Self {
        Self {
            realms: vec![RealmDefinition {
                id: "default".to_string(),
                display_name: "RealmForge".to_string(),
                public_address: "127.0.0.1".to_string(),
                game_port: 8085,
                region: 1,
                site: 1,
                realm_index: 1,
                address_flags: 0,
                timezone_id: 1,
                population_state: 1,
                category_id: 1,
                config_id: 1,
                language_id: 1,
                realm_flags: 0,
                supported_builds: default_supported_builds(),
            }],
            build_profiles: default_build_profiles(),
            allow_unknown_builds: false,
        }
    }
}

fn default_region() -> u8 {
    1
}

fn default_site() -> u8 {
    1
}

fn default_realm_index() -> u8 {
    1
}

fn default_one_u32() -> u32 {
    1
}

fn default_supported_builds() -> Vec<i32> {
    vec![31_650, 40_618]
}

fn default_build_profiles() -> HashMap<i32, ClientVersion> {
    HashMap::from([
        (
            31_650,
            ClientVersion {
                major: 1,
                minor: 13,
                revision: 2,
                build: 31_650,
            },
        ),
        (
            40_618,
            ClientVersion {
                major: 1,
                minor: 14,
                revision: 0,
                build: 40_618,
            },
        ),
    ])
}

fn parse_bool(raw: &str) -> anyhow::Result<bool> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => Ok(true),
        "0" | "false" | "no" | "off" => Ok(false),
        _ => bail!("expected boolean"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_registry_is_realmforge_not_tavern() {
        let registry = RealmRegistry::default();
        assert_eq!(registry.realms[0].display_name, "RealmForge");
        assert_eq!(registry.realms[0].wow_realm_address(), 0x0101_0100);
    }

    #[test]
    fn exact_build_profiles_are_required_by_default() {
        let registry = RealmRegistry::default();
        assert!(registry.version_for_build(Some(31_650)).is_some());
        assert!(registry.version_for_build(Some(40_618)).is_some());
        assert!(registry.version_for_build(Some(99_999)).is_none());
        assert!(registry.advertised_realms(Some(99_999)).is_empty());
    }

    #[test]
    fn parses_multiple_realms_and_selects_join_target() {
        let registry = RealmRegistry::from_json(
            r#"{
                "realms": [
                    {
                        "id": "alpha",
                        "displayName": "RealmForge Alpha",
                        "publicAddress": "10.0.0.10",
                        "gamePort": 8085,
                        "region": 1,
                        "site": 1,
                        "realmIndex": 1,
                        "supportedBuilds": [31650]
                    },
                    {
                        "id": "beta",
                        "displayName": "RealmForge Beta",
                        "publicAddress": "10.0.0.11",
                        "gamePort": 8086,
                        "region": 1,
                        "site": 1,
                        "realmIndex": 2,
                        "supportedBuilds": [31650]
                    }
                ]
            }"#,
        )
        .unwrap();

        assert_eq!(registry.len(), 2);
        let beta = registry.find_join_target(0x0101_0200, Some(31_650)).unwrap();
        assert_eq!(beta.id, "beta");
        assert_eq!(beta.game_port, 8086);
    }

    #[test]
    fn duplicate_wire_addresses_are_rejected() {
        let error = RealmRegistry::from_json(
            r#"{
                "realms": [
                    {
                        "id": "a",
                        "displayName": "A",
                        "publicAddress": "127.0.0.1",
                        "gamePort": 8085,
                        "supportedBuilds": [31650]
                    },
                    {
                        "id": "b",
                        "displayName": "B",
                        "publicAddress": "127.0.0.1",
                        "gamePort": 8086,
                        "supportedBuilds": [31650]
                    }
                ]
            }"#,
        )
        .unwrap_err();
        assert!(error.to_string().contains("duplicate WoW realm address"));
    }
}
