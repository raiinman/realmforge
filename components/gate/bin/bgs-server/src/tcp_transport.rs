// SPDX-License-Identifier: AGPL-3.0-only

//! Raw TCP BGS transport for WoW Classic 1.13.2.
//!
//! The 1.13.2 game client connects to port 1119 via raw TCP (not WebSocket).
//! The transport layer wraps Aurora RPC frames inside a WoW-specific outer
//! protocol:
//!
//! 1. **Banner exchange**: 0x34-byte magic string sent/received before any
//!    framing begins.
//! 2. **Opcode multiplexer**: each frame is prefixed with a u16 opcode;
//!    opcodes 0x3048-0x3052 carry Aurora RPC frames.
//! 3. **Aurora frame**: 2-byte BE header size + protobuf Header + body,
//!    parsed by the existing `tavern_bgs::frame` module.
//!
//! The 0x3052 opcode carries game-RPC frames with a 12-byte header and CRC
//! seed `0x9827D8F1`. Game-RPC is not handled here — only Aurora RPC is
//! dispatched.

use std::sync::Arc;

use anyhow::Context;
use prost::bytes::BytesMut;
use tavern_bgs::frame::{FrameError, parse_frame};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::net::TcpListener;
use tracing::{debug, info, warn};
/// The WoW Classic client banner string (exactly 0x34 bytes).
const BANNER_LEN: usize = 0x34;

/// Aurora RPC opcode range (inclusive).
const AURORA_OPCODE_MIN: u16 = 0x3048;
const AURORA_OPCODE_MAX: u16 = 0x3052;

/// Start a raw TCP BGS listener on the given address, sharing dispatch state
/// with the WebSocket listener.
pub async fn start_tcp_listener(
    bind_addr: &str,
    tls_config: Option<rustls::ServerConfig>,
    state: Arc<super::BgsState>,
) -> anyhow::Result<()> {
    let listener = TcpListener::bind(bind_addr)
        .await
        .with_context(|| format!("failed to bind TCP listener on {bind_addr}"))?;

    let acceptor = tls_config.map(|cfg| tokio_rustls::TlsAcceptor::from(Arc::new(cfg)));
    match &acceptor {
        Some(_) => info!(bind_addr, "BGS TLS TCP transport listening"),
        None => info!(bind_addr, "BGS raw TCP transport listening"),
    }

    loop {
        let (stream, peer) = match listener.accept().await {
            Ok(conn) => conn,
            Err(e) => {
                warn!("TCP accept error: {e}");
                continue;
            }
        };
        info!(%peer, "BGS TCP connection accepted");

        let state = state.clone();
        let acceptor = acceptor.clone();
        tokio::spawn(async move {
            // The 1.13.2 client dials the BGS port over TLS (matching
            // TrinityCore bnetserver); the banner exchange runs inside the
            // TLS stream. When no cert/key is configured we stay plain so
            // the synthetic test clients keep working.
            let result = match acceptor {
                Some(acceptor) => match acceptor.accept(stream).await {
                    Ok(tls_stream) => handle_tcp_connection(tls_stream, state).await,
                    Err(e) => {
                        warn!(%peer, "TLS handshake failed: {e}");
                        return;
                    }
                },
                None => handle_tcp_connection(stream, state).await,
            };
            if let Err(e) = result {
                warn!(%peer, "TCP connection error: {e}");
            }
            info!(%peer, "BGS TCP connection closed");
        });
    }
}

/// Handle a single BGS connection (plain or TLS).
///
/// Generic over the stream so the same framing code serves both a raw
/// `TcpStream` and a `TlsStream<TcpStream>`.
async fn handle_tcp_connection<S>(
    mut stream: S,
    state: Arc<super::BgsState>,
) -> anyhow::Result<()>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    let session_id = uuid::Uuid::new_v4().to_string();
    let mut session = super::BgsSession::new(session_id.clone());

    // --- Step 1: Banner exchange ---
    let banner = b"WORLD OF WARCRAFT CONNECTION - SERVER TO CLIENT - V2";
    debug_assert_eq!(banner.len(), BANNER_LEN);

    let mut banner_buf = vec![0u8; BANNER_LEN];
    stream.read_exact(&mut banner_buf).await?;

    if banner_buf != banner {
        warn!(
            session_id,
            "invalid client banner: {:?}",
            String::from_utf8_lossy(&banner_buf)
        );
        return Ok(());
    }

    stream.write_all(banner).await?;
    stream.flush().await?;
    debug!(session_id, "banner exchange complete");

    // --- Step 2: Read opcode-framed Aurora frames ---
    let mut read_buf = BytesMut::with_capacity(8192);
    let mut tmp = vec![0u8; 4096];

    loop {
        match stream.read(&mut tmp).await {
            Ok(0) => break,
            Ok(n) => read_buf.extend_from_slice(&tmp[..n]),
            Err(e) => {
                warn!(session_id, "TCP read error: {e}");
                break;
            }
        }

        while read_buf.len() >= 2 {
            let opcode = u16::from_be_bytes([read_buf[0], read_buf[1]]);

            if !(AURORA_OPCODE_MIN..=AURORA_OPCODE_MAX).contains(&opcode) {
                let _ = read_buf.split_to(2);
                debug!(session_id, opcode, "skipped unknown opcode");
                continue;
            }

            let _ = read_buf.split_to(2);

            let frame_result = parse_frame(&mut read_buf);

            match frame_result {
                Ok(Some(frame)) => {
                    let frames = super::dispatch_frame(
                        &state,
                        &mut session,
                        &frame.header,
                        frame.body.as_deref(),
                    )
                    .await;

                    match frames {
                        Ok(response_frames) => {
                            for frame_bytes in response_frames {
                                let mut out = Vec::with_capacity(2 + frame_bytes.len());
                                out.extend_from_slice(&opcode.to_be_bytes());
                                out.extend_from_slice(&frame_bytes);
                                if let Err(e) = stream.write_all(&out).await {
                                    warn!(session_id, "TCP write error: {e}");
                                    break;
                                }
                            }
                        }
                        Err(e) => {
                            warn!(session_id, "TCP frame dispatch error: {e}");
                        }
                    }
                }
                Ok(None) | Err(FrameError::Incomplete(..)) => {
                    let opcode_bytes = opcode.to_be_bytes();
                    let mut restored = BytesMut::with_capacity(2 + read_buf.len());
                    restored.extend_from_slice(&opcode_bytes);
                    restored.unsplit(read_buf.split());
                    read_buf = restored;
                    break;
                }
                Err(FrameError::HeaderTooLarge(_)) | Err(FrameError::Decode(_)) => {
                    warn!(session_id, "Aurora frame parse error");
                    read_buf.clear();
                    break;
                }
                Err(FrameError::Encode(_)) => {
                    read_buf.clear();
                    break;
                }
            }
        }
    }

    Ok(())
}
