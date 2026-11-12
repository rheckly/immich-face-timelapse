//! Health check and connection status endpoints.

use crate::immich_api::ImmichClient;
use crate::web::state::AppState;
use axum::{extract::State, response::Json};
use serde::Serialize;

/// Health check endpoint.
pub async fn health_check() -> &'static str {
    "OK"
}

/// Build information of the running server.
///
/// The values are baked in at compile time. The Docker build passes them as
/// build args (see Dockerfile and .github/workflows/ghcr.yml); local builds
/// fall back to the Cargo package version.
#[derive(Serialize)]
pub struct VersionInfo {
    pub version: &'static str,
    pub commit: Option<&'static str>,
    pub build_date: Option<&'static str>,
}

fn non_empty(value: Option<&'static str>) -> Option<&'static str> {
    value.filter(|v| !v.trim().is_empty())
}

impl VersionInfo {
    pub fn current() -> Self {
        Self {
            version: non_empty(option_env!("APP_VERSION")).unwrap_or(env!("CARGO_PKG_VERSION")),
            commit: non_empty(option_env!("GIT_SHA")),
            build_date: non_empty(option_env!("BUILD_DATE")),
        }
    }
}

/// Version endpoint: which build is live.
pub async fn version_info() -> Json<VersionInfo> {
    Json(VersionInfo::current())
}

/// Connection status response.
#[derive(Serialize)]
pub struct ConnectionStatus {
    pub connected: bool,
    pub version: Option<String>,
    pub error: Option<String>,
}

/// Check connection to Immich.
pub async fn check_connection(State(state): State<AppState>) -> Json<ConnectionStatus> {
    let config = state.config.read().await;
    let client = match ImmichClient::new(&config.api) {
        Ok(c) => c,
        Err(e) => {
            return Json(ConnectionStatus {
                connected: false,
                version: None,
                error: Some(e.to_string()),
            });
        }
    };

    match client.validate_connection().await {
        Ok(info) => Json(ConnectionStatus {
            connected: true,
            version: Some(info.version),
            error: None,
        }),
        Err(e) => Json(ConnectionStatus {
            connected: false,
            version: None,
            error: Some(e.to_string()),
        }),
    }
}
