//! Music endpoints.

use crate::video::{list_music, probe_duration, MusicTrack, AUDIO_EXTENSIONS};
use crate::web::state::AppState;
use axum::{
    body::Bytes,
    extract::{Path, State},
    http::StatusCode,
    response::Json,
};
use serde::Serialize;

/// Largest music file accepted by the upload, in bytes.
pub const MAX_UPLOAD_BYTES: usize = 200 * 1024 * 1024;

/// Music tracks available in the music directory.
#[derive(Serialize)]
pub struct MusicListResponse {
    /// Configured music directory (e.g. `music`, mounted at `/app/music` in Docker).
    pub music_dir: String,
    /// Whether the music directory exists.
    pub available: bool,
    /// Audio files found in the directory, sorted by name.
    pub tracks: Vec<MusicTrack>,
}

/// List the music tracks that can be mixed into the video.
pub async fn list_music_tracks(
    State(state): State<AppState>,
) -> Result<Json<MusicListResponse>, (StatusCode, String)> {
    let music_dir = state.config.read().await.music_dir.clone();
    let available = tokio::fs::metadata(&music_dir)
        .await
        .map(|m| m.is_dir())
        .unwrap_or(false);
    let tracks = list_music(&music_dir).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to list music: {}", e),
        )
    })?;
    Ok(Json(MusicListResponse {
        music_dir: music_dir.display().to_string(),
        available,
        tracks,
    }))
}

/// Check an uploaded file name: a plain audio file name, no folders.
fn valid_upload_name(name: &str) -> Result<(), String> {
    let ext = std::path::Path::new(name)
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase());
    if name.is_empty()
        || name.len() > 200
        || name.starts_with('.')
        || name.contains("..")
        || name.contains('/')
        || name.contains('\\')
        || name.chars().any(|c| c.is_control())
    {
        return Err(format!("Invalid file name '{}'", name));
    }
    match ext {
        Some(e) if AUDIO_EXTENSIONS.contains(&e.as_str()) => Ok(()),
        _ => Err(format!(
            "Only audio files can be uploaded ({})",
            AUDIO_EXTENSIONS.join(", ")
        )),
    }
}

/// Upload a music file into the music directory.
///
/// `POST /api/music/{filename}` with the file as the request body. An existing
/// file with the same name is not overwritten.
pub async fn upload_music_track(
    State(state): State<AppState>,
    Path(filename): Path<String>,
    body: Bytes,
) -> Result<Json<MusicTrack>, (StatusCode, String)> {
    valid_upload_name(&filename).map_err(|e| (StatusCode::BAD_REQUEST, e))?;
    if body.is_empty() {
        return Err((StatusCode::BAD_REQUEST, "The file is empty".to_string()));
    }

    let music_dir = state.config.read().await.music_dir.clone();
    let is_dir = tokio::fs::metadata(&music_dir)
        .await
        .map(|m| m.is_dir())
        .unwrap_or(false);
    if !is_dir {
        return Err((
            StatusCode::CONFLICT,
            format!("Music folder {} not found", music_dir.display()),
        ));
    }

    let target = music_dir.join(&filename);
    if tokio::fs::metadata(&target).await.is_ok() {
        return Err((
            StatusCode::CONFLICT,
            format!("'{}' already exists", filename),
        ));
    }

    // Write to a hidden temp file first (hidden = not listed), then rename
    let temp = music_dir.join(format!(".upload-{}.part", filename));
    let write_err = |e: std::io::Error| {
        let read_only = e.kind() == std::io::ErrorKind::PermissionDenied
            || e.raw_os_error() == Some(30); // EROFS
        let message = if read_only {
            format!(
                "Music folder is read-only ({}); mount it writable (without :ro)",
                music_dir.display()
            )
        } else {
            format!("Could not save '{}': {}", filename, e)
        };
        (StatusCode::INTERNAL_SERVER_ERROR, message)
    };
    if let Err(e) = tokio::fs::write(&temp, &body).await {
        let _ = tokio::fs::remove_file(&temp).await;
        return Err(write_err(e));
    }
    if let Err(e) = tokio::fs::rename(&temp, &target).await {
        let _ = tokio::fs::remove_file(&temp).await;
        return Err(write_err(e));
    }

    tracing::info!("Uploaded music track {} ({} bytes)", filename, body.len());
    let duration_secs = probe_duration(&target).await.ok();
    Ok(Json(MusicTrack {
        name: filename,
        duration_secs,
    }))
}

#[cfg(test)]
mod tests {
    use super::valid_upload_name;

    #[test]
    fn upload_names() {
        assert!(valid_upload_name("01 - Song.mp3").is_ok());
        assert!(valid_upload_name("track.FLAC").is_ok());
        assert!(valid_upload_name("../x.mp3").is_err());
        assert!(valid_upload_name("a/b.mp3").is_err());
        assert!(valid_upload_name("a\\b.mp3").is_err());
        assert!(valid_upload_name(".hidden.mp3").is_err());
        assert!(valid_upload_name("notes.txt").is_err());
        assert!(valid_upload_name("").is_err());
    }
}
