//! HTTP route handlers.
//!
//! This module is organized into submodules by domain:
//! - `health`: Health check and connection status
//! - `people`: People listing, thumbnails, asset counts
//! - `albums`: Album listing and thumbnails
//! - `sources`: Other people and Smart Albums recipients as photo sources
//! - `processing`: Job control (progress, start, cancel)
//! - `preview`: Quality filters and the resulting frame on a single photo
//! - `output`: Output folder and image management
//! - `config`: Configuration get/update
//! - `music`: Music tracks available for the video
//! - `titles`: Intro/outro fonts, images, Immich picker and preview

mod albums;
mod config;
mod health;
mod music;
mod output;
mod people;
mod preview;
mod processing;
mod sources;
mod titles;
mod ws;

use crate::web::state::AppState;
use axum::{
    extract::{DefaultBodyLimit, Request},
    http::{header, HeaderMap, Method, StatusCode},
    middleware::{self, Next},
    response::{Html, IntoResponse, Response},
    routing::{delete, get, post, put},
    Router,
};
use serde::Serialize;
use tower::ServiceBuilder;
use tower_http::services::ServeDir;
use tower_http::set_header::SetResponseHeaderLayer;

// Re-export handler functions for use in router
use albums::{get_album_thumbnail, get_albums};
use config::{get_config, get_config_defaults, update_config};
use health::{check_connection, health_check, version_info};
use music::{list_music_tracks, upload_music_track, MAX_UPLOAD_BYTES};
use output::{
    cleanup_all_output, cleanup_output_folder, compile_folder_video, delete_images_bulk,
    delete_single_image, get_run_info, get_selection, list_folder_images, list_output_folders,
    put_selection,
};
use people::{get_people, get_person_asset_count, get_person_thumbnail};
use preview::{preview, preview_samples};
use processing::{cancel_processing, get_progress, start_processing};
use sources::{get_co_counts, get_recipients};
use titles::{
    get_album_assets, get_asset_thumbnail, get_fonts, get_title_image, get_title_images,
    preview_title, search_assets, upload_title_image, MAX_IMAGE_BYTES,
};
use ws::ws_handler;

// Re-export types that may be needed by other modules
pub use albums::AlbumInfo;
pub use config::ConfigResponse;
pub use health::{ConnectionStatus, VersionInfo};
pub use music::MusicListResponse;
pub use output::{BulkDeleteResponse, FolderImagesResponse, ImageInfo, OutputFolderInfo};
pub use people::{AssetCountResponse, PersonInfo};
pub use processing::{ProgressResponse, SkipStatsResponse};
pub use sources::RecipientsResponse;

/// Common response type for simple success/failure operations.
#[derive(Serialize)]
pub struct StartResponse {
    pub success: bool,
    pub message: String,
}

/// Create the router with all routes.
pub fn create_router(state: AppState) -> Router {
    // Get output directory from config for serving results
    let output_dir = state
        .config
        .try_read()
        .map(|c| c.output_dir.clone())
        .unwrap_or_else(|_| std::path::PathBuf::from("output"));

    Router::new()
        // API routes
        .route("/api/health", get(health_check))
        .route("/api/version", get(version_info))
        .route("/api/connection", get(check_connection))
        .route("/api/people", get(get_people))
        .route(
            "/api/people/{person_id}/thumbnail",
            get(get_person_thumbnail),
        )
        .route(
            "/api/people/{person_id}/asset-count",
            get(get_person_asset_count),
        )
        .route("/api/people/{person_id}/co-counts", get(get_co_counts))
        .route("/api/recipients", get(get_recipients))
        .route("/api/albums", get(get_albums))
        .route(
            "/api/albums/{thumbnail_asset_id}/thumbnail",
            get(get_album_thumbnail),
        )
        .route("/api/progress", get(get_progress))
        .route("/api/ws", get(ws_handler))
        .route("/api/start", post(start_processing))
        .route("/api/preview", post(preview))
        .route("/api/preview/samples", get(preview_samples))
        .route("/api/cancel", post(cancel_processing))
        .route("/api/output", get(list_output_folders))
        .route("/api/output", delete(cleanup_all_output))
        .route("/api/output/{folder_name}", delete(cleanup_output_folder))
        .route("/api/output/{folder_name}/images", get(list_folder_images))
        .route(
            "/api/output/{folder_name}/images",
            delete(delete_images_bulk),
        )
        .route(
            "/api/output/{folder_name}/images/{filename}",
            delete(delete_single_image),
        )
        .route("/api/output/{folder_name}/run", get(get_run_info))
        .route(
            "/api/output/{folder_name}/selection",
            get(get_selection).put(put_selection),
        )
        .route(
            "/api/output/{folder_name}/compile",
            post(compile_folder_video),
        )
        .merge(crate::web::shares::api_routes())
        .route("/api/config", get(get_config))
        .route("/api/config", put(update_config))
        .route("/api/config/defaults", get(get_config_defaults))
        .route("/api/music", get(list_music_tracks))
        .route(
            "/api/music/{filename}",
            post(upload_music_track).layer(DefaultBodyLimit::max(MAX_UPLOAD_BYTES)),
        )
        .route("/api/fonts", get(get_fonts))
        .route("/api/analytics", get(crate::web::analytics::get_analytics))
        .route("/api/titles/images", get(get_title_images))
        .route(
            "/api/titles/images/{filename}",
            get(get_title_image)
                .post(upload_title_image)
                .layer(DefaultBodyLimit::max(MAX_IMAGE_BYTES)),
        )
        .route("/api/titles/preview", post(preview_title))
        .route("/api/album-assets/{album_id}", get(get_album_assets))
        .route("/api/asset-search", get(search_assets))
        .route("/api/assets/{asset_id}/thumbnail", get(get_asset_thumbnail))
        // Serve output files (video, images) — no-cache so browsers revalidate
        // after reprocessing (filenames stay the same across runs)
        .nest_service(
            "/output",
            ServiceBuilder::new()
                .layer(SetResponseHeaderLayer::overriding(
                    header::CACHE_CONTROL,
                    header::HeaderValue::from_static("no-store"),
                ))
                .service(ServeDir::new(output_dir)),
        )
        // Serve frontend static files (fallback to index.html for SPA routing)
        .fallback_service(ServeDir::new("frontend/dist").fallback(get(serve_index)))
        .layer(middleware::from_fn(same_origin_only))
        .with_state(state)
}

/// Refuse changing requests and WebSocket connections from another web page.
///
/// A page on another site, open in a browser in the same network, could
/// otherwise cancel jobs, delete folders or upload files here (the access list
/// of a reverse proxy does not stop that, the browser is allowed in). Browsers
/// always send `Origin` for such requests; tools like curl send none and pass.
async fn same_origin_only(req: Request, next: Next) -> Response {
    let changes = !matches!(*req.method(), Method::GET | Method::HEAD | Method::OPTIONS);
    let websocket = req.headers().contains_key(header::UPGRADE);
    if changes || websocket {
        if let Some(origin) = req.headers().get(header::ORIGIN) {
            let origin = origin.to_str().unwrap_or("null");
            if !origin_allowed(origin, req.headers()) {
                tracing::warn!(
                    "Refused {} {} from origin {}",
                    req.method(),
                    req.uri().path(),
                    origin
                );
                return (StatusCode::FORBIDDEN, "Cross-origin request refused").into_response();
            }
        }
    }
    next.run(req).await
}

/// Whether `origin` is this server: its host matches `Host` or
/// `X-Forwarded-Host` (behind a reverse proxy), or it is the local machine
/// (the Vite dev server proxies with a rewritten `Host`).
fn origin_allowed(origin: &str, headers: &HeaderMap) -> bool {
    let Some((_, host)) = origin.split_once("://") else {
        return false; // "null" and the like
    };
    let host = host.trim_end_matches('/');
    let name = host
        .rsplit_once(':')
        .filter(|(_, port)| port.chars().all(|c| c.is_ascii_digit()))
        .map_or(host, |(name, _)| name);
    if matches!(name, "localhost" | "127.0.0.1" | "[::1]") {
        return true;
    }
    [header::HOST.as_str(), "x-forwarded-host"]
        .iter()
        .any(|key| {
            headers
                .get(*key)
                .and_then(|v| v.to_str().ok())
                .is_some_and(|v| v.split(',').any(|h| h.trim().eq_ignore_ascii_case(host)))
        })
}

/// Serve index.html for SPA routing.
async fn serve_index() -> impl IntoResponse {
    match tokio::fs::read_to_string("frontend/dist/index.html").await {
        Ok(html) => Html(html).into_response(),
        Err(_) => Html(
            r#"<!DOCTYPE html>
<html>
<head><title>Immich Timelapse</title></head>
<body style="font-family: sans-serif; display: flex; justify-content: center; align-items: center; height: 100vh; margin: 0; background: #0f0f0f; color: #e0e0e0;">
  <div style="text-align: center;">
    <h1>Frontend not built</h1>
    <p>Run <code style="background: #333; padding: 0.25rem 0.5rem; border-radius: 4px;">cd frontend && npm install && npm run build</code></p>
    <p style="margin-top: 1rem; color: #888;">Or use <code style="background: #333; padding: 0.25rem 0.5rem; border-radius: 4px;">npm run dev</code> for development</p>
  </div>
</body>
</html>"#,
        )
        .into_response(),
    }
}

#[cfg(test)]
mod origin_tests {
    use super::*;

    fn headers(pairs: &[(&'static str, &str)]) -> HeaderMap {
        let mut map = HeaderMap::new();
        for (k, v) in pairs {
            map.insert(*k, v.parse().unwrap());
        }
        map
    }

    #[test]
    fn same_host_is_allowed() {
        let h = headers(&[("host", "nas.local:5000")]);
        assert!(origin_allowed("http://nas.local:5000", &h));
        assert!(origin_allowed("http://NAS.local:5000/", &h));
    }

    #[test]
    fn forwarded_host_is_allowed() {
        let h = headers(&[
            ("host", "timelapse:5000"),
            ("x-forwarded-host", "timelapse.example.org"),
        ]);
        assert!(origin_allowed("https://timelapse.example.org", &h));
    }

    #[test]
    fn local_dev_server_is_allowed() {
        let h = headers(&[("host", "localhost:5000")]);
        assert!(origin_allowed("http://localhost:5173", &h));
        assert!(origin_allowed("http://127.0.0.1:5173", &h));
    }

    #[test]
    fn other_sites_are_refused() {
        let h = headers(&[("host", "nas.local:5000")]);
        assert!(!origin_allowed("https://evil.example", &h));
        assert!(!origin_allowed("http://nas.local:5001", &h));
        assert!(!origin_allowed("http://nas.local.evil.example:5000", &h));
        assert!(!origin_allowed("null", &h));
    }
}
