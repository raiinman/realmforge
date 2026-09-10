use std::env;
use std::fs::{self, File};
use std::io::{ErrorKind, Read, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use realmforge_gate_independent::{
    CaptureArtifact, CaptureManifest, CaptureObservation, ObservationDirection, ObservationLayer,
    RedactionStatus,
};
use sha2::{Digest, Sha256};

const DEFAULT_BIND: &str = "127.0.0.1:1119";
const DEFAULT_MAX_BYTES: usize = 4 * 1024 * 1024;
const DEFAULT_TIMEOUT_SECONDS: u64 = 15;
const ARTIFACT_ID: &str = "observer-tcp-001";
const ARTIFACT_RELATIVE_PATH: &str = "artifacts/observer-tcp-001.bin";

#[derive(Debug)]
struct ObserverArgs {
    manifest: PathBuf,
    bind: String,
    max_bytes: usize,
    timeout: Duration,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = parse_args()?;
    let manifest_json = fs::read_to_string(&args.manifest)?;
    let mut manifest = CaptureManifest::from_json(&manifest_json)?;
    manifest.validate()?;

    if !manifest.artifacts.is_empty() || !manifest.observations.is_empty() {
        return Err(
            "observer requires a seed manifest with empty artifacts and observations; refusing to overwrite existing evidence"
                .into(),
        );
    }

    let bundle_dir = args
        .manifest
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let artifact_path = bundle_dir.join(ARTIFACT_RELATIVE_PATH);
    if let Some(parent) = artifact_path.parent() {
        fs::create_dir_all(parent)?;
    }
    if artifact_path.exists() {
        return Err(format!("artifact already exists: {}", artifact_path.display()).into());
    }

    let listener = TcpListener::bind(&args.bind)?;
    eprintln!(
        "Realmforge protocol-neutral observer listening on {} for capture {}",
        args.bind, manifest.capture_id
    );
    eprintln!(
        "This tool does not identify the application protocol. It records one TCP connection only."
    );

    let started = Instant::now();
    let (mut stream, peer) = listener.accept()?;
    stream.set_read_timeout(Some(args.timeout))?;

    manifest.observations.push(CaptureObservation {
        sequence: 1,
        offset_micros: elapsed_micros(started),
        direction: ObservationDirection::LocalEvent,
        layer: ObservationLayer::Tcp,
        operation: "accept_connection".to_owned(),
        endpoint: Some(peer.to_string()),
        artifact_id: None,
        payload_sha256: None,
        notes: Some(format!("observer bind={}", args.bind)),
    });

    let mut captured = Vec::new();
    let mut buffer = [0u8; 16 * 1024];
    let mut termination = "peer_closed";

    loop {
        if captured.len() >= args.max_bytes {
            termination = "max_bytes_reached";
            break;
        }
        let remaining = args.max_bytes - captured.len();
        let read_len = remaining.min(buffer.len());
        match stream.read(&mut buffer[..read_len]) {
            Ok(0) => break,
            Ok(count) => captured.extend_from_slice(&buffer[..count]),
            Err(error) if matches!(error.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut) => {
                termination = "read_timeout";
                break;
            }
            Err(error) => return Err(error.into()),
        }
    }

    let digest = sha256_hex(&captured);
    let mut artifact = File::create(&artifact_path)?;
    artifact.write_all(&captured)?;
    artifact.sync_all()?;

    manifest.artifacts.push(CaptureArtifact {
        id: ARTIFACT_ID.to_owned(),
        relative_path: ARTIFACT_RELATIVE_PATH.to_owned(),
        media_type: "application/octet-stream".to_owned(),
        sha256: digest.clone(),
        contains_secrets: true,
        redaction_status: RedactionStatus::NotReviewed,
    });
    manifest.observations.push(CaptureObservation {
        sequence: 2,
        offset_micros: elapsed_micros(started),
        direction: ObservationDirection::ClientToServer,
        layer: ObservationLayer::Tcp,
        operation: "receive_bytes".to_owned(),
        endpoint: Some(peer.to_string()),
        artifact_id: Some(ARTIFACT_ID.to_owned()),
        payload_sha256: Some(digest),
        notes: Some(format!(
            "bytes={}; termination={termination}; no application-protocol label assigned",
            captured.len()
        )),
    });
    manifest.actual_result = format!(
        "Realmforge observer accepted one TCP connection and captured {} bytes ({termination})",
        captured.len()
    );
    manifest.validate()?;

    let output = manifest.to_pretty_json()?;
    fs::write(&args.manifest, format!("{output}\n"))?;

    println!("capture_id={}", manifest.capture_id);
    println!("peer={peer}");
    println!("bytes={}", captured.len());
    println!("artifact={}", artifact_path.display());
    println!("manifest={}", args.manifest.display());
    println!("redaction_status=not_reviewed");
    println!("implementation_ready=false");
    eprintln!(
        "Review/redact the raw artifact before changing contains_secrets or redaction_status."
    );
    Ok(())
}

fn parse_args() -> Result<ObserverArgs, Box<dyn std::error::Error>> {
    let mut manifest = None;
    let mut bind = DEFAULT_BIND.to_owned();
    let mut max_bytes = DEFAULT_MAX_BYTES;
    let mut timeout_seconds = DEFAULT_TIMEOUT_SECONDS;

    let mut args = env::args().skip(1);
    while let Some(flag) = args.next() {
        match flag.as_str() {
            "--manifest" => manifest = Some(PathBuf::from(next_value(&mut args, "--manifest")?)),
            "--bind" => bind = next_value(&mut args, "--bind")?,
            "--max-bytes" => {
                max_bytes = next_value(&mut args, "--max-bytes")?
                    .parse()
                    .map_err(|_| "--max-bytes must be a positive integer")?;
                if max_bytes == 0 {
                    return Err("--max-bytes must be greater than zero".into());
                }
            }
            "--timeout-seconds" => {
                timeout_seconds = next_value(&mut args, "--timeout-seconds")?
                    .parse()
                    .map_err(|_| "--timeout-seconds must be a positive integer")?;
                if timeout_seconds == 0 {
                    return Err("--timeout-seconds must be greater than zero".into());
                }
            }
            "--help" | "-h" => {
                print_usage();
                std::process::exit(0);
            }
            _ => return Err(format!("unknown argument: {flag}").into()),
        }
    }

    let manifest = manifest.ok_or("--manifest is required")?;
    Ok(ObserverArgs {
        manifest,
        bind,
        max_bytes,
        timeout: Duration::from_secs(timeout_seconds),
    })
}

fn next_value(
    args: &mut impl Iterator<Item = String>,
    flag: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    args.next()
        .ok_or_else(|| format!("{flag} requires a value").into())
}

fn print_usage() {
    println!(
        "usage: realmforge-gate-observer --manifest <seed-manifest.json> [--bind HOST:PORT] [--max-bytes N] [--timeout-seconds N]"
    );
}

fn elapsed_micros(started: Instant) -> u64 {
    u64::try_from(started.elapsed().as_micros()).unwrap_or(u64::MAX)
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut output = String::with_capacity(64);
    for byte in digest {
        use std::fmt::Write as _;
        write!(&mut output, "{byte:02x}").expect("writing to String cannot fail");
    }
    output
}
