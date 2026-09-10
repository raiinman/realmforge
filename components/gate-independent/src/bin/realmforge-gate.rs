use std::{env, fs};

use realmforge_gate_independent::{
    Issuer, OidcSigningAuthority, ProviderMetadata, gate_http_router,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let bind = env::var("REALMFORGE_GATE_BIND").unwrap_or_else(|_| "127.0.0.1:8080".to_owned());
    let issuer = env::var("REALMFORGE_GATE_ISSUER").unwrap_or_else(|_| format!("http://{bind}"));
    let signing_key_path = env::var("REALMFORGE_GATE_SIGNING_KEY")
        .map_err(|_| "REALMFORGE_GATE_SIGNING_KEY must point to a persistent PKCS#8 PEM key")?;

    let signing_key_pem = fs::read_to_string(&signing_key_path)?;
    let signing = OidcSigningAuthority::from_pkcs8_pem(&signing_key_pem)?;
    let issuer = Issuer::new(issuer)?;
    let metadata = ProviderMetadata::authorization_code(&issuer);
    let app = gate_http_router(metadata, signing.clone());
    let listener = tokio::net::TcpListener::bind(&bind).await?;

    eprintln!(
        "Realmforge Gate listening on {bind} with OIDC signing key {}",
        signing.key_id()
    );
    axum::serve(listener, app).await?;
    Ok(())
}
