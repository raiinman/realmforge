use std::{env, fs};

use realmforge_gate_independent::{
    AccountId, AccountRecord, AccountStatus, ClientId, GateHttpState, IdentitySubject, Issuer,
    LoginName, MemoryAccountDirectory, MemoryNativeCredentialStore, NativeCredential,
    NativeLoginService, OAuthClient, OAuthClientRegistry, OAuthService, OidcSigningAuthority,
    ProviderMetadata, RedirectUri, SessionRegistry, gate_http_router_with_state,
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
    let oauth = OAuthService::new(oauth_clients_from_env()?, 60, 3600)?;
    let mut state = GateHttpState::new(metadata, oauth, SessionRegistry::default(), signing.clone());
    let native_login = native_login_from_env()?;
    if let Some(service) = native_login {
        state = state.with_native_login(service);
        eprintln!("Realmforge native browser login enabled");
    }
    let app = gate_http_router_with_state(state);
    let listener = tokio::net::TcpListener::bind(&bind).await?;

    eprintln!(
        "Realmforge Gate listening on {bind} with OIDC signing key {}",
        signing.key_id()
    );
    axum::serve(listener, app).await?;
    Ok(())
}

fn oauth_clients_from_env() -> Result<OAuthClientRegistry, Box<dyn std::error::Error>> {
    let client_id = env::var("REALMFORGE_GATE_OAUTH_CLIENT_ID").ok();
    let redirect_uri = env::var("REALMFORGE_GATE_OAUTH_REDIRECT_URI").ok();
    match (client_id, redirect_uri) {
        (None, None) => Ok(OAuthClientRegistry::default()),
        (Some(client_id), Some(redirect_uri)) => {
            let client = OAuthClient::new(
                ClientId::new(client_id)?,
                [RedirectUri::new(redirect_uri)?],
            )?;
            Ok(OAuthClientRegistry::new([client])?)
        }
        _ => Err(
            "REALMFORGE_GATE_OAUTH_CLIENT_ID and REALMFORGE_GATE_OAUTH_REDIRECT_URI must be configured together"
                .into(),
        ),
    }
}

fn native_login_from_env() -> Result<Option<NativeLoginService>, Box<dyn std::error::Error>> {
    let login = env::var("REALMFORGE_GATE_NATIVE_LOGIN").ok();
    let password_hash = env::var("REALMFORGE_GATE_NATIVE_PASSWORD_HASH").ok();
    let subject = env::var("REALMFORGE_GATE_NATIVE_SUBJECT").ok();

    match (login, password_hash, subject) {
        (None, None, None) => Ok(None),
        (Some(login), Some(password_hash), Some(subject)) => {
            let subject = IdentitySubject::new(subject)?;
            let credential = NativeCredential::from_phc(
                LoginName::new(login)?,
                subject.clone(),
                password_hash,
            )?;
            let account = AccountRecord {
                id: AccountId::new(format!("native-{}", subject.as_str()))?,
                subject,
                status: AccountStatus::Active,
                game_accounts: vec![],
            };
            Ok(Some(NativeLoginService::new(
                MemoryNativeCredentialStore::new([credential])?,
                MemoryAccountDirectory::new([account])?,
            )))
        }
        _ => Err(
            "REALMFORGE_GATE_NATIVE_LOGIN, REALMFORGE_GATE_NATIVE_PASSWORD_HASH, and REALMFORGE_GATE_NATIVE_SUBJECT must be configured together"
                .into(),
        ),
    }
}
