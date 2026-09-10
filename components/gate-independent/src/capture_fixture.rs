use std::collections::BTreeSet;
use std::fmt;
use std::path::Path;

use serde::{Deserialize, Serialize};

pub const CAPTURE_FIXTURE_SCHEMA_V1: &str = "realmforge.capture.v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceGrade {
    FirstParty,
    Standard,
    Correlated,
    UpstreamLead,
    CommunityLead,
    Hypothesis,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RedactionStatus {
    NotReviewed,
    Reviewed,
    NotRequired,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ObservationDirection {
    ClientToServer,
    ServerToClient,
    LocalEvent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ObservationLayer {
    Process,
    Dns,
    Tcp,
    Tls,
    Http,
    WebSocket,
    Rpc,
    Realm,
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClientDescriptor {
    pub family: String,
    pub version: String,
    pub build: u32,
    pub platform: String,
    pub architecture: String,
    pub executable_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CaptureArtifact {
    pub id: String,
    pub relative_path: String,
    pub media_type: String,
    pub sha256: String,
    pub contains_secrets: bool,
    pub redaction_status: RedactionStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CaptureObservation {
    pub sequence: u32,
    pub offset_micros: u64,
    pub direction: ObservationDirection,
    pub layer: ObservationLayer,
    pub operation: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub endpoint: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub artifact_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub payload_sha256: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CaptureManifest {
    pub schema: String,
    pub capture_id: String,
    pub captured_at_utc: String,
    pub researcher: String,
    pub client: ClientDescriptor,
    pub os: String,
    pub network_topology: String,
    pub server_under_test: String,
    pub server_revision: String,
    pub scenario: String,
    pub expected_result: String,
    pub actual_result: String,
    pub evidence_grade: EvidenceGrade,
    pub artifacts: Vec<CaptureArtifact>,
    pub observations: Vec<CaptureObservation>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FixtureValidationError {
    violations: Vec<String>,
}

impl FixtureValidationError {
    pub fn violations(&self) -> &[String] {
        &self.violations
    }
}

impl fmt::Display for FixtureValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "capture fixture validation failed: {}",
            self.violations.join("; ")
        )
    }
}

impl std::error::Error for FixtureValidationError {}

impl CaptureManifest {
    pub fn from_json(input: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(input)
    }

    pub fn to_pretty_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    pub fn validate(&self) -> Result<(), FixtureValidationError> {
        let mut violations = Vec::new();

        if self.schema != CAPTURE_FIXTURE_SCHEMA_V1 {
            violations.push(format!(
                "schema must be {CAPTURE_FIXTURE_SCHEMA_V1}, got {}",
                self.schema
            ));
        }
        validate_token("capture_id", &self.capture_id, &mut violations);
        validate_nonempty("captured_at_utc", &self.captured_at_utc, &mut violations);
        validate_nonempty("researcher", &self.researcher, &mut violations);
        validate_nonempty("client.family", &self.client.family, &mut violations);
        validate_nonempty("client.version", &self.client.version, &mut violations);
        if self.client.build == 0 {
            violations.push("client.build must be non-zero".to_owned());
        }
        validate_nonempty("client.platform", &self.client.platform, &mut violations);
        validate_nonempty(
            "client.architecture",
            &self.client.architecture,
            &mut violations,
        );
        validate_sha256(
            "client.executable_sha256",
            &self.client.executable_sha256,
            &mut violations,
        );
        validate_nonempty("os", &self.os, &mut violations);
        validate_nonempty("network_topology", &self.network_topology, &mut violations);
        validate_nonempty(
            "server_under_test",
            &self.server_under_test,
            &mut violations,
        );
        validate_nonempty("server_revision", &self.server_revision, &mut violations);
        validate_nonempty("scenario", &self.scenario, &mut violations);
        validate_nonempty("expected_result", &self.expected_result, &mut violations);
        validate_nonempty("actual_result", &self.actual_result, &mut violations);

        let mut artifact_ids = BTreeSet::new();
        let mut artifact_paths = BTreeSet::new();
        for artifact in &self.artifacts {
            validate_token("artifact.id", &artifact.id, &mut violations);
            if !artifact_ids.insert(artifact.id.as_str()) {
                violations.push(format!("duplicate artifact id: {}", artifact.id));
            }
            validate_relative_path(&artifact.relative_path, &mut violations);
            if !artifact_paths.insert(artifact.relative_path.as_str()) {
                violations.push(format!(
                    "duplicate artifact relative_path: {}",
                    artifact.relative_path
                ));
            }
            validate_nonempty("artifact.media_type", &artifact.media_type, &mut violations);
            validate_sha256("artifact.sha256", &artifact.sha256, &mut violations);
        }

        let mut last_sequence = None;
        let mut last_offset = None;
        for observation in &self.observations {
            if let Some(previous) = last_sequence {
                if observation.sequence <= previous {
                    violations.push(format!(
                        "observation sequence {} is not strictly greater than {}",
                        observation.sequence, previous
                    ));
                }
            }
            last_sequence = Some(observation.sequence);

            if let Some(previous) = last_offset {
                if observation.offset_micros < previous {
                    violations.push(format!(
                        "observation offset {} precedes prior offset {}",
                        observation.offset_micros, previous
                    ));
                }
            }
            last_offset = Some(observation.offset_micros);

            validate_nonempty(
                "observation.operation",
                &observation.operation,
                &mut violations,
            );
            if let Some(artifact_id) = observation.artifact_id.as_deref() {
                if !artifact_ids.contains(artifact_id) {
                    violations.push(format!(
                        "observation {} references unknown artifact id {artifact_id}",
                        observation.sequence
                    ));
                }
            }
            if let Some(digest) = observation.payload_sha256.as_deref() {
                validate_sha256("observation.payload_sha256", digest, &mut violations);
            }
        }

        if violations.is_empty() {
            Ok(())
        } else {
            Err(FixtureValidationError { violations })
        }
    }

    pub fn ensure_implementation_ready(&self) -> Result<(), FixtureValidationError> {
        let mut violations = match self.validate() {
            Ok(()) => Vec::new(),
            Err(error) => error.violations,
        };

        if self.evidence_grade != EvidenceGrade::FirstParty {
            violations.push(
                "implementation-ready target-client fixtures must be first_party evidence"
                    .to_owned(),
            );
        }
        if self.observations.is_empty() {
            violations.push("implementation-ready fixtures require observations".to_owned());
        }
        for artifact in &self.artifacts {
            if artifact.contains_secrets {
                violations.push(format!(
                    "artifact {} is marked as containing secrets",
                    artifact.id
                ));
            }
            if artifact.redaction_status == RedactionStatus::NotReviewed {
                violations.push(format!(
                    "artifact {} has not completed redaction review",
                    artifact.id
                ));
            }
        }

        if violations.is_empty() {
            Ok(())
        } else {
            Err(FixtureValidationError { violations })
        }
    }
}

fn validate_nonempty(field: &str, value: &str, violations: &mut Vec<String>) {
    if value.trim().is_empty() {
        violations.push(format!("{field} must not be empty"));
    }
}

fn validate_token(field: &str, value: &str, violations: &mut Vec<String>) {
    if value.is_empty()
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
    {
        violations.push(format!(
            "{field} must contain only ASCII letters, digits, '-', '_' or '.'"
        ));
    }
}

fn validate_sha256(field: &str, value: &str, violations: &mut Vec<String>) {
    if value.len() != 64 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        violations.push(format!(
            "{field} must be a 64-character hexadecimal SHA-256"
        ));
    }
}

fn validate_relative_path(value: &str, violations: &mut Vec<String>) {
    if value.is_empty() {
        violations.push("artifact.relative_path must not be empty".to_owned());
        return;
    }
    let path = Path::new(value);
    if path.is_absolute()
        || path
            .components()
            .any(|component| matches!(component, std::path::Component::ParentDir))
    {
        violations.push(format!(
            "artifact.relative_path must stay inside the capture bundle: {value}"
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn digest(byte: char) -> String {
        std::iter::repeat_n(byte, 64).collect()
    }

    fn manifest() -> CaptureManifest {
        CaptureManifest {
            schema: CAPTURE_FIXTURE_SCHEMA_V1.to_owned(),
            capture_id: "classic-1.13.2.31650-login-001".to_owned(),
            captured_at_utc: "2026-09-10T12:00:00Z".to_owned(),
            researcher: "realmforge".to_owned(),
            client: ClientDescriptor {
                family: "classic".to_owned(),
                version: "1.13.2".to_owned(),
                build: 31650,
                platform: "windows".to_owned(),
                architecture: "x86_64".to_owned(),
                executable_sha256: digest('a'),
            },
            os: "Windows 10".to_owned(),
            network_topology: "isolated-lab".to_owned(),
            server_under_test: "realmforge-observer".to_owned(),
            server_revision: "capture-only".to_owned(),
            scenario: "cold login attempt".to_owned(),
            expected_result: "observe client network sequence".to_owned(),
            actual_result: "client initiated observed connection".to_owned(),
            evidence_grade: EvidenceGrade::FirstParty,
            artifacts: vec![CaptureArtifact {
                id: "pcap".to_owned(),
                relative_path: "artifacts/login.pcapng".to_owned(),
                media_type: "application/vnd.tcpdump.pcap".to_owned(),
                sha256: digest('b'),
                contains_secrets: false,
                redaction_status: RedactionStatus::Reviewed,
            }],
            observations: vec![CaptureObservation {
                sequence: 1,
                offset_micros: 0,
                direction: ObservationDirection::ClientToServer,
                layer: ObservationLayer::Tcp,
                operation: "connect".to_owned(),
                endpoint: Some("observer.invalid:1119".to_owned()),
                artifact_id: Some("pcap".to_owned()),
                payload_sha256: None,
                notes: None,
            }],
            notes: None,
        }
    }

    #[test]
    fn valid_first_party_manifest_is_implementation_ready() {
        manifest().ensure_implementation_ready().unwrap();
    }

    #[test]
    fn lower_grade_evidence_can_be_recorded_but_not_drive_target_code() {
        let mut manifest = manifest();
        manifest.evidence_grade = EvidenceGrade::Correlated;
        manifest.validate().unwrap();
        let error = manifest.ensure_implementation_ready().unwrap_err();
        assert!(error.to_string().contains("must be first_party evidence"));
    }

    #[test]
    fn secrets_or_unreviewed_artifacts_block_implementation_ready_state() {
        let mut manifest = manifest();
        manifest.artifacts[0].contains_secrets = true;
        manifest.artifacts[0].redaction_status = RedactionStatus::NotReviewed;
        let error = manifest.ensure_implementation_ready().unwrap_err();
        assert!(error.to_string().contains("containing secrets"));
        assert!(error.to_string().contains("redaction review"));
    }

    #[test]
    fn traversal_duplicate_artifacts_and_unknown_references_fail_validation() {
        let mut manifest = manifest();
        manifest.artifacts.push(CaptureArtifact {
            id: "pcap".to_owned(),
            relative_path: "../escape.pcapng".to_owned(),
            media_type: "application/vnd.tcpdump.pcap".to_owned(),
            sha256: digest('c'),
            contains_secrets: false,
            redaction_status: RedactionStatus::Reviewed,
        });
        manifest.observations[0].artifact_id = Some("missing".to_owned());
        let error = manifest.validate().unwrap_err();
        let text = error.to_string();
        assert!(text.contains("duplicate artifact id"));
        assert!(text.contains("must stay inside"));
        assert!(text.contains("unknown artifact id"));
    }

    #[test]
    fn observation_order_is_monotonic() {
        let mut manifest = manifest();
        manifest.observations.push(CaptureObservation {
            sequence: 1,
            offset_micros: 0,
            direction: ObservationDirection::ServerToClient,
            layer: ObservationLayer::Tcp,
            operation: "reply".to_owned(),
            endpoint: None,
            artifact_id: Some("pcap".to_owned()),
            payload_sha256: Some(digest('d')),
            notes: None,
        });
        let error = manifest.validate().unwrap_err();
        assert!(error.to_string().contains("not strictly greater"));
    }

    #[test]
    fn json_round_trip_preserves_manifest() {
        let manifest = manifest();
        let json = manifest.to_pretty_json().unwrap();
        let decoded = CaptureManifest::from_json(&json).unwrap();
        assert_eq!(decoded, manifest);
    }
}
