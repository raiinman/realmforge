// SPDX-License-Identifier: AGPL-3.0-only

//! `GameUtilities::ProcessClientRequest` handler for the realm-list flow.
//!
//! Realmforge keeps the retired-client wire contract in Gate, but realm
//! identity, display names, endpoints, and explicit build compatibility are
//! now supplied by `realmforge_realms::RealmRegistry` rather than hard-coded
//! Realmforge constants.

use std::sync::Arc;

use flate2::Compression;
use flate2::write::ZlibEncoder;
use prost::Message as _;
use prost::bytes::Bytes;
use rand::RngCore;
use realmforge_gate_bgs::frame::serialize_frame;
use realmforge_gate_bgs::{Attribute, ClientRequest, ClientResponse, Header, Variant};
use tracing::{info, warn};

use super::realmforge_realms::{ClientVersion, RealmDefinition, RealmRegistry};

/// Build a response to `GameUtilities::ProcessClientRequest`.
pub async fn handle_process_client_request(
    state: &Arc<super::BgsState>,
    session: &mut super::BgsSession,
    header: &Header,
    body: &[u8],
) -> anyhow::Result<Vec<Bytes>> {
    let sid = &session.id;
    let request = ClientRequest::decode(body)?;

    let command_attr = request
        .attribute
        .iter()
        .find(|a| a.name.starts_with("Command_"));

    let command_name = match command_attr {
        Some(a) => a.name.as_str(),
        None => {
            warn!(
                session_id = %sid,
                "ProcessClientRequest with no Command_* attribute"
            );
            return Ok(vec![build_empty_response(header)?]);
        }
    };

    info!(session_id = %sid, command_name, "ProcessClientRequest");

    match command_name {
        "Command_RealmListTicketRequest_v1" => {
            handle_realm_list_ticket_request(session, header, &request)
        }
        "Command_RealmListRequest_v1" => {
            handle_realm_list_request(state, session, header, &request)
        }
        "Command_LastCharPlayedRequest_v1" => {
            info!(session_id = %sid, "LastCharPlayed — returning empty");
            Ok(vec![build_empty_response(header)?])
        }
        "Command_RealmJoinRequest_v1" => {
            handle_realm_join_request(&state.realm_registry, session, header, &request)
        }
        "Command_CharacterListRequest_v1" => {
            info!(session_id = %sid, "CharacterList — returning empty");
            Ok(vec![build_empty_response(header)?])
        }
        _ => {
            info!(
                session_id = %sid,
                command_name,
                "unknown command — returning empty"
            );
            Ok(vec![build_empty_response(header)?])
        }
    }
}

/// Handle `Command_RealmListTicketRequest_v1`.
///
/// This remains inherited compatibility behavior until the ticket contract is
/// independently captured. The literal is therefore intentionally visible as
/// a known replacement target rather than disguised as Realmforge policy.
fn handle_realm_list_ticket_request(
    session: &super::BgsSession,
    header: &Header,
    request: &ClientRequest,
) -> anyhow::Result<Vec<Bytes>> {
    let identity_blob = request
        .attribute
        .iter()
        .find(|a| a.name == "Param_Identity")
        .and_then(|a| a.value.blob_value.as_deref());

    info!(
        session_id = %session.id,
        identity = ?identity_blob.map(|b| String::from_utf8_lossy(b).to_string()),
        "RealmListTicketRequest"
    );

    let ticket = b"AuthRealmListTicket";

    let response = ClientResponse {
        attribute: vec![Attribute {
            name: "Param_RealmListTicket".to_string(),
            value: Variant {
                blob_value: Some(ticket.to_vec()),
                ..Default::default()
            },
        }],
    };

    build_response(header, &response)
}

/// Handle `Command_RealmListRequest_v1` using Realmforge's Gate registry.
fn handle_realm_list_request(
    state: &Arc<super::BgsState>,
    session: &super::BgsSession,
    header: &Header,
    _request: &ClientRequest,
) -> anyhow::Result<Vec<Bytes>> {
    let advertised = state.realm_registry.advertised_realms(session.build);
    let effective_build = RealmRegistry::effective_build(session.build);

    if advertised.is_empty() {
        warn!(
            session_id = %session.id,
            build = effective_build,
            "no Realmforge realm is explicitly compatible with this client build"
        );
    } else {
        info!(
            session_id = %session.id,
            build = effective_build,
            realm_count = advertised.len(),
            "RealmListRequest"
        );
    }

    let updates: Vec<serde_json::Value> = advertised
        .iter()
        .map(|(realm, version)| realm_update_json(realm, *version))
        .collect();

    let realm_list_json = serde_json::json!({ "updates": updates });
    let realm_blob = zlib_blob(&format!("JSONRealmListUpdates:{realm_list_json}"))?;

    // Character counts remain an explicit compatibility gap until Bridge can
    // supply them from the selected emulator adapter.
    let count_json = serde_json::json!({ "counts": [] });
    let count_blob = zlib_blob(&format!("JSONRealmCharacterCountList:{count_json}"))?;

    let response = ClientResponse {
        attribute: vec![
            Attribute {
                name: "Param_RealmList".to_string(),
                value: Variant {
                    blob_value: Some(realm_blob),
                    ..Default::default()
                },
            },
            Attribute {
                name: "Param_CharacterCountList".to_string(),
                value: Variant {
                    blob_value: Some(count_blob),
                    ..Default::default()
                },
            },
        ],
    };

    build_response(header, &response)
}

fn realm_update_json(realm: &RealmDefinition, version: ClientVersion) -> serde_json::Value {
    serde_json::json!({
        "update": {
            "wowRealmAddress": realm.wow_realm_address(),
            "cfgTimezonesID": realm.timezone_id,
            "populationState": realm.population_state,
            "cfgCategoriesID": realm.category_id,
            "version": {
                "versionMajor": version.major,
                "versionMinor": version.minor,
                "versionRevision": version.revision,
                "versionBuild": version.build,
            },
            "cfgRealmsID": u32::from(realm.realm_index),
            "flags": realm.realm_flags,
            "name": realm.display_name,
            "cfgConfigsID": realm.config_id,
            "cfgLanguagesID": realm.language_id,
        },
        "deleting": false,
    })
}

/// Handle `Command_RealmJoinRequest_v1` by resolving the selected wire address
/// back to a Realmforge Gate realm definition.
fn handle_realm_join_request(
    registry: &RealmRegistry,
    session: &super::BgsSession,
    header: &Header,
    request: &ClientRequest,
) -> anyhow::Result<Vec<Bytes>> {
    let realm_address = request
        .attribute
        .iter()
        .find(|a| a.name == "Param_RealmAddress")
        .and_then(|a| a.value.uint_value);

    let Some(realm_address) = realm_address else {
        warn!(
            session_id = %session.id,
            "RealmJoinRequest without Param_RealmAddress"
        );
        return Ok(vec![build_error_response(header)?]);
    };

    let Some(realm) = registry.find_join_target(realm_address, session.build) else {
        warn!(
            session_id = %session.id,
            realm_address,
            build = RealmRegistry::effective_build(session.build),
            "RealmJoinRequest for unknown or incompatible realm"
        );
        return Ok(vec![build_error_response(header)?]);
    };

    let family = if realm.public_address.contains(':') {
        2
    } else {
        1
    };
    let addresses_json = serde_json::json!({
        "families": [{
            "family": family,
            "addresses": [{
                "ip": realm.public_address,
                "port": realm.game_port
            }]
        }]
    });
    let addresses_blob = zlib_blob(&format!("JSONRealmListServerIPAddresses:{addresses_json}"))?;

    // Inherited compatibility behavior: 32 random bytes as the join secret.
    // The realm-side consumer contract remains a P0 capture/integration task.
    let mut join_secret = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut join_secret);

    // Inherited compatibility behavior: account id used as opaque join ticket.
    // Keep this visibly temporary until Bridge/world-auth owns the real handoff.
    let ticket = session
        ._account_id
        .map(|id| id.to_string())
        .unwrap_or_else(|| "unknown".to_string());

    let mut attributes = vec![
        Attribute {
            name: "Param_ServerAddresses".to_string(),
            value: Variant {
                blob_value: Some(addresses_blob),
                ..Default::default()
            },
        },
        Attribute {
            name: "Param_JoinSecret".to_string(),
            value: Variant {
                blob_value: Some(join_secret.to_vec()),
                ..Default::default()
            },
        },
        Attribute {
            name: "Param_RealmJoinTicket".to_string(),
            value: Variant {
                blob_value: Some(ticket.into_bytes()),
                ..Default::default()
            },
        },
    ];

    if let Some(key) = &session.session_key {
        attributes.push(Attribute {
            name: "Param_BnetSessionKey".to_string(),
            value: Variant {
                blob_value: Some(key.to_vec()),
                ..Default::default()
            },
        });
    }

    info!(
        session_id = %session.id,
        realm_id = %realm.id,
        realm_address,
        "RealmJoinRequest answered"
    );

    build_response(
        header,
        &ClientResponse {
            attribute: attributes,
        },
    )
}

/// Compress a `"<TypeName>:<json>"` payload into the realm-list blob format.
fn zlib_blob(json_payload: &str) -> anyhow::Result<Vec<u8>> {
    let mut uncompressed = json_payload.as_bytes().to_vec();
    uncompressed.push(0);
    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
    std::io::Write::write_all(&mut encoder, &uncompressed)?;
    let compressed = encoder.finish()?;
    let mut blob = Vec::with_capacity(4 + compressed.len());
    blob.extend_from_slice(&(uncompressed.len() as u32).to_le_bytes());
    blob.extend_from_slice(&compressed);
    Ok(blob)
}

fn build_response(header: &Header, response: &ClientResponse) -> anyhow::Result<Vec<Bytes>> {
    let resp_header = Header {
        service_id: header.service_id,
        method_id: header.method_id,
        token: header.token,
        service_hash: header.service_hash,
        is_response: Some(true),
        status: Some(0),
        ..Default::default()
    };
    let frame = serialize_frame(&resp_header, Some(&response.encode_to_vec()))?;
    Ok(vec![frame])
}

fn build_empty_response(header: &Header) -> anyhow::Result<Bytes> {
    let response = ClientResponse::default();
    let resp_header = Header {
        service_id: header.service_id,
        method_id: header.method_id,
        token: header.token,
        service_hash: header.service_hash,
        is_response: Some(true),
        status: Some(0),
        ..Default::default()
    };
    serialize_frame(&resp_header, Some(&response.encode_to_vec())).map_err(Into::into)
}

fn build_error_response(header: &Header) -> anyhow::Result<Bytes> {
    let resp_header = Header {
        service_id: header.service_id,
        method_id: header.method_id,
        token: header.token,
        service_hash: header.service_hash,
        is_response: Some(true),
        status: Some(1),
        ..Default::default()
    };
    serialize_frame(&resp_header, None).map_err(Into::into)
}

#[cfg(test)]
mod tests {
    use super::*;
    use flate2::read::ZlibDecoder;

    fn decompress_blob(blob: &[u8]) -> String {
        let len = u32::from_le_bytes(blob[..4].try_into().unwrap()) as usize;
        let mut decoder = ZlibDecoder::new(&blob[4..]);
        let mut decompressed = Vec::new();
        std::io::Read::read_to_end(&mut decoder, &mut decompressed).unwrap();
        assert_eq!(decompressed.len(), len);
        assert_eq!(decompressed.pop(), Some(0));
        String::from_utf8(decompressed).unwrap()
    }

    #[test]
    fn realmforge_projection_uses_registry_identity() {
        let registry = RealmRegistry::default();
        let (realm, version) = registry.advertised_realms(Some(31_650))[0];
        let json = realm_update_json(realm, version);

        assert_eq!(json["update"]["name"], "RealmForge");
        assert_eq!(json["update"]["wowRealmAddress"], 0x0101_0100u32);
        assert_eq!(json["update"]["version"]["versionBuild"], 31_650);
    }

    #[test]
    fn unknown_build_is_not_silently_advertised() {
        let registry = RealmRegistry::default();
        assert!(registry.advertised_realms(Some(99_999)).is_empty());
    }

    #[test]
    fn zlib_round_trip() {
        let blob = zlib_blob("JSONRealmListUpdates:{\"test\":true}").unwrap();
        assert_eq!(
            decompress_blob(&blob),
            "JSONRealmListUpdates:{\"test\":true}"
        );
    }

    #[test]
    fn realm_join_response_carries_all_params() {
        let registry = RealmRegistry::default();
        let mut session = super::super::BgsSession::new("test-session".to_string());
        session._account_id = Some(42);
        session.session_key = Some([7u8; 64]);
        session.build = Some(31_650);

        let request = ClientRequest {
            attribute: vec![Attribute {
                name: "Param_RealmAddress".to_string(),
                value: Variant {
                    uint_value: Some(0x0101_0100),
                    ..Default::default()
                },
            }],
            ..Default::default()
        };
        let header = Header {
            service_id: 1,
            method_id: Some(1),
            token: 7,
            service_hash: Some(realmforge_gate_bgs::service_hash::GAME_UTILITIES_V1),
            ..Default::default()
        };

        let frames = handle_realm_join_request(&registry, &session, &header, &request).unwrap();
        assert_eq!(frames.len(), 1);

        let frame = realmforge_gate_bgs::frame::parse_frame(&mut prost::bytes::BytesMut::from(
            frames[0].as_ref(),
        ))
        .expect("frame parses")
        .expect("complete frame");
        let response = ClientResponse::decode(frame.body.unwrap()).unwrap();

        let mut by_name = std::collections::HashMap::new();
        for attr in &response.attribute {
            by_name.insert(
                attr.name.as_str(),
                attr.value.blob_value.as_deref().unwrap_or(&[]),
            );
        }

        let addresses = decompress_blob(by_name["Param_ServerAddresses"]);
        assert!(addresses.starts_with("JSONRealmListServerIPAddresses:"));
        let json_text = addresses.trim_start_matches("JSONRealmListServerIPAddresses:");
        let json: serde_json::Value = serde_json::from_str(json_text).unwrap();
        assert_eq!(json["families"][0]["family"], 1);
        assert_eq!(json["families"][0]["addresses"][0]["ip"], "127.0.0.1");
        assert_eq!(json["families"][0]["addresses"][0]["port"], 8085);
        assert_eq!(by_name["Param_JoinSecret"].len(), 32);
        assert_eq!(by_name["Param_RealmJoinTicket"], b"42");
        assert_eq!(by_name["Param_BnetSessionKey"], &[7u8; 64]);
    }

    #[test]
    fn realm_join_rejects_unknown_realm() {
        let registry = RealmRegistry::default();
        let mut session = super::super::BgsSession::new("test-session".to_string());
        session.build = Some(31_650);
        let request = ClientRequest {
            attribute: vec![Attribute {
                name: "Param_RealmAddress".to_string(),
                value: Variant {
                    uint_value: Some(0xFFFF_FFFF),
                    ..Default::default()
                },
            }],
            ..Default::default()
        };
        let header = Header {
            service_id: 1,
            method_id: Some(1),
            token: 7,
            service_hash: Some(realmforge_gate_bgs::service_hash::GAME_UTILITIES_V1),
            ..Default::default()
        };
        let frames = handle_realm_join_request(&registry, &session, &header, &request).unwrap();
        let frame = realmforge_gate_bgs::frame::parse_frame(&mut prost::bytes::BytesMut::from(
            frames[0].as_ref(),
        ))
        .expect("frame parses")
        .expect("complete frame");
        assert!(frame.header.is_response.unwrap_or(false));
        assert_eq!(frame.header.status, Some(1));
        assert!(frame.body.is_none());
    }
}
