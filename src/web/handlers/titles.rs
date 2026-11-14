//! Intro / outro endpoints: fonts, background images (upload, Immich picker)
//! and a still preview of a card.

use crate::config::TitleCardConfig;
use crate::immich_api::{ImmichClient, PickAsset};
use crate::video::titles::{
    is_title_image, list_fonts, list_title_images, render_preview, title_image_path, FontInfo,
    TITLE_IMAGES_DIR,
};
use crate::web::state::AppState;
use axum::{
    body::{Body, Bytes},
    extract::{Path, Query, State},
    http::{header, StatusCode},
    response::{Json, Response},
};
use serde::{Deserialize, Serialize};

/// Largest title image accepted by the upload, in bytes.
pub const MAX_IMAGE_BYTES: usize = 40 * 1024 * 1024;

/// Long edge of the preview image in pixels.
const PREVIEW_EDGE: u32 = 640;

type ApiError = (StatusCode, String);

fn internal(e: impl std::fmt::Display) -> ApiError {
    (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
}

fn image_response(bytes: impl Into<Body>, content_type: &str, cache: &str) -> Result<Response<Body>, ApiError> {
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, content_type)
        .header(header::CACHE_CONTROL, cache)
        .body(bytes.into())
        .map_err(internal)
}

async fn client(state: &AppState) -> Result<ImmichClient, ApiError> {
    let config = state.config.read().await;
    ImmichClient::new(&config.api).map_err(internal)
}

/// `GET /api/fonts`: fonts available for the titles.
pub async fn get_fonts() -> Json<Vec<FontInfo>> {
    Json(tokio::task::spawn_blocking(list_fonts).await.unwrap_or_default())
}

/// `GET /api/titles/images`: uploaded title images.
pub async fn get_title_images() -> Result<Json<Vec<String>>, ApiError> {
    list_title_images().await.map(Json).map_err(internal)
}

/// `GET /api/titles/images/{filename}`: an uploaded title image.
pub async fn get_title_image(Path(filename): Path<String>) -> Result<Response<Body>, ApiError> {
    let path = title_image_path(&filename)
        .ok_or((StatusCode::BAD_REQUEST, "Invalid image name".to_string()))?;
    let bytes = tokio::fs::read(&path)
        .await
        .map_err(|_| (StatusCode::NOT_FOUND, format!("'{}' not found", filename)))?;
    let ext = filename.rsplit_once('.').map(|(_, e)| e.to_ascii_lowercase()).unwrap_or_default();
    let content_type = match ext.as_str() {
        "png" => "image/png",
        "webp" => "image/webp",
        _ => "image/jpeg",
    };
    image_response(bytes, content_type, "private, max-age=3600")
}

/// Answer of an upload: the name the image was saved under.
#[derive(Serialize)]
pub struct UploadedImage {
    pub name: String,
}

/// `POST /api/titles/images/{filename}` with the image as body. An existing
/// name gets a number added (`bild-2.jpg`).
pub async fn upload_title_image(
    Path(filename): Path<String>,
    body: Bytes,
) -> Result<Json<UploadedImage>, ApiError> {
    if !is_title_image(&filename) {
        return Err((
            StatusCode::BAD_REQUEST,
            "Only JPG, PNG or WebP images can be uploaded".to_string(),
        ));
    }
    if body.is_empty() {
        return Err((StatusCode::BAD_REQUEST, "The file is empty".to_string()));
    }
    if image::guess_format(&body).is_err() {
        return Err((StatusCode::BAD_REQUEST, "The file is not an image".to_string()));
    }
    tokio::fs::create_dir_all(TITLE_IMAGES_DIR).await.map_err(internal)?;

    let (stem, ext) = filename.rsplit_once('.').unwrap_or((filename.as_str(), "jpg"));
    let mut name = filename.clone();
    let mut n = 2;
    while tokio::fs::metadata(std::path::Path::new(TITLE_IMAGES_DIR).join(&name)).await.is_ok() {
        name = format!("{}-{}.{}", stem, n, ext);
        n += 1;
    }
    let target = std::path::Path::new(TITLE_IMAGES_DIR).join(&name);
    let temp = std::path::Path::new(TITLE_IMAGES_DIR).join(format!(".upload-{}.part", name));
    if let Err(e) = tokio::fs::write(&temp, &body).await {
        let _ = tokio::fs::remove_file(&temp).await;
        return Err(internal(format!("Could not save '{}': {}", name, e)));
    }
    if let Err(e) = tokio::fs::rename(&temp, &target).await {
        let _ = tokio::fs::remove_file(&temp).await;
        return Err(internal(format!("Could not save '{}': {}", name, e)));
    }
    tracing::info!("Uploaded title image {} ({} bytes)", name, body.len());
    Ok(Json(UploadedImage { name }))
}

/// Body of `POST /api/titles/preview`.
#[derive(Deserialize)]
pub struct PreviewRequest {
    pub card: TitleCardConfig,
}

/// `POST /api/titles/preview`: one JPEG of the card with all text visible,
/// in the aspect ratio of the output.
pub async fn preview_title(
    State(state): State<AppState>,
    Json(req): Json<PreviewRequest>,
) -> Result<Response<Body>, ApiError> {
    req.card
        .validate("Card")
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
    let (api, (w, h)) = {
        let config = state.config.read().await;
        (config.api.clone(), config.processing.output.dimensions())
    };
    let scale = PREVIEW_EDGE as f64 / w.max(h) as f64;
    let even = |v: u32| ((v as f64 * scale).round() as u32 & !1).max(2);
    let jpeg = render_preview(&req.card, &api, even(w), even(h))
        .await
        .map_err(internal)?;
    image_response(jpeg, "image/jpeg", "no-store")
}

/// `GET /api/album-assets/{album_id}`: photos of an album for the picker.
pub async fn get_album_assets(
    State(state): State<AppState>,
    Path(album_id): Path<String>,
) -> Result<Json<Vec<PickAsset>>, ApiError> {
    client(&state)
        .await?
        .get_album_assets(&album_id)
        .await
        .map(Json)
        .map_err(|e| (StatusCode::BAD_GATEWAY, e.to_string()))
}

#[derive(Deserialize)]
pub struct SearchQuery {
    pub q: String,
}

/// `GET /api/asset-search?q=…`: smart search over the whole library.
pub async fn search_assets(
    State(state): State<AppState>,
    Query(query): Query<SearchQuery>,
) -> Result<Json<Vec<PickAsset>>, ApiError> {
    let q = query.q.trim();
    if q.is_empty() || q.len() > 200 {
        return Err((StatusCode::BAD_REQUEST, "Enter a search text".to_string()));
    }
    client(&state)
        .await?
        .smart_search(q, 200)
        .await
        .map(Json)
        .map_err(|e| (StatusCode::BAD_GATEWAY, e.to_string()))
}

/// `GET /api/assets/{id}/thumbnail`: small thumbnail of any photo.
pub async fn get_asset_thumbnail(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Response<Body>, ApiError> {
    if id.is_empty() || id.len() > 64 || !id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
        return Err((StatusCode::BAD_REQUEST, "Invalid asset id".to_string()));
    }
    let (bytes, content_type) = client(&state)
        .await?
        .get_asset_thumbnail(&id)
        .await
        .map_err(|e| (StatusCode::NOT_FOUND, e.to_string()))?;
    image_response(bytes, &content_type, "private, max-age=86400")
}
