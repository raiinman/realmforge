use std::env;
use std::fs;

use realmforge_gate_independent::CaptureManifest;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = env::args().skip(1);
    let path = args
        .next()
        .ok_or("usage: realmforge-gate-fixture-check <manifest.json> [--implementation-ready]")?;
    let implementation_ready = match args.next().as_deref() {
        None => false,
        Some("--implementation-ready") => true,
        Some(_) => {
            return Err(
                "usage: realmforge-gate-fixture-check <manifest.json> [--implementation-ready]"
                    .into(),
            );
        }
    };
    if args.next().is_some() {
        return Err(
            "usage: realmforge-gate-fixture-check <manifest.json> [--implementation-ready]".into(),
        );
    }

    let json = fs::read_to_string(&path)?;
    let manifest = CaptureManifest::from_json(&json)?;
    if implementation_ready {
        manifest.ensure_implementation_ready()?;
        println!("{}: implementation-ready", manifest.capture_id);
    } else {
        manifest.validate()?;
        println!("{}: valid", manifest.capture_id);
    }
    Ok(())
}
