// SPDX-License-Identifier: AGPL-3.0-only

//! Bnetserver JSON types — matching `Battlenet::JSON::Login` protobuf-to-JSON.
//!
//! The reference implementation serializes protobuf messages to JSON using the
//! raw proto field names as keys (snake_case, e.g. `public_B`,
//! `authentication_state`) and enum values as their string names (e.g.
//! `"LOGIN"`, `"DONE"`). These serde structs match that format byte-for-byte,
//! mirroring the `Battlenet::JSON::Login::LoginForm` proto definition.

use serde::{Deserialize, Serialize};

/// A single input value submitted by the client (e.g. `account_name`, `password`,
/// `public_A`, `client_evidence_M1`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormInputValue {
    pub input_id: String,
    pub value: String,
}

/// The login form submitted by the client over `POST /bnetserver/login/` and
/// `POST /bnetserver/login/srp/`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginForm {
    #[serde(default)]
    pub platform_id: String,
    #[serde(default)]
    pub program_id: String,
    #[serde(default)]
    pub version: String,
    pub inputs: Vec<FormInputValue>,
}

impl LoginForm {
    /// Look up an input value by its `input_id`.
    pub fn input(&self, id: &str) -> Option<&str> {
        self.inputs
            .iter()
            .find(|i| i.input_id == id)
            .map(|i| i.value.as_str())
    }
}

/// The form definition returned by `GET /bnetserver/login/`. Tells the client
/// which inputs are required and where the SRP endpoint is.
#[derive(Debug, Serialize)]
pub struct FormInputs {
    #[serde(rename = "type")]
    pub form_type: String,
    pub inputs: Vec<FormInputDefinition>,
    pub srp_url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub srp_js: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct FormInputDefinition {
    pub input_id: String,
    #[serde(rename = "type")]
    pub input_type: String,
    pub label: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_length: Option<u32>,
}

/// The SRP challenge returned by `POST /bnetserver/login/srp/`.
#[derive(Debug, Serialize)]
pub struct SrpLoginChallenge {
    pub version: u32,
    pub iterations: u32,
    pub modulus: String,
    pub generator: String,
    pub hash_function: String,
    pub username: String,
    pub salt: String,
    #[serde(rename = "public_B")]
    pub public_b: String,
}

/// The result of `POST /bnetserver/login/`.
#[derive(Debug, Serialize)]
pub struct LoginResult {
    pub authentication_state: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub login_ticket: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "server_evidence_M2")]
    pub server_evidence_m2: Option<String>,
}

impl LoginResult {
    pub fn done(login_ticket: String) -> Self {
        Self {
            authentication_state: "DONE".to_string(),
            error_code: None,
            error_message: None,
            login_ticket: Some(login_ticket),
            server_evidence_m2: None,
        }
    }

    pub fn login() -> Self {
        Self {
            authentication_state: "LOGIN".to_string(),
            error_code: None,
            error_message: None,
            login_ticket: None,
            server_evidence_m2: None,
        }
    }

    pub fn error(code: &str, message: &str) -> Self {
        Self {
            authentication_state: "LOGIN".to_string(),
            error_code: Some(code.to_string()),
            error_message: Some(message.to_string()),
            login_ticket: None,
            server_evidence_m2: None,
        }
    }
}

/// `POST /client/login/external` response for WoW Classic 1.13.2.
///
/// The 1.13.2 client expects `auth.permit` and `remember.auth.permit` fields
/// for session persistence. The `server_evidence_M2` field is not present because
/// 1.13.2 does not use SRP.
#[derive(Debug, Serialize)]
pub struct ClientLoginResponse {
    pub authentication_state: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub login_ticket: Option<String>,
    #[serde(rename = "auth.permit")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auth_permit: Option<String>,
    #[serde(rename = "remember.auth.permit")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remember_auth_permit: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
}

/// `POST /bnetserver/refreshLoginTicket/` response.
#[derive(Debug, Serialize)]
pub struct LoginRefreshResult {
    pub login_ticket_expiry: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_expired: Option<bool>,
}

/// A game account in the `POST /bnetserver/gameAccounts/` response.
#[derive(Debug, Serialize)]
pub struct GameAccountInfo {
    pub display_name: String,
    pub expansion: u32,
}

#[derive(Debug, Serialize)]
pub struct GameAccountList {
    pub game_accounts: Vec<GameAccountInfo>,
}
