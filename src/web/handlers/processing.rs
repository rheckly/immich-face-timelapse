//! Processing job control endpoints (progress, start, cancel).

use crate::job::{run_job, JobParams};
use crate::web::state::{AppState, JobStatus, Progress, SkipStats};
use axum::{extract::State, http::StatusCode, response::Json};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::sources::{merge_ids, resolve_recipient_albums};
use super::StartResponse;

/// Skip statistics for API response.
///
/// Uses a dynamic HashMap to support arbitrary skip reasons from pipeline steps.
#[derive(Serialize)]
pub struct SkipStatsResponse {
    /// Map of skip reason ID to count.
    #[serde(flatten)]
    pub counts: HashMap<String, u32>,
    /// Total number of skipped images.
    pub total: u32,
    /// Number of images kept (passed all filters).
    pub kept: u32,
}

impl From<&SkipStats> for SkipStatsResponse {
    fn from(stats: &SkipStats) -> Self {
        Self {
            counts: stats.counts().clone(),
            total: stats.total(),
            kept: stats.kept(),
        }
    }
}

/// Progress response.
#[derive(Serialize)]
pub struct ProgressResponse {
    pub status: String,
    pub completed: u32,
    pub total: u32,
    pub message: Option<String>,
    pub skip_stats: SkipStatsResponse,
    pub person_id: Option<String>,
    pub person_name: Option<String>,
    pub album_ids: Vec<String>,
    pub album_names: Vec<String>,
    pub output_folder: Option<String>,
    pub video_started_at: Option<u64>,
    pub video_millis: Option<u64>,
    pub video_secs: Option<f64>,
    pub awaiting_review: bool,
}

impl From<&Progress> for ProgressResponse {
    fn from(p: &Progress) -> Self {
        Self {
            status: p.status.as_str().to_string(),
            completed: p.completed,
            total: p.total,
            message: p.message.clone(),
            skip_stats: SkipStatsResponse::from(&p.skip_stats),
            person_id: p.person_id.clone(),
            person_name: p.person_name.clone(),
            album_ids: p.album_ids.clone(),
            album_names: p.album_names.clone(),
            output_folder: p.output_folder.clone(),
            video_started_at: p.video_started_at,
            video_millis: p.video_millis,
            video_secs: p.video_secs,
            awaiting_review: p.awaiting_review,
        }
    }
}

/// Get current progress.
pub async fn get_progress(State(state): State<AppState>) -> Json<ProgressResponse> {
    Json(ProgressResponse::from(&*state.progress.read().await))
}

/// Start processing request.
#[derive(Deserialize)]
pub struct StartRequest {
    pub person_id: String,
    pub person_name: Option<String>,
    pub date_from: Option<String>,
    pub date_to: Option<String>,
    pub album_ids: Vec<String>,
    pub album_names: Vec<String>,
    /// Other people: photos that also show one of them count.
    #[serde(default)]
    pub people_ids: Vec<String>,
    /// People from Smart Albums: photos in an album shared with one of them count.
    #[serde(default)]
    pub recipient_ids: Vec<String>,
    /// Stop after preparing the photos, so they can be picked before the video is made.
    #[serde(default)]
    pub review: bool,
    /// What the user chose in the browser (person, sources, period), stored with
    /// the run so its settings can be taken over later. Not read by the server.
    #[serde(default)]
    pub source: Option<serde_json::Value>,
}

/// Validate that a person_id looks reasonable (non-empty, reasonable length, safe characters).
pub(super) fn validate_person_id(person_id: &str) -> Result<(), (StatusCode, String)> {
    if person_id.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            "person_id must not be empty".to_string(),
        ));
    }
    if person_id.len() > 128 {
        return Err((
            StatusCode::BAD_REQUEST,
            "person_id is too long (max 128 characters)".to_string(),
        ));
    }
    if !person_id
        .chars()
        .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
    {
        return Err((
            StatusCode::BAD_REQUEST,
            "person_id contains invalid characters (only alphanumeric, hyphens, underscores allowed)"
                .to_string(),
        ));
    }
    Ok(())
}

/// Start processing for a person.
pub async fn start_processing(
    State(state): State<AppState>,
    Json(request): Json<StartRequest>,
) -> Result<Json<StartResponse>, (StatusCode, String)> {
    validate_person_id(&request.person_id)?;

    // Albums shared with the chosen people count like chosen albums
    let shared_album_ids = resolve_recipient_albums(&request.recipient_ids).await?;

    // Atomically check-and-set: hold write lock across both check and status update
    // to prevent TOCTOU race conditions between concurrent requests.
    {
        let mut progress = state.progress.write().await;
        // Also while the previous job is still cancelling: it clears its token
        // and writes its final status only when it has stopped.
        if progress.status.is_active() {
            return Err((StatusCode::CONFLICT, "A job is already running".to_string()));
        }
        let new_progress = Progress {
            status: JobStatus::Running,
            message: Some("Starting...".to_string()),
            person_id: Some(request.person_id.clone()),
            person_name: request.person_name.clone(),
            album_ids: request.album_ids.clone(),
            album_names: request.album_names.clone(),
            ..Progress::default()
        };
        *progress = new_progress.clone();
        let _ = state.progress_tx.send(new_progress);
    }

    // Create cancellation token
    let (job_id, cancel_token) = state.create_cancel_token().await;

    tracing::info!(
        "Starting processing for person {} (date range: {:?} - {:?})",
        request.person_id,
        request.date_from,
        request.date_to
    );

    // Spawn the processing job in the background
    let job_params = JobParams {
        person_id: request.person_id,
        person_name: request.person_name,
        date_from: request.date_from,
        date_to: request.date_to,
        album_ids: merge_ids(request.album_ids, shared_album_ids),
        album_names: request.album_names,
        people_ids: request.people_ids,
        review: request.review,
        source: request.source,
    };

    let job_state = state.clone();
    tokio::spawn(async move {
        run_job(job_state, job_params, job_id, cancel_token).await;
    });

    Ok(Json(StartResponse {
        success: true,
        message: "Processing started".to_string(),
    }))
}

/// Cancel the current processing job.
pub async fn cancel_processing(State(state): State<AppState>) -> Json<StartResponse> {
    let cancelled = state.request_cancel().await;

    if cancelled {
        // The job will update its own status when it detects cancellation
        Json(StartResponse {
            success: true,
            message: "Cancellation requested".to_string(),
        })
    } else {
        Json(StartResponse {
            success: false,
            message: "No job running to cancel".to_string(),
        })
    }
}
