use std::env;

use realmforge_gate_independent::{Issuer, ProviderMetadata, gate_http_router};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let bind = env::var("REALMFORGE_GATE_BIND").unwrap_or_else(|_| "127.0.0.1:8080".to_owned());
    let issuer = env::var("REALMFORGE_GATE_ISSUER")
        .unwrap_or_else(|_| format!("http://{bind}"));

    let issuer = Issuer::new(issuer)?;
    let metadata = ProviderMetadata::authorization_code(&issuer);
    let app = gate_http_router(metadata);
    let listener = tokio::net::TcpListener::bind(&bind).await?;

    eprintln!("Realmforge Gate listening on {bind}");
    axum::serve(listener, app).await?;
    Ok(())
}
