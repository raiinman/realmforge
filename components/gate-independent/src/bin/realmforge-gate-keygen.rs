use std::{env, fs::OpenOptions, io::Write, path::PathBuf};

use realmforge_gate_independent::OidcSigningAuthority;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let output = env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .ok_or("usage: realmforge-gate-keygen <output-pkcs8-pem>")?;

    let authority = OidcSigningAuthority::generate_2048()?;
    let pem = authority.to_pkcs8_pem()?;
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);

    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }

    let mut file = options.open(&output)?;
    file.write_all(pem.as_bytes())?;
    file.sync_all()?;

    eprintln!(
        "Generated Realmforge Gate RS256 signing key {} at {}",
        authority.key_id(),
        output.display()
    );
    Ok(())
}
