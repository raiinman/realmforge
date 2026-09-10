// SPDX-License-Identifier: AGPL-3.0-only

//! BGS WebSocket server — accepts WSS connections with subprotocol
//! `v1.rpc.battle.net`, dispatches RPC frames to service handlers.
//!
//! Milestone 17: transport + ConnectionService.Connect
//! Milestone 18: AuthenticationService, GameUtilitiesService, AccountService

mod game_utilities;
mod realmforge_config;
mod realmforge_realms;
mod tcp_transport;

use std::sync::Arc;

use axum::Router;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::routing::get;
use futures_util::{SinkExt, StreamExt};
use prost::Message as _;
use prost::bytes::{Bytes, BytesMut};
use rand::RngCore as _;
use realmforge_gate_bgs::frame::{parse_frame, serialize_frame};
use realmforge_gate_bgs::method_id;
use realmforge_gate_bgs::service_hash;
use realmforge_gate_bgs::{
    AccountLevelInfo, AccountState, ConnectRequest, ConnectResponse, CreateSessionRequest,
    CreateSessionResponse, DestroySessionRequest, DisconnectNotification, GameAccountHandle,
    GameAccountList, GameAccountState, GameLevelInfo, GameSessionInfo, GameStatus, GameTimeInfo,
    GetAccountStateResponse, GetGameAccountStateResponse, GetGameTimeRemainingInfoResponse,
    GetLicensesResponse, Header, LogonQueueUpdateRequest, LogonRequest, LogonResponse3,
    LogonResult, ProcessId, ResolveAccountResponse, SessionId, SessionVariables,
    VerifyWebCredentialsRequest,
};
use tokio::net::TcpListener;
use tracing::{info, warn};

static AUTH_SUCCESS: std::sync::OnceLock<opentelemetry::metrics::Counter<u64>> =
    std::sync::OnceLock::new();
static AUTH_FAILURE: std::sync::OnceLock<opentelemetry::metrics::Counter<u64>> =
    std::sync::OnceLock::new();
static BUILD_COUNTER: std::sync::OnceLock<opentelemetry::metrics::Counter<u64>> =
    std::sync::OnceLock::new();
static KICKED_COUNTER: std::sync::OnceLock<opentelemetry::metrics::Counter<u64>> =
    std::sync::OnceLock::new();
static LICENSE_COUNTER: std::sync::OnceLock<opentelemetry::metrics::Counter<u64>> =
    std::sync::OnceLock::new();
static GAME_TIME_HISTOGRAM: std::sync::OnceLock<opentelemetry::metrics::Histogram<f64>> =
    std::sync::OnceLock::new();
static STATUS_COUNTER: std::sync::OnceLock<opentelemetry::metrics::Counter<u64>> =
    std::sync::OnceLock::new();
/// Per-game-account state loaded from the database at auth time.
#[derive(Debug, Clone)]
pub struct GameAccountInfo {
    pub id: i64,
    pub name: String,
    pub region: i16,
    pub is_suspended: bool,
    pub is_banned: bool,
    pub suspension_expires: Option<i64>,
    pub game_time_expires: Option<i64>,
}

/// Per-client BGS session state.
#[derive(Debug)]
pub struct BgsSession {
    pub id: String,
    pub client_id: Option<String>,
    pub authenticated: bool,
    pub _account_id: Option<u64>,
    pub battle_tag: Option<String>,
    pub game_accounts: Vec<realmforge_gate_bgs::GameAccountHandle>,
    pub game_account_info: Vec<GameAccountInfo>,
    pub licenses: Vec<realmforge_gate_bgs::AccountLicense>,
    pub per_game_account_licenses:
        std::collections::HashMap<i64, Vec<realmforge_gate_bgs::AccountLicense>>,
    pub platform: Option<String>,
    pub locale: Option<String>,
    pub build: Option<i32>,
    pub queued: bool,
    pub protocol_version: Option<u8>,
    /// BGS session key (64 bytes) generated at VerifyWebCredentials. Sent
    /// back to the client as `Param_BnetSessionKey` in the realm-join
    /// response (game-server TLS material for the world handshake).
    pub session_key: Option<[u8; 64]>,
}

impl BgsSession {
    fn new(id: String) -> Self {
        Self {
            id,
            client_id: None,
            authenticated: false,
            _account_id: None,
            battle_tag: None,
            game_accounts: Vec::new(),
            game_account_info: Vec::new(),
            licenses: Vec::new(),
            per_game_account_licenses: std::collections::HashMap::new(),
            platform: None,
            locale: None,
            build: None,
            queued: false,
            protocol_version: None,
            session_key: None,
        }
    }
}

/// Shared BGS server state.
/// An entry in the login queue.
pub struct QueueEntry {
    push_tx: tokio::sync::mpsc::UnboundedSender<prost::bytes::Bytes>,
    on_logon_complete: Option<prost::bytes::Bytes>,
    protocol_version: u8,
    /// SSO token for RestoreSession dequeues (TODO: complete in dequeue_next).
    #[allow(dead_code)]
    sso_id: Option<Vec<u8>>,
}

pub struct BgsState {
    pub db: sqlx::PgPool,
    pub active_logins: std::sync::atomic::AtomicU64,
    pub max_logins: u64,
    pub login_queue: std::sync::Mutex<std::collections::VecDeque<QueueEntry>>,
    pub push_channels:
        dashmap::DashMap<String, tokio::sync::mpsc::UnboundedSender<prost::bytes::Bytes>>,
    /// active_accounts: account_id → session_id. Enforces one session per account.
    pub active_accounts: dashmap::DashMap<u64, String>,
    /// Gate-side projection of Realmforge's realm registry. This is a
    /// temporary compatibility boundary until Core supplies the registry over
    /// an explicit service contract; it is not the canonical product model.
    pub realm_registry: realmforge_realms::RealmRegistry,
}

/// Build a rustls server config from a PEM certificate chain and private key.
///
/// Used for the 1.13.2 TCP listener (BGS_TLS_CERT / BGS_TLS_KEY); the
/// real client dials the BGS port over TLS, mirroring TrinityCore's
/// bnetserver. Left unset, the listener stays plaintext for the synthetic
/// test clients.
fn load_tls_server_config(cert_path: &str, key_path: &str) -> anyhow::Result<rustls::ServerConfig> {
    use std::io::BufReader;

    let certs = rustls_pemfile::certs(&mut BufReader::new(std::fs::File::open(cert_path)?))
        .collect::<Result<Vec<_>, _>>()?;
    let key = rustls_pemfile::private_key(&mut BufReader::new(std::fs::File::open(key_path)?))?
        .ok_or_else(|| anyhow::anyhow!("no private key found in {key_path}"))?;
    Ok(rustls::ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(certs, key)?)
}

/// The WebSocket upgrade handler.
async fn ws_handler(
    axum::extract::State(app_state): axum::extract::State<Arc<BgsState>>,
    ws: WebSocketUpgrade,
) -> axum::response::Response {
    ws.protocols(["v1.rpc.battle.net"])
        .on_upgrade(move |socket| handle_connection(socket, app_state))
}

/// Handle an individual BGS WebSocket connection.
async fn handle_connection(ws: WebSocket, state: Arc<BgsState>) {
    let session_id = uuid::Uuid::new_v4().to_string();
    let mut session = BgsSession::new(session_id.clone());
    let (mut sender, mut receiver) = ws.split();
    info!(session_id, "BGS connection established");

    let (push_tx, mut push_rx) = tokio::sync::mpsc::unbounded_channel();
    state.push_channels.insert(session_id.clone(), push_tx);

    let mut read_buf = BytesMut::new();

    loop {
        tokio::select! {
            msg = receiver.next() => {
                match msg {
                    Some(Ok(Message::Binary(data))) => {
                        read_buf.extend_from_slice(&data);
                        loop {
                            match parse_frame(&mut read_buf) {
                                Ok(Some(frame)) => {
                                    let responses = dispatch_frame(
                                        &state,
                                        &mut session,
                                        &frame.header,
                                        frame.body.as_deref(),
                                    )
                                    .await;

                                    match responses {
                                        Ok(frames) => {
                                            for fb in frames {
                                                if let Err(e) =
                                                    sender.send(Message::Binary(fb)).await
                                                {
                                                    warn!(session_id, "WS send error: {e}");
                                                    break;
                                                }
                                            }
                                        }
                                        Err(e) => {
                                            warn!(session_id, "frame dispatch error: {e}");
                                        }
                                    }
                                }
                                Ok(None) => break,
                                Err(e) => {
                                    warn!("invalid frame: {e}");
                                    read_buf.clear();
                                    break;
                                }
                            }
                        }
                    }
                    Some(Ok(Message::Close(_))) => break,
                    Some(Ok(Message::Ping(data))) => { let _ = data; }
                    Some(Err(e)) => {
                        warn!("WebSocket error: {e}");
                        break;
                    }
                    None => break,
                    _ => {}
                }
            }
            frame = push_rx.recv() => {
                match frame {
                    Some(data) => {
                        if let Err(e) = sender.send(Message::Binary(data)).await {
                            warn!(session_id, "push send error: {e}");
                            break;
                        }
                    }
                    None => break,
                }
            }
        }
    }

    state.push_channels.remove(&session_id);
    // Remove from active_accounts if this was the active session for an account.
    if let Some(aid) = session._account_id {
        state
            .active_accounts
            .remove_if(&aid, |_, sid| sid == &session_id);
    }
    // Decrement the active login counter. Use a saturating decrement so an
    // uncounted connection (e.g. a restore-session or queued connection that
    // never incremented) cannot wrap the counter to u64::MAX and wedge the
    // login queue.
    state
        .active_logins
        .fetch_update(
            std::sync::atomic::Ordering::AcqRel,
            std::sync::atomic::Ordering::Acquire,
            |v| Some(v.saturating_sub(1)),
        )
        .unwrap_or_default();
    dequeue_next(&state);
    info!(session_id, "BGS connection ended");
}

/// Dispatch an incoming RPC frame to the appropriate service handler.
///
/// Returns zero or more response frame bytes to write to the transport.
/// Most handlers return a single response frame. The authentication push
/// path (VerifyWebCredentials → OnLogonComplete) returns two frames:
/// the NoData response to the request, plus the OnLogonComplete push.
pub async fn dispatch_frame(
    state: &Arc<BgsState>,
    session: &mut BgsSession,
    header: &Header,
    body: Option<&[u8]>,
) -> anyhow::Result<Vec<Bytes>> {
    let service_hash = header.service_hash.unwrap_or(0);
    let method_id = header.method_id.unwrap_or(0);

    // Protocol version detection: v2 uses different service hashes.
    if session.protocol_version.is_none() {
        if service_hash == service_hash::AUTHENTICATION_SERVICE_V2
            || service_hash == service_hash::AUTHENTICATION_LISTENER_V2
            || service_hash == service_hash::SESSION_SERVICE_V2
        {
            session.protocol_version = Some(2);
        } else if service_hash == service_hash::AUTHENTICATION_SERVER_V1 {
            session.protocol_version = Some(1);
        }
    }

    match service_hash {
        // ConnectionService — same hash in v1 and v2.
        service_hash::CONNECTION_SERVICE => {
            handle_connection_service(state, session, header, method_id, body).await
        }
        // Authentication — v1 (game client) and v2 (desktop app).
        service_hash::AUTHENTICATION_SERVER_V1
        | service_hash::AUTHENTICATION_SERVICE
        | service_hash::AUTHENTICATION_SERVICE_V2 => {
            handle_authentication_service(state, session, header, method_id, body).await
        }
        // GameUtilities — v1 (game client) and v2 (desktop app).
        service_hash::GAME_UTILITIES_V1 | service_hash::GAME_UTILITIES_SERVICE => {
            handle_game_utilities(state, session, header, method_id, body).await
        }
        // AccountService — same hash in v1 and v2.
        service_hash::ACCOUNT_SERVICE => {
            handle_account_service(state, session, header, method_id, body).await
        }
        // SessionService — v2 only (desktop app).
        service_hash::SESSION_SERVICE | service_hash::SESSION_SERVICE_V2 => {
            handle_session_service(state, session, header, method_id, body).await
        }
        // ChallengeNotify — S→C listener, no C→S messages expected.
        service_hash::CHALLENGE_NOTIFY_V1 => {
            info!(
                session_id = %session.id,
                method_id, "ChallengeNotify (listener, ignoring)"
            );
            Ok(Vec::new())
        }
        // AccountNotify — S→C listener for account state updates.
        // The server pushes OnAccountStateUpdated when subscribed.
        service_hash::ACCOUNT_NOTIFY_V1 => {
            info!(
                session_id = %session.id,
                method_id, "AccountNotify (listener, ignoring)"
            );
            Ok(Vec::new())
        }
        // Known v1 services — not yet implemented, log and ignore.
        //
        // 1.14.0 changes: PresenceService lost Ownership(5) and
        // SubscribeNotification(7). AccountService lost IsIgrAddress(15)
        // and UpdateParentalControlsAndCAIS.
        // GameUtilities lost GetPlayerVariables(3) and
        // GetAchievementsFile(9); gained RegisterUtilities(11) and
        // UnregisterUtilities(12).
        service_hash::USER_MANAGER_SERVICE_V1
        | service_hash::USER_MANAGER_NOTIFY_V1
        | service_hash::FRIENDS_SERVICE_V1
        | service_hash::FRIENDS_NOTIFY_V1
        | service_hash::PRESENCE_SERVICE_V1
        | service_hash::PRESENCE_LISTENER_V1
        | service_hash::REPORT_SERVICE_V1
        | service_hash::RESOURCES_V1
        | service_hash::CLUB_MEMBERSHIP_SERVICE_V1
        | service_hash::CLUB_MEMBERSHIP_LISTENER_V1 => {
            info!(
                session_id = %session.id,
                method_id, service_hash, "known v1 service (not implemented)"
            );
            Ok(Vec::new())
        }
        _ => {
            warn!(service_hash, method_id, "unknown service");
            Ok(Vec::new())
        }
    }
}

// --- ConnectionService ---

async fn handle_connection_service(
    _state: &Arc<BgsState>,
    session: &mut BgsSession,
    header: &Header,
    method_id: u32,
    body: Option<&[u8]>,
) -> anyhow::Result<Vec<Bytes>> {
    if header.is_response.unwrap_or(false) {
        return Ok(Vec::new());
    }

    match method_id {
        method_id::connection::CONNECT => handle_connect(session, header, body).await,
        method_id::connection::ECHO => handle_echo(header, body),
        method_id::connection::KEEP_ALIVE => handle_keep_alive(header),
        method_id::connection::REQUEST_DISCONNECT => handle_keep_alive(header),
        _ => Ok(Vec::new()),
    }
}

// --- ConnectionService handlers ---

async fn handle_connect(
    session: &mut BgsSession,
    header: &Header,
    body: Option<&[u8]>,
) -> anyhow::Result<Vec<Bytes>> {
    let req = if let Some(b) = body {
        ConnectRequest::decode(b)?
    } else {
        ConnectRequest::default()
    };

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();

    let response = ConnectResponse {
        server_id: ProcessId {
            label: 1,
            epoch: now.as_secs() as u32,
        },
        client_id: req.client_id,
        server_time: Some(now.as_millis() as u64),
        use_bindless_rpc: Some(true),
        ..Default::default()
    };

    session.client_id = Some(session.id.clone());

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
    info!(session_id = %session.id, "ConnectResponse");
    Ok(vec![frame])
}

fn handle_echo(header: &Header, body: Option<&[u8]>) -> anyhow::Result<Vec<Bytes>> {
    let resp_header = Header {
        service_id: header.service_id,
        method_id: header.method_id,
        token: header.token,
        service_hash: header.service_hash,
        is_response: Some(true),
        status: Some(0),
        ..Default::default()
    };
    let frame = serialize_frame(&resp_header, body)?;
    Ok(vec![frame])
}

fn handle_keep_alive(header: &Header) -> anyhow::Result<Vec<Bytes>> {
    let resp_header = Header {
        service_id: header.service_id,
        method_id: header.method_id,
        token: header.token,
        service_hash: header.service_hash,
        is_response: Some(true),
        status: Some(0),
        ..Default::default()
    };
    let frame = serialize_frame(&resp_header, None)?;
    Ok(vec![frame])
}

// --- AuthenticationService ---

async fn handle_authentication_service(
    state: &Arc<BgsState>,
    session: &mut BgsSession,
    header: &Header,
    method_id: u32,
    body: Option<&[u8]>,
) -> anyhow::Result<Vec<Bytes>> {
    if header.is_response.unwrap_or(false) {
        return Ok(Vec::new());
    }

    match method_id {
        // Logon (v1:1, v2:1) — lean request, returns NoData.
        method_id::authentication::LOGON => handle_logon(state, session, header, body).await,
        // VerifyWebCredentials (v1:7) / VerifyAuthToken (v2:2).
        m @ (method_id::authentication::VERIFY_CREDENTIALS | 2) => {
            let _ = m;
            handle_verify_web_credentials(state, session, header, body).await
        }
        // GenerateSSOToken (v1:5) / GenerateAuthToken (v2:3).
        m @ (method_id::authentication::GENERATE_TOKEN | 3) => {
            let _ = m;
            handle_generate_sso_token(state, session, header, body).await
        }
        method_id::authentication::GENERATE_WEB_CREDENTIALS => {
            handle_generate_web_credentials(state, session, header, body).await
        }
        method_id::authentication::SELECT_GAME_ACCOUNT => {
            handle_select_game_account(state, session, header, body).await
        }
        method_id::authentication::LOGON_UPDATE => {
            info!(session_id = %session.id, "LogonUpdate (no-op)");
            Ok(Vec::new())
        }
        _ => {
            warn!(method_id, "unknown auth method");
            Ok(Vec::new())
        }
    }
}

async fn handle_logon(
    state: &Arc<BgsState>,
    session: &mut BgsSession,
    header: &Header,
    body: Option<&[u8]>,
) -> anyhow::Result<Vec<Bytes>> {
    let req = if let Some(b) = body {
        LogonRequest::decode(b)?
    } else {
        LogonRequest::default()
    };

    info!(
        session_id = %session.id,
        program = ?req.program,
        platform = ?req.platform,
        locale = ?req.locale,
        email = ?req.email,
        build = req.application_version,
        "LogonRequest (lean, returning NoData)"
    );

    // Validate program (only "WoW" is supported).
    if req.program.as_deref() != Some("WoW") {
        let resp_header = Header {
            service_id: header.service_id,
            method_id: header.method_id,
            token: header.token,
            service_hash: header.service_hash,
            is_response: Some(true),
            status: Some(1),
            ..Default::default()
        };
        let frame = serialize_frame(&resp_header, None)?;
        return Ok(vec![frame]);
    }

    // Validate mandatory client metadata.
    if req.application_version.is_none() || req.platform.is_none() || req.locale.is_none() {
        warn!(
            session_id = %session.id,
            "LogonRequest missing mandatory client metadata"
        );
        let resp_header = Header {
            service_id: header.service_id,
            method_id: header.method_id,
            token: header.token,
            service_hash: header.service_hash,
            is_response: Some(true),
            status: Some(1),
            ..Default::default()
        };
        let frame = serialize_frame(&resp_header, None)?;
        return Ok(vec![frame]);
    }

    // Store the logon metadata in the session for later use by
    // VerifyWebCredentials.
    if let Some(c) = BUILD_COUNTER.get() {
        let b = req.application_version.unwrap_or(0);
        let p = req
            .platform
            .clone()
            .unwrap_or_else(|| "unknown".to_string());
        c.add(
            1,
            &[
                opentelemetry::KeyValue::new("build", b as i64),
                opentelemetry::KeyValue::new("platform", p),
            ],
        );
    }
    session.platform = req.platform;
    session.locale = req.locale;
    session.build = req.application_version;

    // Queue check: if opted in and at capacity, push queue update and enqueue.
    let queued = if req.allow_logon_queue_notifications.unwrap_or(false) {
        let active = state
            .active_logins
            .load(std::sync::atomic::Ordering::Acquire);
        if active >= state.max_logins {
            let push_tx = state
                .push_channels
                .get(&session.id)
                .map(|r| r.value().clone())
                .unwrap();
            let position = {
                let mut q = state.login_queue.lock().unwrap();
                q.push_back(QueueEntry {
                    push_tx,
                    on_logon_complete: None,
                    sso_id: None,
                    protocol_version: session.protocol_version.unwrap_or(1),
                });
                q.len()
            };
            let update = LogonQueueUpdateRequest {
                position: Some(position as u32),
                estimated_time: Some((position * 5) as u64),
                eta_deviation_in_sec: Some(10),
            };
            let (lh, um, _) = queue_push_params(session.protocol_version.unwrap_or(1));
            let push_header = Header {
                service_id: 1,
                method_id: Some(um),
                token: header.token,
                service_hash: Some(lh),
                is_response: Some(false),
                ..Default::default()
            };
            let frame = serialize_frame(&push_header, Some(&update.encode_to_vec()))?;
            // Push queue update through the push channel (server push).
            let _ = state.push_channels.get(&session.id).map(|r| r.send(frame));
            info!(
                session_id = %session.id,
                position,
                "client queued"
            );
            session.queued = true;
            true
        } else {
            state
                .active_logins
                .fetch_add(1, std::sync::atomic::Ordering::AcqRel);
            false
        }
    } else {
        false
    };
    let _ = queued;

    // Always return NoData — the LogonRequest is accepted.
    let resp_header = Header {
        service_id: header.service_id,
        method_id: header.method_id,
        token: header.token,
        service_hash: header.service_hash,
        is_response: Some(true),
        status: Some(0),
        ..Default::default()
    };
    let frame = serialize_frame(&resp_header, None)?;

    Ok(vec![frame])
}

/// If an existing session exists for this account, push a disconnect frame.
fn kick_existing_session(state: &Arc<BgsState>, account_id: u64, new_session_id: &str) {
    if let Some(old) = state
        .active_accounts
        .insert(account_id, new_session_id.to_string())
        .filter(|old| old != new_session_id)
        && let Some(tx) = state.push_channels.get(&old)
    {
        let dc = Header {
            service_id: 1,
            method_id: Some(method_id::connection::FORCE_DISCONNECT),
            token: 0,
            service_hash: Some(realmforge_gate_bgs::service_hash::CONNECTION_SERVICE),
            is_response: Some(false),
            ..Default::default()
        };
        let body = DisconnectNotification {
            error_code: realmforge_gate_bgs::result_code::ERROR_SESSION_DUPLICATE,
            reason: Some("logged in elsewhere".to_string()),
        };
        if let Ok(frame) = serialize_frame(&dc, Some(&body.encode_to_vec())) {
            let _ = tx.send(frame);
            if let Some(c) = KICKED_COUNTER.get() {
                c.add(1, &[]);
            }
        }
    }
}

/// Broadcast Boom to all connected clients during graceful shutdown.
fn broadcast_boom(state: &Arc<BgsState>) {
    let hdr = Header {
        service_id: 1,
        method_id: Some(method_id::connection::FORCE_DISCONNECT),
        token: 0,
        service_hash: Some(realmforge_gate_bgs::service_hash::CONNECTION_SERVICE),
        is_response: Some(false),
        ..Default::default()
    };
    let body = DisconnectNotification {
        error_code: realmforge_gate_bgs::result_code::ERROR_SERVER_SHUTTING_DOWN,
        reason: Some("server shutting down".to_string()),
    };
    if let Ok(frame) = serialize_frame(&hdr, Some(&body.encode_to_vec())) {
        for entry in state.push_channels.iter() {
            let _ = entry.value().send(frame.clone());
        }
    }
}

/// If an existing session exists for this account, push a disconnect frame.
fn dequeue_next(state: &Arc<BgsState>) {
    let entry = {
        let mut q = state.login_queue.lock().unwrap();
        q.pop_front()
    };
    if let Some(entry) = entry {
        let (_, _, em) = queue_push_params(entry.protocol_version);
        let end_header = Header {
            service_id: 1,
            method_id: Some(em),
            token: 0,
            service_hash: Some(realmforge_gate_bgs::service_hash::AUTHENTICATION_CLIENT_V1),
            is_response: Some(false),
            ..Default::default()
        };
        if let Ok(frame) = serialize_frame(&end_header, None) {
            let _ = entry.push_tx.send(frame);
        }
        // Push OnLogonComplete if the client has already verified credentials.
        if let Some(oc) = entry.on_logon_complete {
            state
                .active_logins
                .fetch_add(1, std::sync::atomic::Ordering::AcqRel);
            let _ = entry.push_tx.send(oc);
        }
    }
}

fn queue_push_params(version: u8) -> (u32, u32, u32) {
    if version == 2 {
        (
            realmforge_gate_bgs::service_hash::AUTHENTICATION_LISTENER_V2,
            2,
            3,
        )
    } else {
        (
            realmforge_gate_bgs::service_hash::AUTHENTICATION_CLIENT_V1,
            12,
            13,
        )
    }
}

fn spawn_queue_ticker(state: Arc<BgsState>) {
    tokio::spawn(async move {
        let mut tick = tokio::time::interval(std::time::Duration::from_secs(5));
        loop {
            tick.tick().await;
            let q = state.login_queue.lock().unwrap();
            for (i, entry) in q.iter().enumerate() {
                let update = LogonQueueUpdateRequest {
                    position: Some((i + 1) as u32),
                    estimated_time: Some(((i + 1) * 5) as u64),
                    eta_deviation_in_sec: Some(10),
                };
                let (lh, um, _) = queue_push_params(entry.protocol_version);
                let push_header = Header {
                    service_id: 1,
                    method_id: Some(um),
                    token: 0,
                    service_hash: Some(lh),
                    is_response: Some(false),
                    ..Default::default()
                };
                if let Ok(frame) = serialize_frame(&push_header, Some(&update.encode_to_vec())) {
                    let _ = entry.push_tx.send(frame);
                }
            }
        }
    });
}

/// Handle `VerifyWebCredentials` (method 7): validate the login_ticket bytes
/// and push `OnLogonComplete` on success.
///
/// Returns two frames: the NoData response to VerifyWebCredentials, and the
/// OnLogonComplete push frame.
async fn handle_verify_web_credentials(
    state: &Arc<BgsState>,
    session: &mut BgsSession,
    header: &Header,
    body: Option<&[u8]>,
) -> anyhow::Result<Vec<Bytes>> {
    let req = if let Some(b) = body {
        VerifyWebCredentialsRequest::decode(b)?
    } else {
        warn!(session_id = %session.id, "VerifyWebCredentials with empty body");
        return Ok(Vec::new());
    };

    let web_credentials = req.web_credentials.as_deref().unwrap_or(&[]);

    // Try to decode as a UTF-8 login ticket string.
    let ticket_str = match std::str::from_utf8(web_credentials) {
        Ok(s) => s,
        Err(_) => {
            warn!(
                session_id = %session.id,
                "VerifyWebCredentials: invalid UTF-8 credentials"
            );
            if let Some(c) = AUTH_FAILURE.get() {
                c.add(1, &[]);
            }
            return Ok(Vec::new());
        }
    };

    info!(session_id = %session.id, ticket = ticket_str, "VerifyWebCredentials");

    // Validate the ticket against the service_tickets table.
    let consumed = realmforge_gate_db::service_tickets::consume(&state.db, ticket_str).await?;
    let account_id = match consumed {
        Some(ticket) => {
            info!(
                session_id = %session.id,
                account_id = ticket.account_id,
                "ticket validated"
            );
            if let Some(c) = AUTH_SUCCESS.get() {
                c.add(1, &[]);
            }
            ticket.account_id
        }
        None => {
            warn!(
                session_id = %session.id,
                "VerifyWebCredentials: ticket not found, expired, or already used"
            );
            if let Some(c) = AUTH_FAILURE.get() {
                c.add(1, &[]);
            }
            let err_header = Header {
                service_id: header.service_id,
                method_id: header.method_id,
                token: header.token,
                service_hash: header.service_hash,
                is_response: Some(true),
                status: Some(1),
                ..Default::default()
            };
            let frame = serialize_frame(&err_header, None)?;
            return Ok(vec![frame]);
        }
    };

    // Generate session key and complete the authentication.
    let mut session_key = [0u8; 64];
    rand::thread_rng().fill_bytes(&mut session_key);
    session.session_key = Some(session_key);

    session.authenticated = true;
    session._account_id = Some(account_id as u64);
    kick_existing_session(state, account_id as u64, &session.id);
    // Load game accounts from the database.
    let ga_rows = realmforge_gate_db::game_accounts::find_by_account(&state.db, account_id).await?;

    session.game_accounts = ga_rows
        .iter()
        .map(|r| realmforge_gate_bgs::GameAccountHandle {
            id: r.id as u32,
            program: 0x576f57, // "WoW"
            region: r.region as u32,
        })
        .collect();

    session.game_account_info = ga_rows
        .iter()
        .map(|r| GameAccountInfo {
            id: r.id,
            name: r.name.clone(),
            region: r.region,
            is_suspended: r.is_suspended,
            is_banned: r.is_banned,
            suspension_expires: r.suspension_expires.map(|t| t.timestamp_millis()),
            game_time_expires: r.game_time_expires.map(|t| t.timestamp_millis()),
        })
        .collect();

    // Load licenses: account-level grants plus per-game-account grants.
    let lic_rows =
        realmforge_gate_db::account_licenses::find_by_account(&state.db, account_id).await?;
    let mut per_game: std::collections::HashMap<i64, Vec<realmforge_gate_bgs::AccountLicense>> =
        std::collections::HashMap::new();
    session.licenses = lic_rows
        .into_iter()
        .filter_map(|r| {
            let lic = realmforge_gate_bgs::AccountLicense {
                id: r.license_id as u32,
                expires: None,
            };
            match r.game_account_id {
                Some(ga_id) => {
                    per_game.entry(ga_id).or_default().push(lic);
                    None // game-account grants do not appear in the account list
                }
                None => Some(lic),
            }
        })
        .collect();
    session.per_game_account_licenses = per_game;

    // Record license metrics.
    if let Some(lc) = LICENSE_COUNTER.get() {
        for lic in &session.licenses {
            lc.add(
                1,
                &[opentelemetry::KeyValue::new("license_id", lic.id as i64)],
            );
        }
    }

    // Record game time histogram.
    if let Some(gt) = GAME_TIME_HISTOGRAM.get()
        && let Some(first) = session.game_account_info.first()
        && let Some(expires) = first.game_time_expires
    {
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as i64;
        let remaining_min = ((expires - now_ms).max(0) / 60_000) as f64;
        gt.record(remaining_min, &[]);
    }

    // Record account status metric.
    if let Some(sc) = STATUS_COUNTER.get() {
        let status = if session.game_account_info.iter().any(|g| g.is_banned) {
            "banned"
        } else if session.game_account_info.iter().any(|g| g.is_suspended) {
            "suspended"
        } else if session.game_account_info.iter().any(|g| {
            g.game_time_expires.is_some_and(|exp| {
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis() as i64;
                exp < now
            })
        }) {
            "expired"
        } else {
            "ok"
        };
        sc.add(1, &[opentelemetry::KeyValue::new("status", status)]);
    }

    // Enforce account status: reject banned, suspended, or expired accounts.
    if session.game_account_info.iter().any(|g| g.is_banned) {
        warn!(session_id = %session.id, account_id, "account banned — rejecting login");
        let kick_body = DisconnectNotification {
            error_code: realmforge_gate_bgs::result_code::ERROR_GAME_ACCOUNT_BANNED,
            reason: Some("account banned".to_string()),
        };
        let kick_header = Header {
            service_id: 1,
            method_id: Some(method_id::connection::FORCE_DISCONNECT),
            token: 0,
            service_hash: Some(realmforge_gate_bgs::service_hash::CONNECTION_SERVICE),
            is_response: Some(false),
            ..Default::default()
        };
        let kick_frame = serialize_frame(&kick_header, Some(&kick_body.encode_to_vec()))?;
        return Ok(vec![kick_frame]);
    }

    if session.game_account_info.iter().any(|g| g.is_suspended) {
        warn!(session_id = %session.id, account_id, "account suspended — rejecting login");
        let kick_body = DisconnectNotification {
            error_code: realmforge_gate_bgs::result_code::ERROR_GAME_ACCOUNT_SUSPENDED,
            reason: Some("account suspended".to_string()),
        };
        let kick_header = Header {
            service_id: 1,
            method_id: Some(method_id::connection::FORCE_DISCONNECT),
            token: 0,
            service_hash: Some(realmforge_gate_bgs::service_hash::CONNECTION_SERVICE),
            is_response: Some(false),
            ..Default::default()
        };
        let kick_frame = serialize_frame(&kick_header, Some(&kick_body.encode_to_vec()))?;
        return Ok(vec![kick_frame]);
    }

    // Check if ALL game accounts have expired (no active subscription).
    let all_expired = !session.game_account_info.is_empty()
        && session.game_account_info.iter().all(|g| {
            g.game_time_expires.is_none_or(|exp| {
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis() as i64;
                exp < now
            })
        });
    if all_expired {
        warn!(session_id = %session.id, account_id, "game time expired — rejecting login");
        let kick_body = DisconnectNotification {
            error_code: realmforge_gate_bgs::result_code::ERROR_GAME_ACCOUNT_NO_TIME,
            reason: Some("game time expired".to_string()),
        };
        let kick_header = Header {
            service_id: 1,
            method_id: Some(method_id::connection::FORCE_DISCONNECT),
            token: 0,
            service_hash: Some(realmforge_gate_bgs::service_hash::CONNECTION_SERVICE),
            is_response: Some(false),
            ..Default::default()
        };
        let kick_frame = serialize_frame(&kick_header, Some(&kick_body.encode_to_vec()))?;
        return Ok(vec![kick_frame]);
    }

    // Build the NoData response for VerifyWebCredentials.
    let verify_resp_header = Header {
        service_id: header.service_id,
        method_id: header.method_id,
        token: header.token,
        service_hash: header.service_hash,
        is_response: Some(true),
        status: Some(0),
        ..Default::default()
    };
    let verify_response = serialize_frame(&verify_resp_header, None)?;
    // Build the OnLogonComplete push frame.
    let on_complete_frame = push_on_logon_complete(state, session, 0, Some(&session_key)).await?;

    if session.queued {
        // Store the OnLogonComplete frame in the queue entry.
        if let Some(q_entry) = state
            .login_queue
            .lock()
            .unwrap()
            .iter_mut()
            .find(|e| e.on_logon_complete.is_none())
        {
            q_entry.on_logon_complete = Some(on_complete_frame);
        }
        info!(
            session_id = %session.id,
            "OnLogonComplete stored in queue"
        );
        Ok(vec![verify_response])
    } else {
        info!(
            session_id = %session.id,
            "OnLogonComplete pushed after VerifyWebCredentials"
        );
        Ok(vec![verify_response, on_complete_frame])
    }
}

/// Handle `GenerateSSOToken` (method 5): issues SSO session credentials.
///
/// The client calls this after successful `VerifyWebCredentials` to get
/// sso_id + sso_secret for the realm-join handoff. The sso_secret is
/// used as the client `secret` field in `Param_ClientInfo` during
/// `Command_RealmListTicketRequest_v1`.
async fn handle_generate_sso_token(
    state: &Arc<BgsState>,
    session: &mut BgsSession,
    header: &Header,
    body: Option<&[u8]>,
) -> anyhow::Result<Vec<Bytes>> {
    let req = if let Some(b) = body {
        realmforge_gate_bgs::GenerateSsoTokenRequest::decode(b)?
    } else {
        realmforge_gate_bgs::GenerateSsoTokenRequest::default()
    };

    info!(session_id = %session.id, program = req.program, "GenerateSSOToken");

    // Check that the session is authenticated.
    if !session.authenticated {
        warn!(session_id = %session.id, "GenerateSSOToken: not authenticated");
        let err_header = Header {
            service_id: header.service_id,
            method_id: header.method_id,
            token: header.token,
            service_hash: header.service_hash,
            is_response: Some(true),
            status: Some(1),
            ..Default::default()
        };
        let frame = serialize_frame(&err_header, None)?;
        return Ok(vec![frame]);
    }

    // Generate SSO credentials.
    let sso_id = uuid::Uuid::new_v4().as_u128().to_le_bytes().to_vec();
    let mut sso_secret = vec![0u8; 32];
    rand::thread_rng().fill_bytes(&mut sso_secret);

    // Persist the session for RestoreSession.
    let _ = realmforge_gate_db::bgs_sessions::insert(
        &state.db,
        &sso_id,
        session._account_id.unwrap_or(0) as i64,
        session.battle_tag.as_deref().unwrap_or(""),
        session.build,
        session.platform.as_deref(),
        session.locale.as_deref(),
        session.session_key.as_ref().map(|k| k.as_slice()),
    )
    .await;
    let response = realmforge_gate_bgs::GenerateSsoTokenResponse {
        sso_id: Some(sso_id),
        sso_secret: Some(sso_secret),
    };

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

/// Handle `GenerateWebCredentials` (method 8): issues a WEB_TOKEN for
/// launcher-login handoff.
///
/// The desktop app (Phoenix, BGS v2) calls this after its own Logon to
/// get a transferable token that the game client can present via
/// `VerifyWebCredentials` (method 7). This bridges the two BGS protocol
/// versions: v2 (desktop) → token → v1 (game).
async fn handle_generate_web_credentials(
    _state: &Arc<BgsState>,
    session: &mut BgsSession,
    header: &Header,
    body: Option<&[u8]>,
) -> anyhow::Result<Vec<Bytes>> {
    let req = if let Some(b) = body {
        realmforge_gate_bgs::GenerateWebCredentialsRequest::decode(b)?
    } else {
        realmforge_gate_bgs::GenerateWebCredentialsRequest::default()
    };

    info!(session_id = %session.id, program = req.program, "GenerateWebCredentials");

    // The web credentials are an opaque token. For now, generate a
    // random token. In a full implementation, this would be stored
    // server-side and bound to: account, game_account, program FourCC,
    // and expiration time.
    let token = format!(
        "WC-{}-{}",
        uuid::Uuid::new_v4().to_string().replace('-', ""),
        session.id
    );

    let response = realmforge_gate_bgs::GenerateWebCredentialsResponse {
        web_credentials: Some(token.as_bytes().to_vec()),
    };

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

/// Handle `SelectGameAccount` (method 9): client selects a game account.
///
/// The client sends the EntityId of the game account it wants to use.
/// The server acknowledges with NoData. This is a no-op for single-account
/// setups but must return success for clients that call it.
async fn handle_select_game_account(
    _state: &Arc<BgsState>,
    session: &mut BgsSession,
    header: &Header,
    body: Option<&[u8]>,
) -> anyhow::Result<Vec<Bytes>> {
    let req = if let Some(b) = body {
        realmforge_gate_bgs::SelectGameAccountRequest::decode(b)?
    } else {
        return Ok(Vec::new());
    };

    info!(
        session_id = %session.id,
        game_account_id = ?req.game_account_id,
        "SelectGameAccount"
    );

    // Acknowledge with NoData.
    let resp_header = Header {
        service_id: header.service_id,
        method_id: header.method_id,
        token: header.token,
        service_hash: header.service_hash,
        is_response: Some(true),
        status: Some(0),
        ..Default::default()
    };
    let frame = serialize_frame(&resp_header, None)?;
    Ok(vec![frame])
}
/// Build the `OnLogonComplete` push frame (not a response to any request).
///
/// Returns the serialized frame bytes for the transport to write.
async fn build_on_logon_complete(
    _state: &Arc<BgsState>,
    session: &BgsSession,
    error_code: u32,
    session_key: Option<&[u8; 64]>,
) -> anyhow::Result<Bytes> {
    let mut result = LogonResult {
        error_code,
        ..Default::default()
    };

    if error_code == 0 {
        if let Some(key) = session_key {
            result.session_key = Some(key.to_vec());
        }
        // Push game accounts from the session.
        for ga in &session.game_accounts {
            result.game_account_id.push(realmforge_gate_bgs::EntityId {
                high: 0x576f57, // "WoW"
                low: ga.id as u64,
            });
        }
        result.battle_tag = session.battle_tag.clone();
    }

    let on_complete_header = Header {
        service_id: 1,
        method_id: Some(method_id::authentication_listener::ON_LOGON_COMPLETE),
        token: 0,
        service_hash: Some(service_hash::AUTHENTICATION_CLIENT_V1),
        is_response: Some(false),
        ..Default::default()
    };

    serialize_frame(&on_complete_header, Some(&result.encode_to_vec())).map_err(Into::into)
}

/// Build the v2 LogonResponse3 push frame.
fn build_logon_response_v3(
    session: &BgsSession,
    error_code: u32,
    session_key: Option<&[u8; 64]>,
) -> LogonResponse3 {
    let mut result = LogonResponse3 {
        error_code: Some(error_code),
        ping_timeout: Some(50),
        region: Some(1),
        account_flags: Some(0),
        game_account_region: Some(1),
        game_account_name: session
            .game_accounts
            .first()
            .map(|_| "WoW1".to_string())
            .or_else(|| Some("WoW1".to_string())),
        game_account_flags: Some(0),
        logon_failures: Some(0),
        ..Default::default()
    };
    if error_code == 0 {
        if let Some(key) = session_key {
            result.session_key = Some(key.to_vec());
        }
        result.battle_tag = session.battle_tag.clone();
    }
    result
}

/// Dispatch to the correct response builder based on protocol version.
async fn push_on_logon_complete(
    state: &Arc<BgsState>,
    session: &BgsSession,
    error_code: u32,
    session_key: Option<&[u8; 64]>,
) -> anyhow::Result<Bytes> {
    if session.protocol_version == Some(2) {
        let result = build_logon_response_v3(session, error_code, session_key);
        let (lh, _, _) = queue_push_params(2);
        let hdr = Header {
            service_id: 1,
            method_id: Some(1),
            token: 0,
            service_hash: Some(lh),
            is_response: Some(false),
            ..Default::default()
        };
        serialize_frame(&hdr, Some(&result.encode_to_vec())).map_err(Into::into)
    } else {
        build_on_logon_complete(state, session, error_code, session_key).await
    }
}

// --- GameUtilitiesService (for 1.13.2 realm list) ---

async fn handle_game_utilities(
    state: &Arc<BgsState>,
    session: &mut BgsSession,
    header: &Header,
    method_id: u32,
    body: Option<&[u8]>,
) -> anyhow::Result<Vec<Bytes>> {
    if header.is_response.unwrap_or(false) {
        return Ok(Vec::new());
    }

    match method_id {
        // ProcessClientRequest (method 1) — realm list, realm join, etc.
        method_id::game_utilities::PROCESS_CLIENT_REQUEST => {
            let body = body.unwrap_or(&[]);
            game_utilities::handle_process_client_request(state, session, header, body).await
        }
        // Removed in 1.14.0:
        method_id::game_utilities::GET_PLAYER_VARIABLES => {
            info!(session_id = %session.id, "GetPlayerVariables (removed in 1.14.0)");
            Ok(Vec::new())
        }
        method_id::game_utilities::GET_ACHIEVEMENTS_FILE => {
            info!(session_id = %session.id, "GetAchievementsFile (removed in 1.14.0)");
            Ok(Vec::new())
        }
        // Added in 1.14.0:
        method_id::game_utilities::REGISTER_UTILITIES => {
            info!(session_id = %session.id, "RegisterUtilities");
            // Return a client_id string for instance routing.
            let client_id = uuid::Uuid::new_v4().to_string();
            let response = realmforge_gate_bgs::ClientResponse {
                attribute: vec![realmforge_gate_bgs::Attribute {
                    name: "Param_ClientId".to_string(),
                    value: realmforge_gate_bgs::Variant {
                        string_value: Some(client_id),
                        ..Default::default()
                    },
                }],
            };
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
        method_id::game_utilities::UNREGISTER_UTILITIES => {
            info!(session_id = %session.id, "UnregisterUtilities");
            let resp_header = Header {
                service_id: header.service_id,
                method_id: header.method_id,
                token: header.token,
                service_hash: header.service_hash,
                is_response: Some(true),
                status: Some(0),
                ..Default::default()
            };
            let frame = serialize_frame(&resp_header, None)?;
            Ok(vec![frame])
        }
        _ => {
            info!(session_id = %session.id, method_id, "GameUtilities — unknown method");
            let resp_header = Header {
                service_id: header.service_id,
                method_id: header.method_id,
                token: header.token,
                service_hash: header.service_hash,
                is_response: Some(true),
                status: Some(0),
                ..Default::default()
            };
            let frame = serialize_frame(&resp_header, None)?;
            Ok(vec![frame])
        }
    }
}

// --- AccountService (0x62DA0891) ---
// Target: WoW Classic 1.13.2 (BGS v1). The 1.13.2 client calls only
// GetAccountState (30) + GetGameAccountState (31) in the pre-realm flow
// (caller scan, 2026-08-02); the other methods are stubs / retail
// leftovers. Responses use the 1.13.2 wire shapes (account_types.proto).

async fn handle_account_service(
    state: &Arc<BgsState>,
    session: &mut BgsSession,
    header: &Header,
    method_id: u32,
    body: Option<&[u8]>,
) -> anyhow::Result<Vec<Bytes>> {
    if header.is_response.unwrap_or(false) {
        return Ok(Vec::new());
    }

    let method_name = match method_id {
        method_id::account::RESOLVE_ACCOUNT => "ResolveAccount",
        method_id::account::SUBSCRIBE => "Subscribe",
        method_id::account::UNSUBSCRIBE => "Unsubscribe",
        method_id::account::GET_ACCOUNT_STATE => "GetAccountState",
        method_id::account::GET_GAME_ACCOUNT_STATE => "GetGameAccountState",
        method_id::account::GET_LICENSES => "GetLicenses",
        method_id::account::GET_GAME_TIME_REMAINING_INFO => "GetGameTimeRemainingInfo",
        method_id::account::GET_GAME_SESSION_INFO => "GetGameSessionInfo",
        method_id::account::GET_CAIS_INFO => "GetCAISInfo",
        method_id::account::GET_AUTHORIZED_DATA => "GetAuthorizedData",
        method_id::account::GET_SIGNED_ACCOUNT_STATE => "GetSignedAccountState",
        _ => "unknown",
    };
    info!(session_id = %session.id, method_id, method_name, "AccountService");

    let resp_header = Header {
        service_id: header.service_id,
        method_id: header.method_id,
        token: header.token,
        service_hash: header.service_hash,
        is_response: Some(true),
        status: Some(0),
        ..Default::default()
    };

    let body = build_account_response(state, session, method_id, body);
    let frame = serialize_frame(&resp_header, body.as_deref())?;
    Ok(vec![frame])
}

fn build_account_response(
    _state: &Arc<BgsState>,
    session: &BgsSession,
    method_id: u32,
    body: Option<&[u8]>,
) -> Option<Vec<u8>> {
    // Helper: first game account from session, or a stub
    let first_ga = session
        .game_accounts
        .first()
        .cloned()
        .unwrap_or(GameAccountHandle {
            id: 20001,
            program: 0x576f57, // "WoW" as fixed32
            region: 1,
        });

    // Resolve the license list for a game account: per-account grants
    // (migration 0022) take precedence; otherwise fall back to the
    // account-level licenses.
    let licenses_for = |ga_id: i64| -> Vec<realmforge_gate_bgs::AccountLicense> {
        session
            .per_game_account_licenses
            .get(&ga_id)
            .cloned()
            .filter(|l| !l.is_empty())
            .unwrap_or_else(|| session.licenses.clone())
    };

    match method_id {
        method_id::account::RESOLVE_ACCOUNT => {
            // ResolveAccountResponse { AccountId id = 12 }
            let resp = ResolveAccountResponse {
                id: Some(realmforge_gate_bgs::AccountId { id: first_ga.id }),
            };
            Some(resp.encode_to_vec())
        }
        method_id::account::GET_ACCOUNT_STATE => {
            // GetAccountStateResponse { AccountState state = 1 } (1.13.2 v1).
            // The 1.13.2 login gate only checks the response STATUS; the
            // body supplies the game-account list for ProcessGameAccountNames.
            // We return all fields (field options in the request are not
            // filtered — the client sets presence bits from whatever arrives).
            let game_accounts = if session.game_accounts.is_empty() {
                vec![GameAccountHandle {
                    id: 20001,
                    program: 0x576f57,
                    region: 1,
                }]
            } else {
                session.game_accounts.clone()
            };

            // Per-game-account GameLevelInfo: each game account carries its
            // own licenses (migration 0022) or falls back to the account-
            // level grants.
            let game_level_info: Vec<GameLevelInfo> = session
                .game_account_info
                .iter()
                .map(|g| GameLevelInfo {
                    is_trial: None,
                    is_lifetime: None,
                    is_restricted: Some(g.is_suspended || g.is_banned),
                    is_beta: None,
                    name: Some(g.name.clone()),
                    program: Some(0x576f57),
                    licenses: licenses_for(g.id),
                    realm_permissions: None,
                })
                .collect();

            // Per-game-account GameStatus.
            let game_status: Vec<GameStatus> = session
                .game_account_info
                .iter()
                .map(|g| GameStatus {
                    is_suspended: Some(g.is_suspended),
                    is_banned: Some(g.is_banned),
                    suspension_expires: g.suspension_expires.map(|v| v as u64),
                    program: Some(0x576f57),
                    is_locked: None,
                    is_bam_unlockable: None,
                })
                .collect();

            // Group game account handles by region (GameAccountList).
            let mut game_account_lists: Vec<GameAccountList> = Vec::new();
            for ga in &game_accounts {
                if let Some(list) = game_account_lists
                    .iter_mut()
                    .find(|l| l.region == Some(ga.region))
                {
                    list.handle.push(*ga);
                } else {
                    game_account_lists.push(GameAccountList {
                        region: Some(ga.region),
                        handle: vec![*ga],
                    });
                }
            }

            let account_level = AccountLevelInfo {
                licenses: session.licenses.clone(),
                default_currency: None,
                country: None,
                preferred_region: None,
                full_name: None,
                battle_tag: session.battle_tag.clone(),
                muted: None,
                manual_review: None,
                account_paid_any: Some(!session.licenses.is_empty()),
                email: None,
                headless_account: None,
                test_account: None,
                is_sms_protected: None,
            };

            let state = AccountState {
                account_level_info: Some(account_level),
                privacy_info: None,
                parental_control_info: None,
                game_level_info,
                game_status,
                game_accounts: game_account_lists,
                security_status: None,
            };
            let resp = GetAccountStateResponse { state: Some(state) };
            Some(resp.encode_to_vec())
        }
        method_id::account::GET_GAME_ACCOUNT_STATE => {
            // GetGameAccountStateResponse { GameAccountState state = 1 }
            // The 1.13.2 login gate reads the REQUESTED game account's state
            // (GetGameAccountStateRequest.game_account_id = 2) — licenses
            // (game_level_info.licenses incl. 0x2cde/0x2cdf), game time,
            // and game status. When the request carries no id (or it does
            // not resolve), fall back to the first account with active game
            // time, then the first account.
            let requested_id = body.and_then(|b| {
                realmforge_gate_bgs::GetGameAccountStateRequest::decode(b)
                    .ok()
                    .and_then(|r| r.game_account_id)
                    .map(|e| e.low as i64)
            });
            let gi = requested_id
                .and_then(|id| session.game_account_info.iter().find(|g| g.id == id))
                .or_else(|| {
                    let now_ms = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_millis() as i64;
                    session
                        .game_account_info
                        .iter()
                        .find(|g| g.game_time_expires.is_some_and(|exp| exp > now_ms))
                })
                .or_else(|| session.game_account_info.first());
            let state = GameAccountState {
                game_level_info: gi.map(|g| GameLevelInfo {
                    is_trial: None,
                    is_lifetime: None,
                    is_restricted: Some(g.is_suspended || g.is_banned),
                    is_beta: None,
                    name: Some(g.name.clone()),
                    program: Some(0x576f57),
                    licenses: licenses_for(g.id),
                    realm_permissions: None,
                }),
                game_time_info: Some(GameTimeInfo {
                    is_unlimited_play_time: None,
                    play_time_expires: gi.and_then(|g| g.game_time_expires).map(|v| v as u64),
                    is_subscription: Some(
                        gi.map(|g| g.game_time_expires.is_some()).unwrap_or(false),
                    ),
                    is_recurring_subscription: None,
                }),
                game_status: gi.map(|g| GameStatus {
                    is_suspended: Some(g.is_suspended),
                    is_banned: Some(g.is_banned),
                    suspension_expires: g.suspension_expires.map(|v| v as u64),
                    program: Some(0x576f57),
                    is_locked: None,
                    is_bam_unlockable: None,
                }),
                raf_info: None,
            };
            let resp = GetGameAccountStateResponse { state: Some(state) };
            Some(resp.encode_to_vec())
        }
        method_id::account::GET_LICENSES => {
            // GetLicensesResponse { repeated AccountLicense licenses = 1 }
            let resp = GetLicensesResponse {
                licenses: session.licenses.clone(),
            };
            Some(resp.encode_to_vec())
        }
        method_id::account::GET_GAME_TIME_REMAINING_INFO => {
            // GetGameTimeRemainingInfoResponse { minutes_remaining = 1 }
            let minutes = session
                .game_account_info
                .first()
                .and_then(|g| g.game_time_expires)
                .map(|expires_ms| {
                    let now_ms = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_millis() as i64;
                    ((expires_ms - now_ms) / 60_000).max(0) as u64
                })
                .unwrap_or(0);
            let resp = GetGameTimeRemainingInfoResponse {
                minutes_remaining: Some(minutes as u32),
                parental_daily_minutes_remaining: None,
                parental_weekly_minutes_remaining: None,
            };
            Some(resp.encode_to_vec())
        }
        method_id::account::GET_GAME_SESSION_INFO => {
            // GetGameSessionInfo
            let resp = GameSessionInfo {
                session_count: Some(1),
            };
            Some(resp.encode_to_vec())
        }
        // Subscribe(25), Unsubscribe(26), GetCAISInfo(35),
        // GetAuthorizedData(37), GetSignedAccountState(44): NoData
        _ => None,
    }
}
// --- SessionService ---

async fn handle_session_service(
    state: &Arc<BgsState>,
    session: &mut BgsSession,
    header: &Header,
    method_id: u32,
    body: Option<&[u8]>,
) -> anyhow::Result<Vec<Bytes>> {
    if header.is_response.unwrap_or(false) {
        return Ok(Vec::new());
    }

    match method_id {
        method_id::session::CREATE_SESSION => {
            handle_create_session(state, session, header, body).await
        }
        method_id::session::RESTORE_SESSION => {
            handle_restore_session(state, session, header, body).await
        }
        method_id::session::DESTROY_SESSION => {
            handle_destroy_session(state, session, header, body).await
        }
        _ => {
            info!(session_id = %session.id, method_id, "unknown session method");
            Ok(Vec::new())
        }
    }
}

async fn handle_restore_session(
    state: &Arc<BgsState>,
    session: &mut BgsSession,
    _header: &Header,
    body: Option<&[u8]>,
) -> anyhow::Result<Vec<Bytes>> {
    let sid = if let Some(b) = body {
        realmforge_gate_bgs::SessionId::decode(b)?
    } else {
        return Ok(Vec::new());
    };

    // v1: instance_id is hex-encoded. v2: instance_id is raw bytes.
    let sso_id = if session.protocol_version == Some(2) {
        sid.instance_id
            .as_deref()
            .map(|s| s.as_bytes().to_vec())
            .unwrap_or_default()
    } else {
        sid.instance_id
            .as_deref()
            .and_then(|s| hex::decode(s).ok())
            .unwrap_or_default()
    };

    {
        let active = state
            .active_logins
            .load(std::sync::atomic::Ordering::Acquire);
        if active >= state.max_logins {
            let push_tx = state
                .push_channels
                .get(&session.id)
                .map(|r| r.value().clone())
                .unwrap();
            let position = {
                let mut q = state.login_queue.lock().unwrap();
                q.push_back(QueueEntry {
                    push_tx,
                    on_logon_complete: None,
                    protocol_version: session.protocol_version.unwrap_or(1),
                    sso_id: Some(sso_id.clone()),
                });
                q.len()
            };
            let update = LogonQueueUpdateRequest {
                position: Some(position as u32),
                estimated_time: Some((position * 5) as u64),
                eta_deviation_in_sec: Some(10),
            };
            let (lh, um, _) = queue_push_params(session.protocol_version.unwrap_or(1));
            let push_header = Header {
                service_id: 1,
                method_id: Some(um),
                token: 0,
                service_hash: Some(lh),
                is_response: Some(false),
                ..Default::default()
            };
            let frame = serialize_frame(&push_header, Some(&update.encode_to_vec()))?;
            let _ = state.push_channels.get(&session.id).map(|r| r.send(frame));
            session.queued = true;
            let resp = serialize_frame(
                &realmforge_gate_bgs::Header {
                    service_id: 1,
                    method_id: Some(2),
                    token: 0,
                    service_hash: Some(realmforge_gate_bgs::service_hash::SESSION_SERVICE),
                    is_response: Some(true),
                    status: Some(0),
                    ..Default::default()
                },
                None,
            )?;
            return Ok(vec![resp]);
        }
    }

    if let Some(row) = realmforge_gate_db::bgs_sessions::find_valid(&state.db, &sso_id).await? {
        session.authenticated = true;
        session._account_id = Some(row.account_id as u64);
        kick_existing_session(state, row.account_id as u64, &session.id);
        session.battle_tag = Some(row.battle_tag);
        session.build = row.build;
        session.platform = row.platform;
        session.locale = row.locale;
        // Restored sessions keep the BGS session key so the realm-join
        // handoff can still serve Param_BnetSessionKey.
        session.session_key = row.session_key.and_then(|k| k.try_into().ok());
        info!(
            session_id = %session.id,
            account_id = row.account_id,
            "session restored"
        );
        let on_complete = push_on_logon_complete(state, session, 0, None).await?;
        let resp = serialize_frame(
            &realmforge_gate_bgs::Header {
                service_id: 1,
                method_id: Some(2),
                token: 0,
                service_hash: Some(realmforge_gate_bgs::service_hash::SESSION_SERVICE),
                is_response: Some(true),
                status: Some(0),
                ..Default::default()
            },
            None,
        )?;
        Ok(vec![resp, on_complete])
    } else {
        warn!(
            session_id = %session.id,
            "RestoreSession: invalid or expired token"
        );
        Ok(Vec::new())
    }
}

async fn handle_create_session(
    _state: &Arc<BgsState>,
    session: &mut BgsSession,
    header: &Header,
    body: Option<&[u8]>,
) -> anyhow::Result<Vec<Bytes>> {
    let req = if let Some(b) = body {
        CreateSessionRequest::decode(b)?
    } else {
        CreateSessionRequest::default()
    };

    let sid = SessionId {
        account_id: None,
        game_account: req.game_account,
        instance_id: Some(session.id.clone()),
        region: Some(1),
    };

    let variables = SessionVariables {
        keep_alive_interval: Some(50),
        idle_timeout: Some(600),
    };

    let response = CreateSessionResponse {
        session_id: Some(sid),
        variables: Some(variables),
    };

    info!(session_id = %session.id, ?req.game_account, "CreateSession");

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

async fn handle_destroy_session(
    _state: &Arc<BgsState>,
    session: &mut BgsSession,
    _header: &Header,
    body: Option<&[u8]>,
) -> anyhow::Result<Vec<Bytes>> {
    if let Some(b) = body {
        let _ = DestroySessionRequest::decode(b);
    }

    info!(session_id = %session.id, "DestroySession");
    Ok(Vec::new())
}

fn main() -> anyhow::Result<()> {
    // The whole dependency tree pins rustls to the ring provider (see the
    // workspace Cargo.toml); install it before any TLS use.
    rustls::crypto::ring::default_provider()
        .install_default()
        .expect("install rustls CryptoProvider");
    let gate_config = realmforge_config::GateConfig::from_env()?;
    let workers = gate_config.worker_threads;
    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(workers)
        .enable_all()
        .build()?
        .block_on(async move {
            tracing_subscriber::fmt()
                .with_env_filter(
                    tracing_subscriber::EnvFilter::try_from_default_env()
                        .unwrap_or_else(|_| "bgs_server=info".into()),
                )
                .init();

            let meter_provider = realmforge_gate_observability::init("realmforge-gate");
            let bind_addr = gate_config.ws_bind_addr;
            let database_url = gate_config.database_url.clone();

            if !gate_config.legacy_aliases_used.is_empty() {
                warn!(
                    aliases = ?gate_config.legacy_aliases_used,
                    "legacy Gate configuration aliases are in use; migrate to REALMFORGE_GATE_* names"
                );
            }
            let pool_cfg = realmforge_gate_db::PoolConfig::from_env();
            let db = realmforge_gate_db::connect_with(&database_url, &pool_cfg).await?;
            realmforge_gate_db::run_migrations(&db).await?;

            realmforge_gate_observability::register_pool_gauge(db.clone());
            let (health_router, health_state) =
                realmforge_gate_observability::health_router(Some(db.clone()));
            health_state.mark_started();

            let realm_registry = realmforge_realms::RealmRegistry::from_env()?;
            info!(
                realm_count = realm_registry.len(),
                "Realmforge Gate realm registry loaded"
            );

            let state = Arc::new(BgsState {
                active_logins: std::sync::atomic::AtomicU64::new(0),
                max_logins: gate_config.max_logins,
                login_queue: std::sync::Mutex::new(std::collections::VecDeque::new()),
                push_channels: dashmap::DashMap::new(),
                active_accounts: dashmap::DashMap::new(),
                realm_registry,
                db,
            });
            spawn_queue_ticker(state.clone());

            // BGS metrics.
            let q_state = state.clone();
            let bgs_meter = opentelemetry::global::meter("realmforge.gate");
            bgs_meter
                .u64_observable_gauge("realmforge.gate.bgs.login_queue.length")
                .with_unit("{client}")
                .with_description("Clients waiting in the login queue")
                .with_callback(move |obs| {
                    obs.observe(q_state.login_queue.lock().unwrap().len() as u64, &[]);
                })
                .build();
            AUTH_SUCCESS.get_or_init(|| {
                bgs_meter
                    .u64_counter("realmforge.gate.bgs.auth.success")
                    .with_unit("{login}")
                    .with_description("Successful BGS logins")
                    .build()
            });
            BUILD_COUNTER.get_or_init(|| {
                bgs_meter
                    .u64_counter("realmforge.gate.bgs.client.build")
                    .with_unit("{client}")
                    .with_description("Client connections by build and platform")
                    .build()
            });
            KICKED_COUNTER.get_or_init(|| {
                bgs_meter
                    .u64_counter("realmforge.gate.bgs.session.kicked")
                    .with_unit("{session}")
                    .with_description("Sessions kicked due to concurrent login")
                    .build()
            });
            AUTH_FAILURE.get_or_init(|| {
                bgs_meter
                    .u64_counter("realmforge.gate.bgs.auth.failure")
                    .with_unit("{login}")
                    .with_description("Failed BGS logins")
                    .build()
            });
            LICENSE_COUNTER.get_or_init(|| {
                bgs_meter
                    .u64_counter("realmforge.gate.bgs.license.count")
                    .with_unit("{license}")
                    .with_description("Licenses granted at login")
                    .build()
            });
            GAME_TIME_HISTOGRAM.get_or_init(|| {
                bgs_meter
                    .f64_histogram("realmforge.gate.bgs.game_time.remaining_minutes")
                    .with_unit("min")
                    .with_description("Remaining game time at login")
                    .build()
            });
            STATUS_COUNTER.get_or_init(|| {
                bgs_meter
                    .u64_counter("realmforge.gate.bgs.account.status")
                    .with_unit("{account}")
                    .with_description("Account status at login (ok, suspended, banned, expired)")
                    .build()
            });

            let app = Router::new()
                .route("/", get(ws_handler))
                .with_state(state.clone())
                .layer(axum::middleware::from_fn(
                    realmforge_gate_observability::metrics_middleware,
                ))
                .merge(health_router);

            info!(%bind_addr, "Realmforge Gate BGS WebSocket server starting");
            let listener = TcpListener::bind(bind_addr).await?;

            // Start raw TCP/TLS listener for WoW Classic 1.13.2 (port 1119).
            let tcp_bind_addr = gate_config.tcp_bind_addr;
            let tcp_tls = gate_config
                .tcp_tls
                .as_ref()
                .map(|tls| load_tls_server_config(&tls.cert_path, &tls.key_path))
                .transpose()?;
            let tcp_state = state.clone();
            tokio::spawn(async move {
                if let Err(e) = tcp_transport::start_tcp_listener(
                    &tcp_bind_addr.to_string(),
                    tcp_tls,
                    tcp_state,
                )
                .await
                {
                    tracing::error!("TCP transport error: {e}");
                }
            });

            axum::serve(listener, app)
                .with_graceful_shutdown(realmforge_gate_observability::shutdown_signal())
                .await?;

            broadcast_boom(&state);
            realmforge_gate_observability::shutdown(meter_provider);

            Ok(())
        })
}
