//! Music endpoints.

use crate::video::{list_music, MusicTrack};
use crate::web::state::AppState;
use axum::{extract::State, http::StatusCode, response::Json};
use serde::Serialize;

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
