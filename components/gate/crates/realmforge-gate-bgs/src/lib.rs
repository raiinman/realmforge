// SPDX-License-Identifier: AGPL-3.0-only

//! BGS WebSocket RPC transport — protobuf frame parsing and service routing.

pub mod frame;
pub mod method_id;
pub mod result_code;
pub mod service_hash;

// Generated protobuf code.
include!(concat!(env!("OUT_DIR"), "/bgs.protocol.rs"));
