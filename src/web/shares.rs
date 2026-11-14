//! Shared links for finished videos.
//!
//! A share is a random token pointing at an output folder, with an optional
//! expiry. Shares live in `config/shares.json` together with the public
//! address under which they are reachable.
//!
//! Two routers use them:
//! - [`api_routes`] on the main (internal) port: create, change, revoke.
//! - [`public_router`] on a separate port, meant to be published on the
//!   internet: serves only `/s/<token>` (a page with player and link
//!   preview), its video and its poster image, and - when the link allows
//!   it - the edited photos as ZIP. Nothing else is reachable.

use crate::config::CONFIG_PATH;
use crate::web::state::{validate_path_component, AppState};
use axum::{
    body::Body,
    extract::{Path, Request, State},
    http::{header, HeaderMap, HeaderValue, StatusCode},
    response::{Html, IntoResponse, Json, Response},
    routing::{get, patch, put},
    Router,
};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tower::ServiceExt;
use tower_http::services::ServeFile;
use tower_http::set_header::SetResponseHeaderLayer;

/// Port of the public listener, overridable via `SHARE_PORT`.
pub const DEFAULT_SHARE_PORT: u16 = 5001;

/// Longest validity that can be chosen, in days.
const MAX_DAYS: u32 = 3650;

/// Poster image written next to the video on first request.
const POSTER_NAME: &str = "share-poster.jpg";
/// Edited photos of a folder as ZIP, made on the first download.
const PHOTOS_ZIP: &str = "share-photos.zip";
/// One ZIP is made at a time.
static ZIP_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

/// Serializes reads and writes of the share file.
static LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

/// One shared link.
#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub struct Share {
    pub token: String,
    pub folder: String,
    #[serde(default)]
    pub title: String,
    pub created_at: DateTime<Utc>,
    /// `None` = valid until revoked.
    #[serde(default)]
    pub expires_at: Option<DateTime<Utc>>,
    /// The share page offers a download button.
    #[serde(default)]
    pub download: bool,
    /// The share page offers the edited photos as ZIP.
    #[serde(default)]
    pub photos: bool,
}

impl Share {
    fn is_expired(&self, now: DateTime<Utc>) -> bool {
        self.expires_at.is_some_and(|e| e <= now)
    }
}

/// Content of `config/shares.json`.
#[derive(Serialize, Deserialize, Default, Debug)]
struct ShareStore {
    /// Public address of the share listener, e.g. `https://share.example.com`.
    #[serde(default)]
    public_url: String,
    #[serde(default)]
    shares: Vec<Share>,
}

fn store_path() -> PathBuf {
    std::path::Path::new(CONFIG_PATH).with_file_name("shares.json")
}

async fn load() -> ShareStore {
    match tokio::fs::read_to_string(store_path()).await {
        Ok(text) => serde_json::from_str(&text).unwrap_or_else(|e| {
            tracing::warn!("Ignoring unreadable {}: {}", store_path().display(), e);
            ShareStore::default()
        }),
        Err(_) => ShareStore::default(),
    }
}

async fn save(store: &ShareStore) -> Result<(), (StatusCode, String)> {
    let path = store_path();
    if let Some(dir) = path.parent() {
        let _ = tokio::fs::create_dir_all(dir).await;
    }
    let text = serde_json::to_string_pretty(store).map_err(internal)?;
    let tmp = path.with_extension("json.tmp");
    tokio::fs::write(&tmp, text).await.map_err(internal)?;
    tokio::fs::rename(&tmp, &path).await.map_err(internal)
}

fn internal(e: impl std::fmt::Display) -> (StatusCode, String) {
    (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
}

fn new_token() -> String {
    uuid::Uuid::new_v4().simple().to_string()
}

fn valid_token(token: &str) -> bool {
    token.len() == 32 && token.bytes().all(|b| b.is_ascii_hexdigit())
}

/// Normalize the public address: trimmed, no trailing slash, http(s) only.
fn normalize_url(url: &str) -> Result<String, String> {
    let url = url.trim().trim_end_matches('/');
    if url.is_empty() {
        return Ok(String::new());
    }
    let url = if url.contains("://") {
        url.to_string()
    } else {
        format!("https://{url}")
    };
    if !(url.starts_with("https://") || url.starts_with("http://")) {
        return Err("Die Adresse muss mit https:// beginnen".into());
    }
    if url
        .chars()
        .any(|c| c.is_whitespace() || c == '"' || c == '<' || c == '>')
    {
        return Err("Ungültige Adresse".into());
    }
    Ok(url)
}

fn expiry(
    days: Option<u32>,
    now: DateTime<Utc>,
) -> Result<Option<DateTime<Utc>>, (StatusCode, String)> {
    match days {
        None => Ok(None),
        Some(d) if d == 0 || d > MAX_DAYS => Err((
            StatusCode::BAD_REQUEST,
            format!("days must be between 1 and {MAX_DAYS}"),
        )),
        Some(d) => Ok(Some(now + Duration::days(i64::from(d)))),
    }
}

fn video_path(output_dir: &std::path::Path, folder: &str) -> PathBuf {
    output_dir.join(folder).join(format!("{folder}.mp4"))
}

// ---------------------------------------------------------------------------
// Internal API
// ---------------------------------------------------------------------------

/// A share as the UI sees it.
#[derive(Serialize)]
pub struct ShareInfo {
    #[serde(flatten)]
    pub share: Share,
    /// Full link, `None` while no public address is set.
    pub url: Option<String>,
}

#[derive(Serialize)]
pub struct SharesResponse {
    pub public_url: String,
    pub share_port: u16,
    pub shares: Vec<ShareInfo>,
}

fn info(share: Share, public_url: &str) -> ShareInfo {
    let url = (!public_url.is_empty()).then(|| format!("{public_url}/s/{}", share.token));
    ShareInfo { share, url }
}

pub fn share_port() -> u16 {
    std::env::var("SHARE_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(DEFAULT_SHARE_PORT)
}

/// Routes for the main port, under `/api/shares`.
pub fn api_routes() -> Router<AppState> {
    Router::new()
        .route("/api/shares", get(list_shares).post(create_share))
        .route("/api/shares/settings", put(update_settings))
        .route(
            "/api/shares/{token}",
            patch(update_share).delete(delete_share),
        )
}

/// All active shares. Expired ones and those whose video is gone are dropped.
async fn list_shares(
    State(state): State<AppState>,
) -> Result<Json<SharesResponse>, (StatusCode, String)> {
    let output_dir = state.config.read().await.output_dir.clone();
    let _guard = LOCK.lock().await;
    let mut store = load().await;
    let now = Utc::now();
    let before = store.shares.len();
    let mut keep = Vec::with_capacity(before);
    for s in store.shares.drain(..) {
        if !s.is_expired(now)
            && tokio::fs::metadata(video_path(&output_dir, &s.folder))
                .await
                .is_ok()
        {
            keep.push(s);
        }
    }
    store.shares = keep;
    if store.shares.len() != before {
        save(&store).await?;
    }
    let public_url = store.public_url.clone();
    Ok(Json(SharesResponse {
        shares: store
            .shares
            .into_iter()
            .map(|s| info(s, &public_url))
            .collect(),
        public_url,
        share_port: share_port(),
    }))
}

#[derive(Deserialize)]
pub struct SettingsRequest {
    pub public_url: String,
}

async fn update_settings(
    Json(req): Json<SettingsRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let url = normalize_url(&req.public_url).map_err(|e| (StatusCode::BAD_REQUEST, e))?;
    let _guard = LOCK.lock().await;
    let mut store = load().await;
    store.public_url = url.clone();
    save(&store).await?;
    Ok(Json(serde_json::json!({ "public_url": url })))
}

#[derive(Deserialize)]
pub struct CreateRequest {
    pub folder: String,
    #[serde(default)]
    pub title: String,
    /// Validity in days, `None` = until revoked.
    pub days: Option<u32>,
    #[serde(default)]
    pub download: bool,
    #[serde(default)]
    pub photos: bool,
}

async fn create_share(
    State(state): State<AppState>,
    Json(req): Json<CreateRequest>,
) -> Result<Json<ShareInfo>, (StatusCode, String)> {
    validate_path_component(&req.folder, "folder name")
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
    let output_dir = state.config.read().await.output_dir.clone();
    if tokio::fs::metadata(video_path(&output_dir, &req.folder))
        .await
        .is_err()
    {
        return Err((
            StatusCode::NOT_FOUND,
            "Dieser Zeitraffer hat kein Video".into(),
        ));
    }
    let now = Utc::now();
    let share = Share {
        token: new_token(),
        folder: req.folder,
        title: req.title.trim().chars().take(200).collect(),
        created_at: now,
        expires_at: expiry(req.days, now)?,
        download: req.download,
        photos: req.photos,
    };
    let _guard = LOCK.lock().await;
    let mut store = load().await;
    store.shares.push(share.clone());
    save(&store).await?;
    tracing::info!("Shared {} until {:?}", share.folder, share.expires_at);
    Ok(Json(info(share, &store.public_url)))
}

#[derive(Deserialize)]
pub struct UpdateRequest {
    pub title: Option<String>,
    /// New validity counted from now; `null` = until revoked, missing = keep.
    #[serde(default, deserialize_with = "present")]
    pub days: Option<Option<u32>>,
    pub download: Option<bool>,
    pub photos: Option<bool>,
}

/// Tells a field sent as `null` (`Some(None)`) from a missing one (`None`).
fn present<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Option<Option<u32>>, D::Error> {
    Option::<u32>::deserialize(d).map(Some)
}

async fn update_share(
    Path(token): Path<String>,
    Json(req): Json<UpdateRequest>,
) -> Result<Json<ShareInfo>, (StatusCode, String)> {
    let now = Utc::now();
    let expires_at = match req.days {
        Some(days) => Some(expiry(days, now)?),
        None => None,
    };
    let _guard = LOCK.lock().await;
    let mut store = load().await;
    let share = store
        .shares
        .iter_mut()
        .find(|s| s.token == token)
        .ok_or((StatusCode::NOT_FOUND, "Link nicht gefunden".to_string()))?;
    if let Some(e) = expires_at {
        share.expires_at = e;
    }
    if let Some(d) = req.download {
        share.download = d;
    }
    if let Some(p) = req.photos {
        share.photos = p;
    }
    if let Some(title) = req.title {
        share.title = title.trim().chars().take(200).collect();
    }
    let share = share.clone();
    save(&store).await?;
    Ok(Json(info(share, &store.public_url)))
}

async fn delete_share(Path(token): Path<String>) -> Result<StatusCode, (StatusCode, String)> {
    let _guard = LOCK.lock().await;
    let mut store = load().await;
    let before = store.shares.len();
    store.shares.retain(|s| s.token != token);
    if store.shares.len() == before {
        return Err((StatusCode::NOT_FOUND, "Link nicht gefunden".into()));
    }
    save(&store).await?;
    tracing::info!("Share revoked");
    Ok(StatusCode::NO_CONTENT)
}

// ---------------------------------------------------------------------------
// Public listener
// ---------------------------------------------------------------------------

/// Router for the public port: only shared links, nothing else.
pub fn public_router(state: AppState) -> Router {
    Router::new()
        .route("/s/{token}", get(share_page))
        .route("/s/{token}/video.mp4", get(share_video))
        .route("/s/{token}/poster.jpg", get(share_poster))
        .route("/s/{token}/photos.zip", get(share_photos))
        .route(
            "/robots.txt",
            get(|| async { "User-agent: *\nDisallow: /\n" }),
        )
        .fallback(|| async {
            message_page(
                StatusCode::NOT_FOUND,
                "Hier liegt kein Zeitraffer",
                &["Diese Adresse allein zeigt nichts an. Öffne den ganzen Link, den du bekommen hast. Er sieht etwa so aus: …/s/ und dann eine lange Folge von Buchstaben und Zahlen."],
            )
        })
        .layer(SetResponseHeaderLayer::overriding(
            header::HeaderName::from_static("x-robots-tag"),
            HeaderValue::from_static("noindex, nofollow"),
        ))
        .layer(SetResponseHeaderLayer::overriding(
            header::REFERRER_POLICY,
            HeaderValue::from_static("no-referrer"),
        ))
        .with_state(state)
}

/// Why a link can't be opened.
enum Lookup {
    Found(Share, PathBuf),
    Expired,
    Missing,
}

async fn lookup(state: &AppState, token: &str) -> Lookup {
    if !valid_token(token) {
        return Lookup::Missing;
    }
    let output_dir = state.config.read().await.output_dir.clone();
    let share = {
        let _guard = LOCK.lock().await;
        load().await.shares.into_iter().find(|s| s.token == token)
    };
    let Some(share) = share else {
        return Lookup::Missing;
    };
    if share.is_expired(Utc::now()) {
        return Lookup::Expired;
    }
    if validate_path_component(&share.folder, "folder name").is_err() {
        return Lookup::Missing;
    }
    let video = video_path(&output_dir, &share.folder);
    if tokio::fs::metadata(&video).await.is_err() {
        return Lookup::Missing;
    }
    Lookup::Found(share, video)
}

fn not_available(l: Lookup) -> Response {
    // Same text for expired, revoked and unknown links (like Mosaik-Share);
    // only the status code tells them apart.
    let status = match l {
        Lookup::Expired => StatusCode::GONE,
        _ => StatusCode::NOT_FOUND,
    };
    message_page(
        status,
        "Dieser Link gilt nicht (mehr)",
        &[
            "Der Link ist abgelaufen, wurde zurückgezogen oder ist nicht vollständig kopiert.",
            "Frag die Person, die ihn dir geschickt hat, nach einem neuen Link.",
        ],
    )
}

/// Address of this listener as the visitor sees it (behind the proxy).
fn origin(headers: &HeaderMap) -> String {
    let get = |name: &str| {
        headers
            .get(name)
            .and_then(|v| v.to_str().ok())
            .map(|v| v.split(',').next().unwrap_or("").trim().to_string())
            .filter(|v| !v.is_empty())
    };
    let host = get("x-forwarded-host")
        .or_else(|| get("host"))
        .unwrap_or_else(|| "localhost".into());
    let host: String = host
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | ':' | '[' | ']'))
        .collect();
    let proto = match get("x-forwarded-proto").as_deref() {
        Some("http") => "http",
        Some(_) => "https",
        None if host.starts_with("localhost")
            || host.starts_with("127.")
            || host.starts_with("192.168.") =>
        {
            "http"
        }
        None => "https",
    };
    format!("{proto}://{host}")
}

fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

fn expiry_text(share: &Share) -> String {
    match share.expires_at {
        Some(e) => format!("Link gültig bis {}", e.format("%d.%m.%Y")),
        None => String::new(),
    }
}

async fn share_page(
    State(state): State<AppState>,
    Path(token): Path<String>,
    headers: HeaderMap,
) -> Response {
    let (share, _) = match lookup(&state, &token).await {
        Lookup::Found(s, v) => (s, v),
        other => return not_available(other),
    };
    let base = format!("{}/s/{}", origin(&headers), share.token);
    let title = if share.title.is_empty() {
        "Zeitraffer".to_string()
    } else {
        share.title.clone()
    };
    let t = esc(&title);
    let desc = esc(&expiry_text(&share));
    let mut buttons = String::new();
    if share.download {
        buttons += &format!(
            "<a class=\"dl\" href=\"{base}/video.mp4?download=1\" download=\"{}.mp4\">&#8681; Video herunterladen</a>",
            esc(&download_name(&share))
        );
    }
    if share.photos {
        buttons += &format!(
            "<a class=\"dl\" href=\"{base}/photos.zip\" download=\"{}-Fotos.zip\">&#8681; Fotos herunterladen (ZIP)</a>",
            esc(&download_name(&share))
        );
    }
    let download = if buttons.is_empty() {
        String::new()
    } else {
        format!("<div class=\"dls\">{buttons}</div>\n  ")
    };
    let html = format!(
        r#"<!DOCTYPE html>
<html lang="de">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<meta name="robots" content="noindex, nofollow">
<title>{t}</title>
<meta property="og:type" content="video.other">
<meta property="og:title" content="{t}">
<meta property="og:description" content="{desc}">
<meta property="og:url" content="{base}">
<meta property="og:image" content="{base}/poster.jpg">
<meta property="og:image:type" content="image/jpeg">
<meta property="og:video" content="{base}/video.mp4">
<meta property="og:video:secure_url" content="{base}/video.mp4">
<meta property="og:video:type" content="video/mp4">
<meta name="twitter:card" content="summary_large_image">
<style>
  :root {{ color-scheme: dark; }}
  html, body {{ margin: 0; background: #0f0f10; color: #e8e8ea; font-family: system-ui, -apple-system, "Segoe UI", sans-serif; }}
  main {{ min-height: 100vh; box-sizing: border-box; display: flex; flex-direction: column; align-items: center; justify-content: center; gap: 14px; padding: 16px; }}
  h1 {{ font-size: 1.15rem; font-weight: 600; margin: 0; text-align: center; }}
  video {{ width: 100%; max-width: min(960px, calc((100vh - 120px) * 1)); max-height: calc(100vh - 120px); border-radius: 10px; background: #000; }}
  p {{ margin: 0; color: #9a9aa2; font-size: .85rem; }}
  .dl {{ display: inline-flex; align-items: center; gap: 8px; padding: 9px 18px; border-radius: 999px; background: #2a2a2e; color: #e8e8ea; text-decoration: none; font-size: .9rem; font-weight: 600; }}
  .dl:hover {{ background: #3a3a40; }}
  .dls {{ display: flex; flex-wrap: wrap; gap: 10px; justify-content: center; }}
</style>
</head>
<body>
<main>
  <h1>{t}</h1>
  <video src="{base}/video.mp4" poster="{base}/poster.jpg" controls playsinline preload="metadata"></video>
  {download}<p>{desc}</p>
</main>
</body>
</html>"#
    );
    let mut res = Html(html).into_response();
    res.headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    res
}

async fn share_video(
    State(state): State<AppState>,
    Path(token): Path<String>,
    req: Request,
) -> Response {
    let (share, video) = match lookup(&state, &token).await {
        Lookup::Found(s, v) => (s, v),
        other => return not_available(other),
    };
    // Only links that allow it get the video as a download
    let attach = share.download
        && req
            .uri()
            .query()
            .is_some_and(|q| q.split('&').any(|p| p == "download=1"));
    let mut res = serve_file(video, req).await;
    if attach {
        let value = format!("attachment; filename=\"{}.mp4\"", download_name(&share));
        if let Ok(v) = HeaderValue::from_str(&value) {
            res.headers_mut().insert(header::CONTENT_DISPOSITION, v);
        }
    }
    res
}

/// `GET /s/{token}/photos.zip`: the edited photos, if the link allows it.
async fn share_photos(
    State(state): State<AppState>,
    Path(token): Path<String>,
    req: Request,
) -> Response {
    let (share, video) = match lookup(&state, &token).await {
        Lookup::Found(s, v) => (s, v),
        other => return not_available(other),
    };
    if !share.photos {
        return message_page(
            StatusCode::NOT_FOUND,
            "Fotos nicht freigegeben",
            &["Die Fotos dieses Zeitraffers sind nicht zum Herunterladen freigegeben."],
        );
    }
    let Some(zip) = ensure_photos_zip(&video).await else {
        return message_page(
            StatusCode::NOT_FOUND,
            "Keine Fotos",
            &["Zu diesem Zeitraffer liegen keine bearbeiteten Fotos vor."],
        );
    };
    let mut res = serve_file(zip, req).await;
    let value = format!("attachment; filename=\"{}-Fotos.zip\"", download_name(&share));
    if let Ok(v) = HeaderValue::from_str(&value) {
        res.headers_mut().insert(header::CONTENT_DISPOSITION, v);
    }
    res
}

/// ZIP of the folder's `images/`, made again when a photo was added or
/// taken out since.
async fn ensure_photos_zip(video: &std::path::Path) -> Option<PathBuf> {
    let dir = video.parent()?.to_path_buf();
    let _guard = ZIP_LOCK.lock().await;
    let made = tokio::task::spawn_blocking(move || build_photos_zip(&dir)).await.ok()?;
    made.map_err(|e| tracing::warn!("Photos ZIP: {e}")).ok()
}

fn build_photos_zip(dir: &std::path::Path) -> std::io::Result<PathBuf> {
    let images = dir.join("images");
    let zip = dir.join(PHOTOS_ZIP);
    let modified = |p: &std::path::Path| std::fs::metadata(p).and_then(|m| m.modified());
    let mut files: Vec<PathBuf> = std::fs::read_dir(&images)?
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            p.is_file()
                && p.extension()
                    .and_then(|e| e.to_str())
                    .is_some_and(|e| matches!(e.to_ascii_lowercase().as_str(), "jpg" | "jpeg" | "png"))
        })
        .collect();
    if files.is_empty() {
        return Err(std::io::ErrorKind::NotFound.into());
    }
    files.sort();
    // The folder changes when photos are moved out or back in
    let newest = files
        .iter()
        .filter_map(|f| modified(f).ok())
        .chain(modified(&images).ok())
        .max();
    if let (Ok(z), Some(n)) = (modified(&zip), newest) {
        if z >= n {
            return Ok(zip);
        }
    }
    let entries: Vec<(String, PathBuf)> = files
        .into_iter()
        .filter_map(|f| Some((f.file_name()?.to_str()?.to_string(), f)))
        .collect();
    let tmp = dir.join(format!("share-photos-{}.tmp", new_token()));
    if let Err(e) = crate::web::zip_store::write_stored(&tmp, &entries) {
        let _ = std::fs::remove_file(&tmp);
        return Err(e);
    }
    std::fs::rename(&tmp, &zip)?;
    tracing::info!("Photos ZIP made: {} ({} photos)", zip.display(), entries.len());
    Ok(zip)
}

/// File name of a download (without extension): the title (or folder) in plain characters.
fn download_name(share: &Share) -> String {
    let src = if share.title.trim().is_empty() {
        &share.folder
    } else {
        &share.title
    };
    let mut name: String = src
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    while name.contains("__") {
        name = name.replace("__", "_");
    }
    let name = name.trim_matches('_');
    if name.is_empty() { "Zeitraffer" } else { name }.to_string()
}

async fn share_poster(
    State(state): State<AppState>,
    Path(token): Path<String>,
    req: Request,
) -> Response {
    let video = match lookup(&state, &token).await {
        Lookup::Found(_, v) => v,
        other => return not_available(other),
    };
    match ensure_poster(&video).await {
        Some(poster) => serve_file(poster, req).await,
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

/// Serve a file with range support (type from its extension: .mp4 / .jpg).
async fn serve_file(path: PathBuf, req: Request) -> Response {
    match ServeFile::new(path).oneshot(req).await {
        Ok(mut res) => {
            res.headers_mut().insert(
                header::CACHE_CONTROL,
                HeaderValue::from_static("private, max-age=300"),
            );
            res.map(Body::new)
        }
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}

/// Poster image for the link preview, taken from the video. Made again
/// when the video is newer (recompiled).
async fn ensure_poster(video: &std::path::Path) -> Option<PathBuf> {
    let poster = video.with_file_name(POSTER_NAME);
    let modified = |p: PathBuf| async move { tokio::fs::metadata(p).await.ok()?.modified().ok() };
    if let (Some(p), Some(v)) = (
        modified(poster.clone()).await,
        modified(video.to_path_buf()).await,
    ) {
        if p >= v {
            return Some(poster);
        }
    }
    let tmp = video.with_file_name(format!("share-poster-{}.jpg", new_token()));
    for seek in ["1", "0"] {
        let ok = tokio::process::Command::new("ffmpeg")
            .args([
                "-hide_banner",
                "-loglevel",
                "error",
                "-y",
                "-ss",
                seek,
                "-i",
            ])
            .arg(video)
            .args([
                "-frames:v",
                "1",
                "-vf",
                "scale='min(1200,iw)':-2",
                "-q:v",
                "3",
            ])
            .arg(&tmp)
            .status()
            .await
            .is_ok_and(|s| s.success());
        if ok && tokio::fs::metadata(&tmp).await.is_ok_and(|m| m.len() > 0) {
            if tokio::fs::rename(&tmp, &poster).await.is_ok() {
                return Some(poster);
            }
            break;
        }
    }
    let _ = tokio::fs::remove_file(&tmp).await;
    None
}

/// Info page of the public port, same look as the Mosaik-Share pages.
fn message_page(status: StatusCode, title: &str, paragraphs: &[&str]) -> Response {
    let text: String = paragraphs
        .iter()
        .map(|p| format!("<p>{}</p>", esc(p)))
        .collect();
    let html = format!(
        r#"<!DOCTYPE html>
<html lang="de">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1, viewport-fit=cover">
<meta name="robots" content="noindex, nofollow">
<meta name="referrer" content="no-referrer">
<title>Timelapse-Share</title>
<style>
  :root {{ --bg: #111; --card: #1c1c1c; --text: #f2f2f2; --muted: #aaa; --accent: #6aa9ff; }}
  * {{ box-sizing: border-box; }}
  html, body {{ margin: 0; min-height: 100%; background: var(--bg); color: var(--text);
    font: 16px/1.5 system-ui, -apple-system, "Segoe UI", sans-serif; }}
  body {{ display: flex; align-items: center; justify-content: center; min-height: 100vh;
    padding: max(24px, env(safe-area-inset-top)) 16px max(24px, env(safe-area-inset-bottom)); }}
  main {{ background: var(--card); border-radius: 14px; padding: 32px 28px; max-width: 460px; width: 100%; text-align: center;
    box-shadow: 0 10px 40px rgba(0, 0, 0, .5); }}
  .frames {{ display: flex; gap: 4px; justify-content: center; align-items: center; margin-bottom: 18px; }}
  .frames span {{ width: 16px; height: 22px; border-radius: 2px; background: var(--accent); }}
  .frames span:nth-child(1) {{ opacity: .2; }} .frames span:nth-child(2) {{ opacity: .4; }}
  .frames span:nth-child(3) {{ opacity: .6; }} .frames span:nth-child(4) {{ opacity: .8; }}
  .frames i {{ width: 0; height: 0; margin-left: 4px; border-style: solid; border-width: 9px 0 9px 14px;
    border-color: transparent transparent transparent var(--accent); }}
  h1 {{ font-size: 22px; margin: 0 0 6px; font-weight: 600; }}
  .kicker {{ color: var(--muted); font-size: 13px; letter-spacing: .06em; text-transform: uppercase; margin: 0 0 14px; }}
  p {{ margin: 0 0 12px; color: #d6d6d6; }}
  p.muted {{ color: var(--muted); font-size: 14px; margin: 18px 0 0; }}
</style>
</head>
<body>
<main>
  <div class="frames" aria-hidden="true"><span></span><span></span><span></span><span></span><span></span><i></i></div>
  <p class="kicker">Timelapse-Share</p>
  <h1>{t}</h1>
  {text}
  <p class="muted">Hier werden Zeitraffer-Videos geteilt. Ein Video öffnet sich nur mit dem persönlichen Link, den du bekommen hast.</p>
</main>
</body>
</html>"#,
        t = esc(title),
    );
    (status, Html(html)).into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn urls_are_normalized() {
        assert_eq!(normalize_url(" ").unwrap(), "");
        assert_eq!(
            normalize_url("yourtimelapse.example.ch/").unwrap(),
            "https://yourtimelapse.example.ch"
        );
        assert_eq!(
            normalize_url("http://host:5001").unwrap(),
            "http://host:5001"
        );
        assert!(normalize_url("ftp://x").is_err());
        assert!(normalize_url("https://a\"b").is_err());
    }

    #[test]
    fn tokens_are_checked() {
        assert!(valid_token(&new_token()));
        assert!(!valid_token("../etc"));
        assert!(!valid_token("abc"));
    }

    #[test]
    fn expiry_rules() {
        let now = Utc::now();
        assert_eq!(expiry(None, now).unwrap(), None);
        assert_eq!(expiry(Some(7), now).unwrap(), Some(now + Duration::days(7)));
        assert!(expiry(Some(0), now).is_err());
        let s = Share {
            token: new_token(),
            folder: "a".into(),
            title: String::new(),
            created_at: now,
            expires_at: Some(now - Duration::seconds(1)),
            download: false,
            photos: false,
        };
        assert_eq!(download_name(&Share { title: "Anna – 2024!".into(), ..s.clone() }), "Anna_2024");
        assert!(s.is_expired(now));
    }

    #[test]
    fn origin_uses_proxy_headers() {
        let mut h = HeaderMap::new();
        h.insert("host", HeaderValue::from_static("yourtimelapse.example.ch"));
        assert_eq!(origin(&h), "https://yourtimelapse.example.ch");
        h.insert("x-forwarded-proto", HeaderValue::from_static("http"));
        assert_eq!(origin(&h), "http://yourtimelapse.example.ch");
        let mut h = HeaderMap::new();
        h.insert("host", HeaderValue::from_static("evil\"<x>.ch"));
        assert_eq!(origin(&h), "https://evilx.ch");
    }

    #[test]
    fn html_is_escaped() {
        assert_eq!(
            esc("<a href=\"x\">&'"),
            "&lt;a href=&quot;x&quot;&gt;&amp;&#39;"
        );
    }
}
