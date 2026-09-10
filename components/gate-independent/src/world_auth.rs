use crate::{ClientBuild, GameAccountProjection, GateError, RealmEndpoint, RealmId};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldAuthRequest {
    pub realm: RealmId,
    pub build: ClientBuild,
    pub account: GameAccountProjection,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldAuthGrant {
    pub endpoint: RealmEndpoint,
    pub join_ticket: Vec<u8>,
    pub session_material: Vec<u8>,
}

/// Emulator-specific world authentication belongs behind this contract.
///
/// Implementations may use an emulator database, RPC endpoint, sidecar, or
/// another adapter. Gate itself does not own emulator schema or wire details.
pub trait WorldAuthBridge {
    fn issue_world_auth(&self, request: &WorldAuthRequest) -> Result<WorldAuthGrant, GateError>;
}
