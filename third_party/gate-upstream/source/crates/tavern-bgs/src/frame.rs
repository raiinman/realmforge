// SPDX-License-Identifier: AGPL-3.0-only

//! BGS RPC binary frame format: 2-byte big-endian header size +
//! serialized protobuf Header + optional message body.

use bytes::{Buf, BufMut, Bytes, BytesMut};
use prost::Message;
use thiserror::Error;

use crate::Header;

#[derive(Debug, Error)]
pub enum FrameError {
    #[error("incomplete frame: need {0} bytes, got {1}")]
    Incomplete(usize, usize),
    #[error("header too large: {0} bytes")]
    HeaderTooLarge(usize),
    #[error("protobuf decode error: {0}")]
    Decode(#[from] prost::DecodeError),
    #[error("protobuf encode error: {0}")]
    Encode(#[from] prost::EncodeError),
}

/// Maximum header size (64KB — the 2-byte prefix caps at 65535).
const MAX_HEADER_SIZE: usize = 65535;

/// A decoded BGS RPC frame.
#[derive(Debug)]
pub struct Frame {
    pub header: Header,
    pub body: Option<Bytes>,
}

/// Serialize a BGS RPC frame: 2-byte header size + header protobuf + body.
pub fn serialize_frame(header: &Header, body: Option<&[u8]>) -> Result<Bytes, FrameError> {
    let mut buf = BytesMut::new();

    // Encode header protobuf.
    let header_bytes = header.encode_to_vec();
    let header_len = header_bytes.len() as u16;

    // Write 2-byte big-endian header size.
    buf.put_u16(header_len);

    // Write header bytes.
    buf.extend_from_slice(&header_bytes);

    // Write body bytes if present.
    if let Some(b) = body {
        buf.extend_from_slice(b);
    }

    Ok(buf.freeze())
}

/// Try to parse a BGS RPC frame from a buffer.
///
/// Returns `Ok(Some(frame))` if a complete frame was parsed, `Ok(None)`
/// if more data is needed, or `Err` if the frame is malformed.
pub fn parse_frame(buf: &mut BytesMut) -> Result<Option<Frame>, FrameError> {
    if buf.len() < 2 {
        return Ok(None);
    }

    let header_size = u16::from_be_bytes([buf[0], buf[1]]) as usize;

    if header_size > MAX_HEADER_SIZE {
        return Err(FrameError::HeaderTooLarge(header_size));
    }

    let total_needed = 2 + header_size;
    if buf.len() < total_needed {
        return Ok(None);
    }

    // Advance past the 2-byte size.
    buf.advance(2);

    // Parse header protobuf.
    let header = Header::decode(&buf[..header_size])?;
    buf.advance(header_size);

    // Remaining bytes are the body.
    let body = if buf.is_empty() {
        None
    } else {
        Some(buf.split().freeze())
    };

    Ok(Some(Frame { header, body }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_empty_body() {
        let header = Header {
            service_id: 1,
            method_id: Some(1),
            token: 42,
            service_hash: Some(0x65446991),
            ..Default::default()
        };

        let frame = serialize_frame(&header, None).unwrap();
        let mut buf = BytesMut::from(frame.as_ref());
        let parsed = parse_frame(&mut buf).unwrap().unwrap();

        assert_eq!(parsed.header.service_id, 1);
        assert_eq!(parsed.header.token, 42);
        assert!(parsed.body.is_none());
    }

    #[test]
    fn round_trip_with_body() {
        let header = Header {
            service_id: 1,
            token: 0,
            is_response: Some(true),
            ..Default::default()
        };

        let body = b"hello world";
        let frame = serialize_frame(&header, Some(body)).unwrap();
        let mut buf = BytesMut::from(frame.as_ref());
        let parsed = parse_frame(&mut buf).unwrap().unwrap();

        assert_eq!(parsed.header.is_response, Some(true));
        assert_eq!(parsed.body.as_deref(), Some(&body[..]));
    }

    #[test]
    fn incomplete_returns_none() {
        let mut buf = BytesMut::from(&[0x00, 0x10][..]); // header size 16, but only 2 bytes
        let result = parse_frame(&mut buf).unwrap();
        assert!(result.is_none());
    }
}
