use std::collections::BTreeMap;
use std::env;
use std::fs::{self, File};
use std::io::Read;
use std::path::{Path, PathBuf};

use realmforge_gate_independent::{
    CAPTURE_FIXTURE_SCHEMA_V1, CaptureManifest, ClientDescriptor, EvidenceGrade,
};
use sha2::{Digest, Sha256};
use time::{OffsetDateTime, format_description::well_known::Rfc3339};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = parse_flags()?;
    if args.contains_key("help") {
        print_usage();
        return Ok(());
    }

    let out = PathBuf::from(required(&args, "out")?);
    let manifest_path = out.join("manifest.json");
    if manifest_path.exists() {
        return Err(format!("refusing to overwrite existing {}", manifest_path.display()).into());
    }

    let executable = PathBuf::from(required(&args, "exe")?);
    if !executable.is_file() {
        return Err(format!("client executable not found: {}", executable.display()).into());
    }
    let executable_sha256 = sha256_file(&executable)?;
    let build: u32 = required(&args, "build")?
        .parse()
        .map_err(|_| "--build must be a positive integer")?;
    if build == 0 {
        return Err("--build must be greater than zero".into());
    }

    let captured_at_utc = OffsetDateTime::now_utc().format(&Rfc3339)?;
    let manifest = CaptureManifest {
        schema: CAPTURE_FIXTURE_SCHEMA_V1.to_owned(),
        capture_id: required(&args, "id")?.to_owned(),
        captured_at_utc,
        researcher: args
            .get("researcher")
            .cloned()
            .unwrap_or_else(|| "realmforge".to_owned()),
        client: ClientDescriptor {
            family: required(&args, "family")?.to_owned(),
            version: required(&args, "version")?.to_owned(),
            build,
            platform: required(&args, "platform")?.to_owned(),
            architecture: required(&args, "arch")?.to_owned(),
            executable_sha256,
        },
        os: required(&args, "os")?.to_owned(),
        network_topology: required(&args, "topology")?.to_owned(),
        server_under_test: args
            .get("server")
            .cloned()
            .unwrap_or_else(|| "realmforge-observer".to_owned()),
        server_revision: required(&args, "server-revision")?.to_owned(),
        scenario: required(&args, "scenario")?.to_owned(),
        expected_result: required(&args, "expected")?.to_owned(),
        actual_result: "capture initialized; no observations recorded yet".to_owned(),
        evidence_grade: EvidenceGrade::FirstParty,
        artifacts: vec![],
        observations: vec![],
        notes: args.get("notes").cloned(),
    };
    manifest.validate()?;

    fs::create_dir_all(&out)?;
    let json = manifest.to_pretty_json()?;
    fs::write(&manifest_path, format!("{json}\n"))?;

    println!("capture_id={}", manifest.capture_id);
    println!("client_build={}", manifest.client.build);
    println!("client_sha256={}", manifest.client.executable_sha256);
    println!("captured_at_utc={}", manifest.captured_at_utc);
    println!("manifest={}", manifest_path.display());
    println!("implementation_ready=false");
    eprintln!(
        "Bundle initialized. Record observations/artifacts before implementation-ready validation."
    );
    Ok(())
}

fn parse_flags() -> Result<BTreeMap<String, String>, Box<dyn std::error::Error>> {
    let mut parsed = BTreeMap::new();
    let mut args = env::args().skip(1);
    while let Some(flag) = args.next() {
        if matches!(flag.as_str(), "--help" | "-h") {
            parsed.insert("help".to_owned(), String::new());
            continue;
        }
        let Some(name) = flag.strip_prefix("--") else {
            return Err(format!("unexpected positional argument: {flag}").into());
        };
        if name.is_empty() {
            return Err("empty flag name".into());
        }
        if parsed.contains_key(name) {
            return Err(format!("duplicate flag: --{name}").into());
        }
        let value = args
            .next()
            .ok_or_else(|| format!("--{name} requires a value"))?;
        parsed.insert(name.to_owned(), value);
    }
    Ok(parsed)
}

fn required<'a>(
    args: &'a BTreeMap<String, String>,
    name: &str,
) -> Result<&'a str, Box<dyn std::error::Error>> {
    args.get(name)
        .map(String::as_str)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| format!("--{name} is required").into())
}

fn sha256_file(path: &Path) -> Result<String, Box<dyn std::error::Error>> {
    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let count = file.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);
    }
    let digest = hasher.finalize();
    let mut output = String::with_capacity(64);
    for byte in digest {
        use std::fmt::Write as _;
        write!(&mut output, "{byte:02x}").expect("writing to String cannot fail");
    }
    Ok(output)
}

fn print_usage() {
    println!(
        r#"Realmforge target-client capture bundle initializer

Required:
  --out <bundle-directory>
  --id <capture-id>
  --family <client-family>
  --version <client-version>
  --build <numeric-build>
  --platform <platform>
  --arch <architecture>
  --exe <path-to-client-executable>
  --os <operating-system>
  --topology <network-topology>
  --server-revision <observer-or-lab-revision>
  --scenario <capture-scenario>
  --expected <expected-result>

Optional:
  --researcher <name>           default: realmforge
  --server <server-under-test>  default: realmforge-observer
  --notes <text>

Example:
  realmforge-gate-capture-init \
    --out captures/cata-4.4.2.60895/cold-start-001 \
    --id cata-4.4.2.60895-cold-start-001 \
    --family cata-classic \
    --version 4.4.2 \
    --build 60895 \
    --platform windows \
    --arch x86_64 \
    --exe 'C:\\Games\\World of Warcraft\\_classic_\\WowClassic.exe' \
    --os 'Windows 11' \
    --topology isolated-lab \
    --server-revision <realmforge-commit> \
    --scenario 'cold-start endpoint observation' \
    --expected 'record initial client network behavior'
"#
    );
}
