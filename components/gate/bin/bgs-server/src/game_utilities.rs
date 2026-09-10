// SPDX-License-Identifier: AGPL-3.0-only

//! `GameUtilities::ProcessClientRequest` handler for the realm-list flow.
//!
//! The WoW Classic 1.13.2 / 1.14.0 clients send `Command_*_v1` attributes
//! inside `ClientRequest` messages to negotiate realm-list access (the
//! command set is identical in both builds; 1.14.0/40618 doc:
//! gameutilities-realmlist-flow.md). This module implements the full
//! pre-realm-join handoff:
//!
//! 1. `Command_RealmListTicketRequest_v1` — issues an opaque realm-list
//!    ticket. The client sends `Param_Identity` + `Param_ClientInfo`.
//! 2. `Command_RealmListRequest_v1` — returns the realm catalog as
//!    zlib-compressed `JSONRealmListUpdates:` JSON + character counts.
//! 3. `Command_LastCharPlayedRequest_v1` / `Command_CharacterListRequest_v1`
//!    — empty stubs (no characters yet).
//! 4. `Command_RealmJoinRequest_v1` — returns the world-server connection
//!    parameters: `Param_ServerAddresses` (zlib JSON), `Param_JoinSecret`
//!    (32 bytes), `Param_RealmJoinTicket`, `Param_BnetSessionKey`. Tavern
//!    stops at this handoff; the realm server is an external integration.
//!
//! The carrier protos (`ClientRequest`, `ClientResponse`, `Attribute`,
//! `Variant`) are identical across BGS v1 and v2, so this handler is
//! version-agnostic. Wire shapes cross-referenced against
//! protobuf-decompiler/output/1.13.2.31650/bgs/low/pb/client/
//! (game_utilities_service.proto, attribute_types.proto).

use std::sync::Arc;

use flate2::Compression;
use flate2::write::ZlibEncoder;
use prost::Message as _;
use rand::RngCore;
use tavern_bgs::frame::serialize_frame;
use tavern_bgs::{Attribute, ClientRequest, ClientResponse, Header, Variant};
use tracing::{info, warn};

use prost::bytes::Bytes;

/// The single realm tavern advertises. Encoded as
/// `(region << 24) | (site << 16) | (realm << 8) | flags` — region 1
/// (US), site 1, realm 1, no flags. Must match `wowRealmAddress` in the
/// realm-list JSON.
const REALM_ADDRESS: u32 = 0x0101_0100;

/// Build a response to `GameUtilities::ProcessClientRequest`.
///
/// Parses the `ClientRequest` attributes, routes to the matching command
/// handler, and returns zero or more response frames.
pub async fn handle_process_client_request(
    state: &Arc<super::BgsState>,
    session: &mut super::BgsSession,
    header: &Header,
    body: &[u8],
) -> anyhow::Result<Vec<Bytes>> {
    let sid = &session.id;
    let request = ClientRequest::decode(body)?;

    // Find the first `Command_*_v1` attribute.
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
        "Command_RealmListRequest_v1" => handle_realm_list_request(session, header, &request),
        "Command_LastCharPlayedRequest_v1" => {
            info!(session_id = %sid, "LastCharPlayed — returning empty");
            Ok(vec![build_empty_response(header)?])
        }
        "Command_RealmJoinRequest_v1" => {
            handle_realm_join_request(&state.realm_ip, state.realm_port, session, header, &request)
        }
        "Command_CharacterListRequest_v1" => {
            info!(session_id = %sid, "CharacterList — returning empty");
            Ok(vec![build_empty_response(header)?])
        }
        _ => {
            info!(
                session_id = %sid,
                command_name, "unknown command — returning empty"
            );
            Ok(vec![build_empty_response(header)?])
        }
    }
}

/// Handle `Command_RealmListTicketRequest_v1`.
///
/// Parses `Param_Identity` (JSON-encoded identity blob) and `Param_ClientInfo`
/// (JSON-encoded client info with `secret`) and returns `Param_RealmListTicket`
/// as an opaque string blob. TrinityCore uses the literal string
/// "AuthRealmListTicket"; the 1.13.2 client accepts it.
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

/// Handle `Command_RealmListRequest_v1`.
///
/// Returns `Param_RealmList` as zlib-compressed JSON containing the realm
/// catalog, plus `Param_CharacterCountList` (empty counts).
///
/// The JSON uses the canonical `JSON.RealmList` proto field names
/// (`wowRealmAddress`, `cfgTimezonesID`, `version.versionBuild`, `name`, …)
/// — the client deserializes with protoc-gen-json field-name matching
/// (realmlist-flow.md; reference RealmList.proto). The realm version is
/// served per client build: 1.14.0/40618 clients get the 1.14.0 version
/// tuple, everything else the 1.13.2 one, so the realm never shows a
/// version mismatch.
fn handle_realm_list_request(
    session: &super::BgsSession,
    header: &Header,
    _request: &ClientRequest,
) -> anyhow::Result<Vec<Bytes>> {
    info!(session_id = %session.id, "RealmListRequest");

    let (major, minor, revision, build) = realm_version(session.build);

    let realm_list_json = serde_json::json!({
        "updates": [{
            "update": {
                "wowRealmAddress": REALM_ADDRESS,
                "cfgTimezonesID": 1,
                "populationState": 1,
                "cfgCategoriesID": 1,
                "version": {
                    "versionMajor": major,
                    "versionMinor": minor,
                    "versionRevision": revision,
                    "versionBuild": build,
                },
                "cfgRealmsID": 1,
                "flags": 0,
                "name": "Tavern Realm",
                "cfgConfigsID": 1,
                "cfgLanguagesID": 1,
            },
            "deleting": false,
        }]
    });
    let realm_blob = zlib_blob(&format!("JSONRealmListUpdates:{realm_list_json}"))?;

    // Character counts per realm — empty for now (no characters yet).
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

/// Handle `Command_RealmJoinRequest_v1`.
///
/// The client sends `Param_RealmAddress` (uint) naming the realm it wants
/// to join. The server replies with the world-server connection parameters
/// (realmlist-flow.md §RealmJoin + the 1.13.2 realm-join walk-through):
///
/// - `Param_ServerAddresses` — zlib-compressed
///   `JSONRealmListServerIPAddresses:` JSON (proto
///   `JSON.RealmList.RealmListServerIPAddresses`): families → family +
///   addresses[] of {ip, port}, pointing at the configured realm listener.
/// - `Param_JoinSecret` — 32 random bytes; the `serverSecret` half of the
///   1.13.2 world-auth digest key material
///   (`SHA256(clientSecret ‖ serverSecret ‖ osAuthSeed)`).
/// - `Param_RealmJoinTicket` — opaque account handle the client echoes in
///   `CMSG_AUTH_SESSION.RealmJoinTicket`; the realm server resolves it.
/// - `Param_BnetSessionKey` — the BGS session key from VerifyWebCredentials
///   (1.14.0 world auth TLS material).
///
/// The client reads named params, so unknown extras are ignored; the set
/// above covers both builds (1.14.0 never uses `JoinSecret`, per the
/// 1.14.0 param table).
fn handle_realm_join_request(
    realm_ip: &str,
    realm_port: u16,
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

    if realm_address != u64::from(REALM_ADDRESS) {
        warn!(
            session_id = %session.id,
            realm_address, "RealmJoinRequest for unknown realm"
        );
        return Ok(vec![build_error_response(header)?]);
    }

    let family = if realm_ip.contains(':') { 2 } else { 1 };
    let addresses_json = serde_json::json!({
        "families": [{
            "family": family,
            "addresses": [{ "ip": realm_ip, "port": realm_port }]
        }]
    });
    let addresses_blob = zlib_blob(&format!("JSONRealmListServerIPAddresses:{addresses_json}"))?;

    // JoinSecret: 32 random bytes (serverSecret). Random per join; the
    // realm server shares it with the client's digest computation.
    let mut join_secret = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut join_secret);

    // RealmJoinTicket: the account id as an opaque string. The realm
    // server looks the account up by this handle.
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

    info!(session_id = %session.id, realm_address, "RealmJoinRequest answered");
    build_response(
        header,
        &ClientResponse {
            attribute: attributes,
        },
    )
}

/// Version tuple for the advertised realm, per client build.
///
/// 1.14.0/40618 is a Shadowlands rebase (major/minor/revision/build
/// 1/14/0/40618); every other build gets the 1.13.2 tuple with the
/// client's own build number so the realm never shows a version mismatch.
fn realm_version(build: Option<i32>) -> (u32, u32, u32, u32) {
    match build {
        Some(40618) => (1, 14, 0, 40618),
        Some(b) => (1, 13, 2, b as u32),
        None => (1, 13, 2, 31650),
    }
}

/// Compress a `"<TypeName>:<json>"` payload into the realm-list blob
/// format: 4-byte LE uncompressed length + deflate stream, matching the
/// TC Classic reference (`uint32(json.length() + 1)` prefix, NUL included
/// in the compressed payload).
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

/// Build a response frame for a `ClientResponse`.
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

/// Build an empty success response frame.
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

/// Build an error response frame (non-zero status, empty body).
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
        assert_eq!(
            decompressed.len(),
            len,
            "prefix must match decompressed size"
        );
        // Trailing NUL included per TC reference.
        assert_eq!(decompressed.pop(), Some(0));
        String::from_utf8(decompressed).unwrap()
    }

    #[test]
    fn realm_list_json_is_valid() {
        let json = serde_json::json!({
            "updates": [{
                "update": {
                    "wowRealmAddress": REALM_ADDRESS,
                    "cfgTimezonesID": 1,
                    "populationState": 1,
                    "cfgCategoriesID": 1,
                    "version": {
                        "versionMajor": 1, "versionMinor": 13,
                        "versionRevision": 2, "versionBuild": 31650
                    },
                    "cfgRealmsID": 1,
                    "flags": 0,
                    "name": "Tavern Realm",
                    "cfgConfigsID": 1,
                    "cfgLanguagesID": 1,
                },
                "deleting": false,
            }]
        });
        let s = serde_json::to_string(&json).unwrap();
        assert!(s.contains("Tavern Realm"));
        assert!(s.contains("31650"));
        // Canonical proto field names (protoc-gen-json compatible).
        assert!(s.contains("wowRealmAddress"));
        assert!(s.contains("versionBuild"));
        assert!(!s.contains("realmName"));
        assert!(!s.contains("enUS"));
    }

    #[test]
    fn realm_version_maps_builds() {
        assert_eq!(realm_version(Some(31650)), (1, 13, 2, 31650));
        assert_eq!(realm_version(Some(40618)), (1, 14, 0, 40618));
        assert_eq!(realm_version(Some(12345)), (1, 13, 2, 12345));
        assert_eq!(realm_version(None), (1, 13, 2, 31650));
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
        let mut session = super::super::BgsSession::new("test-session".to_string());
        session._account_id = Some(42);
        session.session_key = Some([7u8; 64]);

        let request = ClientRequest {
            attribute: vec![Attribute {
                name: "Param_RealmAddress".to_string(),
                value: Variant {
                    uint_value: Some(u64::from(REALM_ADDRESS)),
                    ..Default::default()
                },
            }],
            ..Default::default()
        };
        let header = Header {
            service_id: 1,
            method_id: Some(1),
            token: 7,
            service_hash: Some(tavern_bgs::service_hash::GAME_UTILITIES_V1),
            ..Default::default()
        };

        let frames =
            handle_realm_join_request("127.0.0.1", 8085, &session, &header, &request).unwrap();
        assert_eq!(frames.len(), 1);

        // Re-parse the frame body and inspect the attributes.
        let frame =
            tavern_bgs::frame::parse_frame(&mut prost::bytes::BytesMut::from(frames[0].as_ref()))
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

        // ServerAddresses decompresses to a valid JSONRealmListServerIPAddresses doc.
        let addresses = decompress_blob(by_name["Param_ServerAddresses"]);
        assert!(addresses.starts_with("JSONRealmListServerIPAddresses:"));
        let json_text = addresses.trim_start_matches("JSONRealmListServerIPAddresses:");
        let json: serde_json::Value = serde_json::from_str(json_text).unwrap();
        assert_eq!(json["families"][0]["family"], 1);
        assert_eq!(json["families"][0]["addresses"][0]["ip"], "127.0.0.1");
        assert_eq!(json["families"][0]["addresses"][0]["port"], 8085);

        // JoinSecret is exactly 32 bytes.
        assert_eq!(by_name["Param_JoinSecret"].len(), 32);

        // RealmJoinTicket echoes the account id.
        assert_eq!(by_name["Param_RealmJoinTicket"], b"42");

        // BnetSessionKey is the session key.
        assert_eq!(by_name["Param_BnetSessionKey"], &[7u8; 64]);
    }

    #[test]
    fn realm_join_rejects_unknown_realm() {
        let session = super::super::BgsSession::new("test-session".to_string());
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
            service_hash: Some(tavern_bgs::service_hash::GAME_UTILITIES_V1),
            ..Default::default()
        };
        let frames =
            handle_realm_join_request("127.0.0.1", 8085, &session, &header, &request).unwrap();
        let frame =
            tavern_bgs::frame::parse_frame(&mut prost::bytes::BytesMut::from(frames[0].as_ref()))
                .expect("frame parses")
                .expect("complete frame");
        assert!(frame.header.is_response.unwrap_or(false));
        assert_eq!(frame.header.status, Some(1));
        assert!(frame.body.is_none());
    }
}
