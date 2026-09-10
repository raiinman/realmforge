use std::collections::{BTreeMap, BTreeSet};

use crate::GateError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ClientBuild(pub u32);

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RealmId(String);

impl RealmId {
    pub fn new(value: impl Into<String>) -> Result<Self, GateError> {
        let value = value.into();
        let trimmed = value.trim();
        if trimmed.is_empty() {
            return Err(GateError::InvalidRealmId);
        }
        Ok(Self(trimmed.to_owned()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RealmEndpoint {
    pub host: String,
    pub port: u16,
}

impl RealmEndpoint {
    pub fn new(host: impl Into<String>, port: u16) -> Result<Self, GateError> {
        let host = host.into();
        if host.trim().is_empty() {
            return Err(GateError::InvalidHost);
        }
        if port == 0 {
            return Err(GateError::InvalidPort);
        }
        Ok(Self { host, port })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RealmDescriptor {
    pub id: RealmId,
    pub display_name: String,
    pub endpoint: RealmEndpoint,
    pub compatible_builds: BTreeSet<ClientBuild>,
}

impl RealmDescriptor {
    pub fn supports(&self, build: ClientBuild) -> bool {
        self.compatible_builds.contains(&build)
    }
}

#[derive(Debug, Clone, Default)]
pub struct RealmCatalog {
    realms: BTreeMap<RealmId, RealmDescriptor>,
}

impl RealmCatalog {
    pub fn new(realms: impl IntoIterator<Item = RealmDescriptor>) -> Result<Self, GateError> {
        let mut catalog = Self::default();
        for realm in realms {
            if realm.display_name.trim().is_empty() {
                return Err(GateError::InvalidDisplayName);
            }
            if realm.compatible_builds.is_empty() {
                return Err(GateError::NoCompatibleBuilds(realm.id.clone()));
            }
            if catalog.realms.insert(realm.id.clone(), realm).is_some() {
                return Err(GateError::DuplicateRealmId);
            }
        }
        Ok(catalog)
    }

    pub fn list_for_build(&self, build: ClientBuild) -> Vec<&RealmDescriptor> {
        self.realms
            .values()
            .filter(|realm| realm.supports(build))
            .collect()
    }

    pub fn resolve_for_build(
        &self,
        id: &RealmId,
        build: ClientBuild,
    ) -> Result<&RealmDescriptor, GateError> {
        let realm = self.realms.get(id).ok_or(GateError::RealmNotFound)?;
        if !realm.supports(build) {
            return Err(GateError::BuildNotSupported {
                realm: id.clone(),
                build,
            });
        }
        Ok(realm)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn realm(id: &str, builds: &[u32]) -> RealmDescriptor {
        RealmDescriptor {
            id: RealmId::new(id).unwrap(),
            display_name: format!("Realm {id}"),
            endpoint: RealmEndpoint::new("127.0.0.1", 8085).unwrap(),
            compatible_builds: builds.iter().copied().map(ClientBuild).collect(),
        }
    }

    #[test]
    fn catalog_fails_closed_for_unknown_build() {
        let catalog = RealmCatalog::new([realm("alpha", &[40618])]).unwrap();
        assert!(catalog.list_for_build(ClientBuild(99999)).is_empty());
    }

    #[test]
    fn exact_build_match_lists_only_compatible_realms() {
        let catalog =
            RealmCatalog::new([realm("alpha", &[40618]), realm("beta", &[31650, 40618])]).unwrap();

        let ids: Vec<_> = catalog
            .list_for_build(ClientBuild(31650))
            .into_iter()
            .map(|realm| realm.id.as_str())
            .collect();

        assert_eq!(ids, vec!["beta"]);
    }

    #[test]
    fn duplicate_realm_ids_are_rejected() {
        let result = RealmCatalog::new([realm("alpha", &[40618]), realm("alpha", &[40618])]);
        assert_eq!(result.unwrap_err(), GateError::DuplicateRealmId);
    }

    #[test]
    fn unsupported_join_is_rejected_before_bridge() {
        let catalog = RealmCatalog::new([realm("alpha", &[40618])]).unwrap();
        let id = RealmId::new("alpha").unwrap();
        let err = catalog
            .resolve_for_build(&id, ClientBuild(31650))
            .unwrap_err();

        assert_eq!(
            err,
            GateError::BuildNotSupported {
                realm: id,
                build: ClientBuild(31650)
            }
        );
    }
}
