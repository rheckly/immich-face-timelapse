//! Output folder and image management endpoints.

use crate::utils::split_folder_version;
use crate::web::state::{validate_path_component, AppState, JobStatus, Progress};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
};
use serde::{Deserialize, Serialize};

use super::StartResponse;

/// Output folder information.
#[derive(Serialize)]
pub struct OutputFolderInfo {
    pub name: String,
    /// Person part of the folder name (`anna` for `anna_2026-11-12_1430`).
    pub person: String,
    /// Version stamp (`2026-11-12_1430`), `None` for folders from before versioning.
    pub version: Option<String>,
    pub image_count: u32,
    pub size_bytes: u64,
    pub has_video: bool,
    /// From run.json (all `None` for folders from before it existed):
    /// the person's name as shown in Immich (`Jürg Baumann`, not `jurg_baumann`),
    pub display_name: Option<String>,
    /// when the run started and finished (RFC 3339; no `finished` while it
    /// runs or when it was cancelled or failed)
    pub created: Option<String>,
    pub finished: Option<String>,
    /// and the length of its video in seconds.
    pub video_secs: Option<f64>,
    /// Whether the folder has a run.json.
    pub has_run: bool,
}

/// Image information for gallery view.
#[derive(Serialize)]
pub struct ImageInfo {
    pub filename: String,
    pub size_bytes: u64,
}

/// Response for listing images in a folder.
#[derive(Serialize)]
pub struct FolderImagesResponse {
    pub folder_name: String,
    pub images: Vec<ImageInfo>,
    pub total_count: u32,
    pub total_size_bytes: u64,
    pub video_exists: bool,
}

/// Request for bulk deleting images.
#[derive(Deserialize)]
pub struct BulkDeleteRequest {
    pub filenames: Vec<String>,
}

/// Response for bulk delete operations.
#[derive(Serialize)]
pub struct BulkDeleteResponse {
    pub deleted_count: u32,
    pub failed_count: u32,
    pub remaining_images: u32,
}

/// Check if a path exists asynchronously.
async fn path_exists(path: &std::path::Path) -> bool {
    tokio::fs::metadata(path).await.is_ok()
}

/// Check if a path is a directory asynchronously.
async fn path_is_dir(path: &std::path::Path) -> bool {
    tokio::fs::metadata(path)
        .await
        .map(|m| m.is_dir())
        .unwrap_or(false)
}

/// List all output folders with their stats.
pub async fn list_output_folders(
    State(state): State<AppState>,
) -> Result<Json<Vec<OutputFolderInfo>>, (StatusCode, String)> {
    let config = state.config.read().await;
    let output_dir = &config.output_dir;

    let mut folders = Vec::new();

    if path_exists(output_dir).await {
        let mut entries = tokio::fs::read_dir(output_dir).await.map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to read output directory: {}", e),
            )
        })?;

        while let Some(entry) = entries.next_entry().await.map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to read directory entry: {}", e),
            )
        })? {
            let path = entry.path();
            if path_is_dir(&path).await {
                let name = entry.file_name().to_string_lossy().to_string();

                // Count images in the images subfolder
                let images_dir = path.join("images");
                let mut image_count = 0u32;
                let mut size_bytes = 0u64;

                if path_exists(&images_dir).await {
                    if let Ok(mut img_entries) = tokio::fs::read_dir(&images_dir).await {
                        while let Ok(Some(img_entry)) = img_entries.next_entry().await {
                            let img_path = img_entry.path();
                            if img_path.extension().is_some_and(|ext| ext == "jpg") {
                                image_count += 1;
                                if let Ok(metadata) = tokio::fs::metadata(&img_path).await {
                                    size_bytes += metadata.len();
                                }
                            }
                        }
                    }
                }

                // Check for video file (named after the folder/person)
                let video_filename = format!("{}.mp4", name);
                let has_video = path_exists(&path.join(&video_filename)).await;

                let (person, version) = split_folder_version(&name);
                let person = person.to_string();
                let version = version.map(str::to_string);

                let run = crate::run_info::read(&path).await;
                let text = |v: Option<&serde_json::Value>| {
                    v.and_then(|v| v.as_str())
                        .filter(|s| !s.is_empty())
                        .map(str::to_string)
                };
                let display_name = run.as_ref().and_then(|r| text(r.get("person_name")));
                let created = run.as_ref().and_then(|r| text(r.get("created")));
                let result = run.as_ref().and_then(|r| r.get("result"));
                let finished = text(result.and_then(|r| r.get("finished")));
                let video_secs = result
                    .and_then(|r| r.get("video_secs"))
                    .and_then(|v| v.as_f64());

                folders.push(OutputFolderInfo {
                    name,
                    person,
                    version,
                    image_count,
                    size_bytes,
                    has_video,
                    display_name,
                    created,
                    finished,
                    video_secs,
                    has_run: run.is_some(),
                });
            }
        }
    }

    // Sort by person, then newest version first; folders without a version
    // stamp (from before versioning) come last within their person.
    folders.sort_by(|a, b| {
        a.person
            .cmp(&b.person)
            .then_with(|| b.version.cmp(&a.version))
            .then_with(|| a.name.cmp(&b.name))
    });

    Ok(Json(folders))
}

/// Clean up all output folders.
pub async fn cleanup_all_output(
    State(state): State<AppState>,
) -> Result<Json<StartResponse>, (StatusCode, String)> {
    state
        .ensure_no_job_running()
        .await
        .map_err(|e| (StatusCode::CONFLICT, e.to_string()))?;

    let config = state.config.read().await;
    let output_dir = &config.output_dir;

    // Remove all contents of output directory
    if path_exists(output_dir).await {
        let mut entries = tokio::fs::read_dir(output_dir).await.map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to read output directory: {}", e),
            )
        })?;

        let mut deleted_count = 0;
        while let Some(entry) = entries.next_entry().await.map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to read directory entry: {}", e),
            )
        })? {
            let path = entry.path();
            if path_is_dir(&path).await {
                tokio::fs::remove_dir_all(&path).await.map_err(|e| {
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        format!("Failed to remove directory {}: {}", path.display(), e),
                    )
                })?;
                deleted_count += 1;
            }
        }

        tracing::info!("Cleaned up {} output folders", deleted_count);

        Ok(Json(StartResponse {
            success: true,
            message: format!("Deleted {} output folders", deleted_count),
        }))
    } else {
        Ok(Json(StartResponse {
            success: true,
            message: "Output directory does not exist".to_string(),
        }))
    }
}

/// Clean up a specific output folder by name.
pub async fn cleanup_output_folder(
    State(state): State<AppState>,
    Path(folder_name): Path<String>,
) -> Result<Json<StartResponse>, (StatusCode, String)> {
    state
        .ensure_no_job_running()
        .await
        .map_err(|e| (StatusCode::CONFLICT, e.to_string()))?;

    validate_path_component(&folder_name, "folder name")
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;

    let config = state.config.read().await;

    let folder_path = config.output_dir.join(&folder_name);

    let folder_meta = tokio::fs::metadata(&folder_path).await.map_err(|_| {
        (
            StatusCode::NOT_FOUND,
            format!("Folder '{}' not found", folder_name),
        )
    })?;

    if !folder_meta.is_dir() {
        return Err((
            StatusCode::BAD_REQUEST,
            format!("'{}' is not a directory", folder_name),
        ));
    }

    tokio::fs::remove_dir_all(&folder_path).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to remove folder: {}", e),
        )
    })?;

    tracing::info!("Cleaned up output folder: {}", folder_name);

    Ok(Json(StartResponse {
        success: true,
        message: format!("Deleted folder '{}'", folder_name),
    }))
}

/// List images in a specific output folder.
pub async fn list_folder_images(
    State(state): State<AppState>,
    Path(folder_name): Path<String>,
) -> Result<Json<FolderImagesResponse>, (StatusCode, String)> {
    validate_path_component(&folder_name, "folder name")
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;

    let config = state.config.read().await;

    let images_dir = config.output_dir.join(&folder_name).join("images");

    if !path_exists(&images_dir).await {
        return Err((
            StatusCode::NOT_FOUND,
            format!("Folder '{}' not found or has no images", folder_name),
        ));
    }

    let mut images = Vec::new();
    let mut total_size_bytes = 0u64;

    let mut entries = tokio::fs::read_dir(&images_dir).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to read images directory: {}", e),
        )
    })?;

    while let Some(entry) = entries.next_entry().await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to read directory entry: {}", e),
        )
    })? {
        let path = entry.path();
        if path.extension().is_some_and(|ext| ext == "jpg") {
            let filename = entry.file_name().to_string_lossy().to_string();
            let metadata = tokio::fs::metadata(&path).await.map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Failed to read file metadata: {}", e),
                )
            })?;
            let size_bytes = metadata.len();
            total_size_bytes += size_bytes;

            images.push(ImageInfo {
                filename,
                size_bytes,
            });
        }
    }

    // Sort by filename (chronological since filenames are timestamp-based)
    images.sort_by(|a, b| a.filename.cmp(&b.filename));

    let total_count = images.len() as u32;
    let video_filename = format!("{}.mp4", folder_name);
    let video_exists =
        path_exists(&config.output_dir.join(&folder_name).join(&video_filename)).await;

    Ok(Json(FolderImagesResponse {
        folder_name,
        images,
        total_count,
        total_size_bytes,
        video_exists,
    }))
}

/// Delete a single image from an output folder.
pub async fn delete_single_image(
    State(state): State<AppState>,
    Path((folder_name, filename)): Path<(String, String)>,
) -> Result<Json<StartResponse>, (StatusCode, String)> {
    state
        .ensure_no_job_running()
        .await
        .map_err(|e| (StatusCode::CONFLICT, e.to_string()))?;

    validate_path_component(&folder_name, "folder name")
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
    validate_path_component(&filename, "filename")
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;

    let config = state.config.read().await;
    if !filename.ends_with(".jpg") {
        return Err((
            StatusCode::BAD_REQUEST,
            "Only .jpg files can be deleted".to_string(),
        ));
    }

    let file_path = config
        .output_dir
        .join(&folder_name)
        .join("images")
        .join(&filename);

    if !path_exists(&file_path).await {
        return Err((
            StatusCode::NOT_FOUND,
            format!("Image '{}' not found in folder '{}'", filename, folder_name),
        ));
    }

    tokio::fs::remove_file(&file_path).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to delete image: {}", e),
        )
    })?;

    tracing::info!("Deleted image: {}/{}", folder_name, filename);

    Ok(Json(StartResponse {
        success: true,
        message: format!("Deleted image '{}'", filename),
    }))
}

/// Bulk delete images from an output folder.
pub async fn delete_images_bulk(
    State(state): State<AppState>,
    Path(folder_name): Path<String>,
    Json(request): Json<BulkDeleteRequest>,
) -> Result<Json<BulkDeleteResponse>, (StatusCode, String)> {
    state
        .ensure_no_job_running()
        .await
        .map_err(|e| (StatusCode::CONFLICT, e.to_string()))?;

    validate_path_component(&folder_name, "folder name")
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;

    let config = state.config.read().await;

    let images_dir = config.output_dir.join(&folder_name).join("images");

    if !path_exists(&images_dir).await {
        return Err((
            StatusCode::NOT_FOUND,
            format!("Folder '{}' not found or has no images", folder_name),
        ));
    }

    let mut deleted_count = 0u32;
    let mut failed_count = 0u32;

    for filename in &request.filenames {
        // Validate each filename
        if validate_path_component(filename, "filename").is_err() {
            failed_count += 1;
            continue;
        }
        if !filename.ends_with(".jpg") {
            failed_count += 1;
            continue;
        }

        let file_path = images_dir.join(filename);
        match tokio::fs::remove_file(&file_path).await {
            Ok(_) => deleted_count += 1,
            Err(_) => failed_count += 1,
        }
    }

    // Count remaining images
    let mut remaining_images = 0u32;
    if let Ok(mut entries) = tokio::fs::read_dir(&images_dir).await {
        while let Ok(Some(entry)) = entries.next_entry().await {
            if entry.path().extension().is_some_and(|ext| ext == "jpg") {
                remaining_images += 1;
            }
        }
    }

    tracing::info!(
        "Bulk deleted {} images from folder {} ({} failed)",
        deleted_count,
        folder_name,
        failed_count
    );

    Ok(Json(BulkDeleteResponse {
        deleted_count,
        failed_count,
        remaining_images,
    }))
}

/// Sub folder for photos the user took out before the video was made.
const EXCLUDED_DIR: &str = "excluded";

/// The `.jpg` file names in a folder, sorted (empty if it does not exist).
async fn jpg_names(dir: &std::path::Path) -> Vec<String> {
    let mut names = Vec::new();
    if let Ok(mut entries) = tokio::fs::read_dir(dir).await {
        while let Ok(Some(entry)) = entries.next_entry().await {
            if entry.path().extension().is_some_and(|ext| ext == "jpg") {
                names.push(entry.file_name().to_string_lossy().to_string());
            }
        }
    }
    names.sort();
    names
}

/// `GET /api/output/{folder}/run`: the run's settings (`run.json`); 404 for
/// folders from before it existed.
pub async fn get_run_info(
    State(state): State<AppState>,
    Path(folder_name): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    validate_path_component(&folder_name, "folder name")
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
    let folder = state.config.read().await.output_dir.join(&folder_name);
    crate::run_info::read(&folder)
        .await
        .map(Json)
        .ok_or((StatusCode::NOT_FOUND, "No run information".to_string()))
}

/// Photos of a folder: the ones used for the video and the ones taken out.
#[derive(Serialize, Deserialize)]
pub struct Selection {
    #[serde(default)]
    pub images: Vec<String>,
    pub excluded: Vec<String>,
}

/// `GET /api/output/{folder}/selection`
pub async fn get_selection(
    State(state): State<AppState>,
    Path(folder_name): Path<String>,
) -> Result<Json<Selection>, (StatusCode, String)> {
    validate_path_component(&folder_name, "folder name")
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
    let folder = state.config.read().await.output_dir.join(&folder_name);
    if !path_exists(&folder.join("images")).await {
        return Err((StatusCode::NOT_FOUND, format!("Folder '{}' has no images", folder_name)));
    }
    Ok(Json(Selection {
        images: jpg_names(&folder.join("images")).await,
        excluded: jpg_names(&folder.join(EXCLUDED_DIR)).await,
    }))
}

/// `PUT /api/output/{folder}/selection`: the named photos move to `excluded/`,
/// all others back to `images/`. Nothing is deleted.
pub async fn put_selection(
    State(state): State<AppState>,
    Path(folder_name): Path<String>,
    Json(request): Json<Selection>,
) -> Result<Json<Selection>, (StatusCode, String)> {
    validate_path_component(&folder_name, "folder name")
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
    for name in &request.excluded {
        validate_path_component(name, "filename")
            .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
    }
    {
        let progress = state.progress.read().await;
        let busy = matches!(progress.status, JobStatus::Running | JobStatus::CompilingVideo)
            && !progress.awaiting_review
            && progress.output_folder.as_deref() == Some(folder_name.as_str());
        if busy {
            return Err((StatusCode::CONFLICT, "This folder is being processed".to_string()));
        }
    }
    let folder = state.config.read().await.output_dir.join(&folder_name);
    let images = folder.join("images");
    let excluded = folder.join(EXCLUDED_DIR);
    if !path_exists(&images).await {
        return Err((StatusCode::NOT_FOUND, format!("Folder '{}' has no images", folder_name)));
    }
    let wanted: std::collections::HashSet<&str> =
        request.excluded.iter().map(String::as_str).collect();
    let io = |e: std::io::Error| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string());
    for name in jpg_names(&images).await {
        if wanted.contains(name.as_str()) {
            tokio::fs::create_dir_all(&excluded).await.map_err(io)?;
            tokio::fs::rename(images.join(&name), excluded.join(&name))
                .await
                .map_err(io)?;
        }
    }
    for name in jpg_names(&excluded).await {
        if !wanted.contains(name.as_str()) {
            tokio::fs::rename(excluded.join(&name), images.join(&name))
                .await
                .map_err(io)?;
        }
    }
    Ok(Json(Selection {
        images: jpg_names(&images).await,
        excluded: jpg_names(&excluded).await,
    }))
}

/// Options for compiling a folder's video.
#[derive(Deserialize, Default)]
pub struct CompileQuery {
    /// Finish a run that waited for the photo selection: report it like the
    /// end of that run and drop the frames if they should not be kept.
    #[serde(default)]
    pub finish: bool,
}

/// Compile video for a specific output folder.
pub async fn compile_folder_video(
    State(state): State<AppState>,
    Path(folder_name): Path<String>,
    Query(query): Query<CompileQuery>,
) -> Result<Json<StartResponse>, (StatusCode, String)> {
    validate_path_component(&folder_name, "folder name")
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;

    let config = state.config.read().await;

    let folder_path = config.output_dir.join(&folder_name);
    let images_dir = folder_path.join("images");

    if !path_exists(&images_dir).await {
        return Err((
            StatusCode::NOT_FOUND,
            format!("Folder '{}' not found or has no images", folder_name),
        ));
    }

    // Count images to verify there are some
    let mut image_count = 0u32;
    if let Ok(mut entries) = tokio::fs::read_dir(&images_dir).await {
        while let Ok(Some(entry)) = entries.next_entry().await {
            if entry.path().extension().is_some_and(|ext| ext == "jpg") {
                image_count += 1;
            }
        }
    }

    if image_count == 0 {
        return Err((
            StatusCode::BAD_REQUEST,
            "No images found in folder to compile".to_string(),
        ));
    }

    // Atomically check-and-set: hold write lock across both check and status update
    // to prevent TOCTOU race conditions between concurrent requests.
    {
        let mut progress = state.progress.write().await;
        if progress.status.is_active() && !progress.awaiting_review
        {
            return Err((StatusCode::CONFLICT, "A job is already running".to_string()));
        }
        // Finishing the run that waited for the selection keeps its person and counts
        let carry = query.finish && progress.output_folder.as_deref() == Some(folder_name.as_str());
        let base = if carry {
            progress.clone()
        } else {
            Progress {
                person_name: Some(folder_name.clone()),
                ..Progress::default()
            }
        };
        let new_progress = Progress {
            status: JobStatus::CompilingVideo,
            completed: 0,
            total: image_count,
            message: Some(format!("Compiling video for {}...", folder_name)),
            output_folder: Some(folder_name.clone()),
            awaiting_review: false,
            video_started_at: Some(unix_millis()),
            video_millis: None,
            video_secs: None,
            ..base
        };
        *progress = new_progress.clone();
        let _ = state.progress_tx.send(new_progress);
    }

    // Create cancellation token
    let (job_id, cancel_token) = state.create_cancel_token().await;

    // Clone values needed for the async task
    let video_config = config.video.clone();
    let music_dir = config.music_dir.clone();
    let api_config = config.api.clone();
    let video_filename = format!("{}.mp4", folder_name);
    let output_path = folder_path.join(&video_filename);
    let job_state = state.clone();
    let base = state.progress.read().await.clone();
    let keep_frames = config.processing.output.keep_frames;
    let finish = query.finish;
    let started = std::time::Instant::now();

    // Spawn the compilation job in the background
    tokio::spawn(async move {
        let result = crate::video::compile_timelapse(
            &images_dir,
            &output_path,
            &video_config,
            &music_dir,
            &api_config,
            Some(&cancel_token),
            |current, total| {
                // Check for cancellation
                if cancel_token.is_cancelled() {
                    return;
                }

                // Update progress (fire and forget since we're in sync callback)
                let state_clone = job_state.clone();
                let update = Progress {
                    status: JobStatus::CompilingVideo,
                    completed: current,
                    total,
                    ..base.clone()
                };
                tokio::spawn(async move {
                    state_clone.update_progress(update).await;
                });
            },
        )
        .await;

        // Update final status
        match result {
            Ok(_) => {
                let video_secs = crate::video::probe_duration(&output_path).await.ok();
                let excluded = jpg_names(&folder_path.join(EXCLUDED_DIR)).await.len();
                crate::run_info::update(&folder_path, |run| {
                    run["result"]["kept"] = serde_json::json!(image_count);
                    run["result"]["excluded"] = serde_json::json!(excluded);
                    run["result"]["video_secs"] = serde_json::json!(video_secs);
                    run["result"]["finished"] = serde_json::json!(chrono::Local::now().to_rfc3339());
                })
                .await;
                if finish && !keep_frames {
                    crate::job::remove_frames(&images_dir).await;
                    let _ = tokio::fs::remove_dir_all(folder_path.join(EXCLUDED_DIR)).await;
                }
                job_state
                    .update_progress(Progress {
                        status: JobStatus::Completed,
                        completed: image_count,
                        total: image_count,
                        message: Some("Video compilation complete".to_string()),
                        video_millis: Some(started.elapsed().as_millis() as u64),
                        video_secs,
                        ..base.clone()
                    })
                    .await;
            }
            Err(e) => {
                if cancel_token.is_cancelled() {
                    job_state
                        .update_progress(Progress {
                            status: JobStatus::Cancelled,
                            total: image_count,
                            message: Some("Video compilation cancelled".to_string()),
                            ..base.clone()
                        })
                        .await;
                } else {
                    tracing::error!("Video compilation failed: {}", e);
                    job_state
                        .update_progress(Progress {
                            status: JobStatus::Error(e.to_string()),
                            total: image_count,
                            message: Some(format!("Video compilation failed: {}", e)),
                            ..base.clone()
                        })
                        .await;
                }
            }
        }

        job_state.clear_cancel_token(job_id).await;
    });

    Ok(Json(StartResponse {
        success: true,
        message: format!("Video compilation started for '{}'", folder_name),
    }))
}

/// Current time as Unix milliseconds (0 if the clock is before 1970).
fn unix_millis() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_millis() as u64)
}
