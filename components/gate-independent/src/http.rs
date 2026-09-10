use axum::{Json, Router, extract::State, routing::get};
use serde::Serialize;

use crate::ProviderMetadata;

#[derive(Debug, Clone)]
struct HttpState {
    metadata: ProviderMetadata,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct HealthResponse {
    pub status: &'static str,
    pub service: &'static str,
}

pub fn gate_http_router(metadata: ProviderMetadata) -> Router {
    let state = HttpState { metadata };
    Router::new()
        .route("/health", get(health))
        .route("/.well-known/openid-configuration", get(discovery))
        .with_state(state)
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok",
        service: "realmforge-gate",
    })
}

async fn discovery(State(state): State<HttpState>) -> Json<ProviderMetadata> {
    Json(state.metadata)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Issuer;

    #[tokio::test]
    async fn health_is_realmforge_native() {
        let Json(body) = health().await;
        assert_eq!(body.status, "ok");
        assert_eq!(body.service, "realmforge-gate");
    }

    #[tokio::test]
    async fn discovery_returns_configured_provider_metadata() {
        let issuer = Issuer::new("https://gate.realmforge.test").unwrap();
        let expected = ProviderMetadata::authorization_code(&issuer);
        let state = HttpState {
            metadata: expected.clone(),
        };

        let Json(actual) = discovery(State(state)).await;
        assert_eq!(actual, expected);
    }
}
