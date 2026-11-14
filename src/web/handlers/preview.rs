//! Preview of the quality filters on a single photo.
//!
//! Runs the same pipeline steps as the job on one photo, with every filter measured,
//! and returns the measured values, each filter's verdict and the resulting frame.
//! The downloaded photo and its face data are cached for a few minutes, so moving a
//! slider only redoes the cheap part.

use super::processing::validate_person_id;
use super::sources::{merge_ids, resolve_recipient_albums, split_ids};
use crate::config::{Config, ProcessingConfig};
use crate::immich_api::{Asset, FaceData, ImmichClient};
use crate::pipeline::steps::{
    AlignmentStep, BlurStep, BrightnessStep, CropAndResizeStep, EyeFilterStep, FaceResolutionStep,
    HeadPoseStep, LandmarksStep, TimestampStep,
};
use crate::pipeline::{
    computed_keys, load_image_with_orientation, PipelineContext, ProcessingStep, StepOutcome,
};
use crate::web::state::AppState;
use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::Json,
};
use image::codecs::jpeg::JpegEncoder;
use image::DynamicImage;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

type ApiError = (StatusCode, String);

/// How long a downloaded photo stays cached.
const CACHE_TTL: Duration = Duration::from_secs(10 * 60);
/// How many photos are kept at most (decoded photos are large).
const CACHE_MAX: usize = 8;
/// Long edge of the original photo shown next to the frame.
const ORIGINAL_EDGE: u32 = 480;
/// Number of sample photos offered when nothing else is asked for.
const DEFAULT_SAMPLES: usize = 24;

/// Request for a preview of one photo.
#[derive(Deserialize)]
pub struct PreviewRequest {
    pub asset_id: String,
    /// The person whose face is used.
    pub person_id: String,
    /// Date of the photo for the date overlay, as returned by the samples.
    #[serde(default)]
    pub timestamp: Option<String>,
    /// Processing settings in the same shape as in `PUT /api/config`.
    #[serde(default)]
    pub processing: ProcessingConfig,
}

/// Values measured on the photo. Missing values could not be measured.
#[derive(Debug, Default, Serialize)]
pub struct Measured {
    /// Face size in pixels of the photo (the smaller side of the face box).
    pub face_size: Option<i32>,
    /// Share of the frame that has to be filled because the face is at the edge, in percent.
    pub padding_percent: Option<f32>,
    /// Brightness of the face, 0 to 1.
    pub brightness: Option<f32>,
    /// Sharpness of the face (Laplacian variance).
    pub sharpness: Option<f32>,
    /// Head turn left or right in degrees.
    pub yaw: Option<f32>,
    /// Head tilt up or down in degrees.
    pub pitch: Option<f32>,
    /// Eye opening (eye aspect ratio).
    pub ear: Option<f32>,
}

/// Verdict of one filter with the current thresholds.
#[derive(Debug, Serialize, PartialEq)]
pub struct FilterVerdict {
    /// Filter key as in the config (`face_resolution`, `crop`, ...).
    pub key: &'static str,
    pub enabled: bool,
    /// `None` when the value could not be measured.
    pub passed: Option<bool>,
    /// Why the photo fails this filter, e.g. "too_blurry".
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

/// Face box in the original photo, as fractions of its width and height.
#[derive(Debug, Clone, Copy, Serialize, PartialEq)]
pub struct FaceBox {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

/// Preview of one photo.
#[derive(Serialize)]
pub struct PreviewResponse {
    pub asset_id: String,
    /// Small version of the photo as a data URL.
    pub original: String,
    pub face_box: FaceBox,
    /// The resulting frame as a data URL, missing when none could be made.
    pub frame: Option<String>,
    pub values: Measured,
    pub filters: Vec<FilterVerdict>,
    /// Whether the photo would be used with the current settings.
    pub used: bool,
    /// Why no frame could be made (e.g. no landmarks found).
    pub problem: Option<String>,
    /// Whether the photo came from the cache.
    pub cached: bool,
    /// Time spent in the pipeline, in milliseconds.
    pub millis: u64,
}

/// A downloaded and decoded photo with the face of the person.
struct CachedPhoto {
    face: FaceData,
    image: DynamicImage,
    original: String,
    face_box: FaceBox,
    loaded_at: Instant,
}

fn cache() -> &'static Mutex<HashMap<String, Arc<CachedPhoto>>> {
    static CACHE: OnceLock<Mutex<HashMap<String, Arc<CachedPhoto>>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

fn cached_photo(key: &str) -> Option<Arc<CachedPhoto>> {
    let mut map = cache().lock().ok()?;
    map.retain(|_, photo| photo.loaded_at.elapsed() < CACHE_TTL);
    map.get(key).cloned()
}

fn store_photo(key: String, photo: Arc<CachedPhoto>) {
    if let Ok(mut map) = cache().lock() {
        while map.len() >= CACHE_MAX {
            let oldest = map
                .iter()
                .min_by_key(|(_, p)| p.loaded_at)
                .map(|(k, _)| k.clone());
            let Some(k) = oldest else { break };
            map.remove(&k);
        }
        map.insert(key, photo);
    }
}

/// Face box as fractions of the photo the face data refers to.
fn face_box(face: &FaceData) -> FaceBox {
    let w = face.image_width.max(1) as f32;
    let h = face.image_height.max(1) as f32;
    FaceBox {
        x: (face.bounding_box_x1 / w).clamp(0.0, 1.0),
        y: (face.bounding_box_y1 / h).clamp(0.0, 1.0),
        w: ((face.bounding_box_x2 - face.bounding_box_x1) / w).clamp(0.0, 1.0),
        h: ((face.bounding_box_y2 - face.bounding_box_y1) / h).clamp(0.0, 1.0),
    }
}

/// Encode an image as a JPEG data URL.
fn jpeg_data_url(image: &DynamicImage) -> Result<String, String> {
    let mut jpeg = Vec::new();
    JpegEncoder::new_with_quality(&mut jpeg, 88)
        .encode_image(&image.to_rgb8())
        .map_err(|e| e.to_string())?;
    Ok(format!("data:image/jpeg;base64,{}", base64(&jpeg)))
}

/// Standard base64 with padding.
fn base64(data: &[u8]) -> String {
    const CHARS: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    for chunk in data.chunks(3) {
        let b = [
            chunk[0],
            chunk.get(1).copied().unwrap_or(0),
            chunk.get(2).copied().unwrap_or(0),
        ];
        let n = (u32::from(b[0]) << 16) | (u32::from(b[1]) << 8) | u32::from(b[2]);
        for i in 0..4 {
            if i <= chunk.len() {
                out.push(CHARS[((n >> (18 - 6 * i)) & 63) as usize] as char);
            } else {
                out.push('=');
            }
        }
    }
    out
}

fn client_for(config: &Config) -> Result<ImmichClient, ApiError> {
    ImmichClient::new(&config.api).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to create client: {}", e),
        )
    })
}

/// Download the photo, find the person's face and decode it, or take it from the cache.
async fn load_photo(
    client: &ImmichClient,
    asset_id: &str,
    person_id: &str,
    use_preview: bool,
) -> Result<(Arc<CachedPhoto>, bool), ApiError> {
    let key = format!("{asset_id}|{person_id}|{use_preview}");
    if let Some(photo) = cached_photo(&key) {
        return Ok((photo, true));
    }

    let face = client
        .get_face_for_person(asset_id, person_id)
        .await
        .map_err(|e| (StatusCode::BAD_GATEWAY, e.to_string()))?
        .ok_or_else(|| {
            (
                StatusCode::UNPROCESSABLE_ENTITY,
                "No face of this person was found in this photo.".to_string(),
            )
        })?;

    let download = if use_preview {
        client.download_asset_preview(asset_id).await
    } else {
        client.download_asset(asset_id).await
    };
    let bytes = download.map_err(|e| (StatusCode::BAD_GATEWAY, e.to_string()))?;

    let decoded = tokio::task::spawn_blocking(move || {
        let image = load_image_with_orientation(&bytes).map_err(|e| e.to_string())?;
        let original = jpeg_data_url(&image.thumbnail(ORIGINAL_EDGE, ORIGINAL_EDGE))?;
        Ok::<_, String>((image, original))
    })
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let (image, original) = decoded.map_err(|e| {
        (
            StatusCode::UNPROCESSABLE_ENTITY,
            format!("The photo could not be read: {e}"),
        )
    })?;

    let photo = Arc::new(CachedPhoto {
        face_box: face_box(&face),
        face,
        image,
        original,
        loaded_at: Instant::now(),
    });
    store_photo(key, photo.clone());
    Ok((photo, false))
}

/// Skip reasons of the filters, by filter key.
const FILTER_REASONS: &[(&str, &[&str])] = &[
    ("face_resolution", &["face_too_small"]),
    ("crop", &["excessive_padding"]),
    ("brightness", &["too_dark", "too_bright"]),
    ("blur", &["too_blurry"]),
    ("head_pose", &["head_turned"]),
    ("eye_filter", &["eyes_closed"]),
];

fn filter_of(reason: &str) -> Option<&'static str> {
    FILTER_REASONS
        .iter()
        .find(|(_, reasons)| reasons.contains(&reason))
        .map(|(key, _)| *key)
}

fn filter_enabled(p: &ProcessingConfig, key: &str) -> bool {
    match key {
        "face_resolution" => p.face_resolution.enabled,
        "crop" => p.crop.enabled,
        "brightness" => p.brightness.enabled,
        "blur" => p.blur.enabled,
        "head_pose" => p.head_pose.enabled,
        "eye_filter" => p.eye_filter.enabled,
        _ => false,
    }
}

/// The settings with every filter switched on, so that all values are measured.
fn measuring(p: &ProcessingConfig) -> ProcessingConfig {
    let mut m = p.clone();
    m.face_resolution.enabled = true;
    m.crop.enabled = true;
    m.brightness.enabled = true;
    m.blur.enabled = true;
    m.head_pose.enabled = true;
    m.eye_filter.enabled = true;
    m
}

/// Whether the value of a filter was measured.
fn measured(values: &Measured, key: &str) -> bool {
    match key {
        "face_resolution" => values.face_size.is_some(),
        "crop" => values.padding_percent.is_some(),
        "brightness" => values.brightness.is_some(),
        "blur" => values.sharpness.is_some(),
        "head_pose" => values.yaw.is_some(),
        "eye_filter" => values.ear.is_some(),
        _ => false,
    }
}

fn read_values(ctx: &PipelineContext) -> Measured {
    let float = |key: &str| ctx.get_computed(key).and_then(|v| v.as_float());
    let pose = ctx
        .get_computed(computed_keys::HEAD_POSE)
        .and_then(|v| v.as_head_pose());
    Measured {
        face_size: ctx
            .get_computed(computed_keys::FACE_SIZE)
            .and_then(|v| v.as_int()),
        padding_percent: float(computed_keys::PADDING_PERCENT),
        brightness: float(computed_keys::BRIGHTNESS),
        sharpness: float(computed_keys::BLUR_METRIC),
        yaw: pose.map(|p| p.yaw),
        pitch: pose.map(|p| p.pitch),
        ear: float(computed_keys::EAR),
    }
}

/// Outcome of running the steps on one photo.
struct Run {
    values: Measured,
    filters: Vec<FilterVerdict>,
    frame: Option<DynamicImage>,
    problem: Option<String>,
}

/// Run the job's steps on the photo, measuring every filter and going on after a filter fails.
async fn run_steps(
    photo: &CachedPhoto,
    asset_id: &str,
    timestamp: String,
    config: &Config,
) -> Result<Run, ApiError> {
    let wanted = &config.processing;
    let mut measure_config = config.clone();
    measure_config.processing = measuring(wanted);

    let mut steps: Vec<Box<dyn ProcessingStep>> = vec![
        Box::new(FaceResolutionStep),
        Box::new(CropAndResizeStep),
        Box::new(BrightnessStep),
        Box::new(HeadPoseStep),
        Box::new(BlurStep),
        Box::new(LandmarksStep),
        Box::new(EyeFilterStep),
        Box::new(AlignmentStep),
    ];
    if wanted.timestamp.enabled {
        steps.push(Box::new(TimestampStep));
    }

    let mut ctx = PipelineContext::new(asset_id.to_string(), timestamp, photo.face.clone())
        .with_image(photo.image.clone());
    let mut failed: HashMap<&'static str, (String, Option<String>)> = HashMap::new();
    let mut problem = None;

    for step in &steps {
        match step.execute(ctx, &measure_config).await {
            StepOutcome::Continue(next) => ctx = next,
            StepOutcome::Skip {
                ctx: next,
                reason,
                detail,
            } => {
                ctx = next;
                if let Some(key) = filter_of(&reason) {
                    failed.entry(key).or_insert((reason, detail));
                } else {
                    problem = Some(problem_text(&reason, detail.as_deref()));
                    break;
                }
            }
            StepOutcome::Error { error, .. } => {
                return Err((StatusCode::INTERNAL_SERVER_ERROR, error));
            }
        }
    }

    let values = read_values(&ctx);
    let filters = FILTER_REASONS
        .iter()
        .map(|(key, _)| {
            let failure = failed.remove(key);
            let passed = if failure.is_some() {
                Some(false)
            } else if measured(&values, key) {
                Some(true)
            } else {
                None
            };
            let (reason, detail) = failure.map_or((None, None), |(r, d)| (Some(r), d));
            FilterVerdict {
                key,
                enabled: filter_enabled(wanted, key),
                passed,
                reason,
                detail,
            }
        })
        .collect();
    let frame = if problem.is_none() {
        ctx.image.take()
    } else {
        None
    };
    Ok(Run {
        values,
        filters,
        frame,
        problem,
    })
}

/// A readable message for a step that stopped the preview.
fn problem_text(reason: &str, detail: Option<&str>) -> String {
    let text = match reason {
        "landmarks_failed" => "The eyes could not be found in this photo",
        "alignment" => "The face could not be aligned",
        "crop_failed" => "The face could not be cut out",
        "decode_failed" => "The photo could not be read",
        other => other,
    };
    match detail {
        Some(d) if !d.is_empty() => format!("{text} ({d})."),
        _ => format!("{text}."),
    }
}

/// Whether the photo would be used: no step stopped and every switched-on filter passed.
fn would_be_used(filters: &[FilterVerdict], problem: &Option<String>) -> bool {
    problem.is_none() && filters.iter().all(|f| !f.enabled || f.passed != Some(false))
}

/// Preview the quality filters and the resulting frame on one photo.
pub async fn preview(
    State(state): State<AppState>,
    Json(request): Json<PreviewRequest>,
) -> Result<Json<PreviewResponse>, ApiError> {
    validate_person_id(&request.person_id)?;
    validate_person_id(&request.asset_id)
        .map_err(|(code, text)| (code, text.replace("person_id", "asset_id")))?;

    // Same limits as for saved settings: an unchecked size (e.g. a huge
    // output.size) could exhaust the memory and take a running job down too.
    request
        .processing
        .validate()
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;

    let mut config = state.config.read().await.clone();
    config.processing = request.processing;
    let client = client_for(&config)?;

    let (photo, cached) = load_photo(
        &client,
        &request.asset_id,
        &request.person_id,
        config.processing.use_preview,
    )
    .await?;

    let started = Instant::now();
    let timestamp = request
        .timestamp
        .unwrap_or_else(|| request.asset_id.clone());
    let run = run_steps(&photo, &request.asset_id, timestamp, &config).await?;
    let frame = run
        .frame
        .as_ref()
        .map(jpeg_data_url)
        .transpose()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;
    let millis = started.elapsed().as_millis() as u64;
    tracing::debug!("Preview of {} took {} ms (cached: {})", request.asset_id, millis, cached);

    Ok(Json(PreviewResponse {
        asset_id: request.asset_id,
        original: photo.original.clone(),
        face_box: photo.face_box,
        frame,
        used: would_be_used(&run.filters, &run.problem),
        values: run.values,
        filters: run.filters,
        problem: run.problem,
        cached,
        millis,
    }))
}

/// Query of the sample photos: the selection of the guided view.
#[derive(Deserialize)]
pub struct SamplesQuery {
    pub person_id: String,
    pub album_ids: Option<String>,
    pub people_ids: Option<String>,
    pub recipient_ids: Option<String>,
    pub date_from: Option<String>,
    pub date_to: Option<String>,
    pub count: Option<usize>,
}

/// A photo to preview.
#[derive(Debug, Serialize, PartialEq)]
pub struct Sample {
    pub id: String,
    pub timestamp: Option<String>,
}

#[derive(Serialize)]
pub struct SamplesResponse {
    /// Number of photos in the selection.
    pub total: usize,
    pub samples: Vec<Sample>,
}

fn asset_time(asset: &Asset) -> Option<String> {
    asset
        .file_created_at
        .clone()
        .or_else(|| asset.local_date_time.clone())
}

/// Up to `count` photos spread evenly over time.
fn spread(mut assets: Vec<Asset>, count: usize) -> Vec<Sample> {
    assets.sort_by_key(asset_time);
    let n = assets.len();
    if n == 0 || count == 0 {
        return Vec::new();
    }
    let take = count.min(n);
    (0..take)
        .map(|i| {
            let asset = &assets[i * n / take];
            Sample {
                id: asset.id.clone(),
                timestamp: asset_time(asset),
            }
        })
        .collect()
}

/// Sample photos of the selection (person, albums, people, period) to preview.
pub async fn preview_samples(
    State(state): State<AppState>,
    Query(query): Query<SamplesQuery>,
) -> Result<Json<SamplesResponse>, ApiError> {
    validate_person_id(&query.person_id)?;
    let config = state.config.read().await;
    let client = client_for(&config)?;
    drop(config);

    let shared = resolve_recipient_albums(&split_ids(query.recipient_ids.as_deref())).await?;
    let album_ids = merge_ids(split_ids(query.album_ids.as_deref()), shared);
    let people_ids = split_ids(query.people_ids.as_deref());
    let assets = client
        .get_assets_with_sources(
            &query.person_id,
            query.date_from.as_deref().filter(|d| !d.is_empty()),
            query.date_to.as_deref().filter(|d| !d.is_empty()),
            &album_ids,
            &people_ids,
        )
        .await
        .map_err(|e| (StatusCode::BAD_GATEWAY, e.to_string()))?;

    let total = assets.len();
    let count = query.count.unwrap_or(DEFAULT_SAMPLES).min(100);
    Ok(Json(SamplesResponse {
        total,
        samples: spread(assets, count),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn face() -> FaceData {
        FaceData {
            bounding_box_x1: 100.0,
            bounding_box_y1: 50.0,
            bounding_box_x2: 300.0,
            bounding_box_y2: 250.0,
            image_width: 1000,
            image_height: 500,
        }
    }

    fn asset(id: &str, time: Option<&str>) -> Asset {
        Asset {
            id: id.to_string(),
            device_asset_id: None,
            original_file_name: None,
            file_created_at: time.map(String::from),
            local_date_time: None,
        }
    }

    #[test]
    fn test_base64() {
        assert_eq!(base64(b""), "");
        assert_eq!(base64(b"f"), "Zg==");
        assert_eq!(base64(b"fo"), "Zm8=");
        assert_eq!(base64(b"foo"), "Zm9v");
        assert_eq!(base64(b"foobar"), "Zm9vYmFy");
        assert_eq!(base64(&[0xff, 0xfe]), "//4=");
    }

    #[test]
    fn test_face_box() {
        let b = face_box(&face());
        assert_eq!(
            b,
            FaceBox {
                x: 0.1,
                y: 0.1,
                w: 0.2,
                h: 0.4
            }
        );
    }

    #[test]
    fn test_filter_of() {
        assert_eq!(filter_of("too_dark"), Some("brightness"));
        assert_eq!(filter_of("too_bright"), Some("brightness"));
        assert_eq!(filter_of("eyes_closed"), Some("eye_filter"));
        assert_eq!(filter_of("landmarks_failed"), None);
    }

    #[test]
    fn test_measuring_switches_every_filter_on() {
        let mut p = ProcessingConfig::default();
        p.blur.enabled = false;
        p.head_pose.enabled = false;
        p.blur.min_sharpness = 77.0;
        let m = measuring(&p);
        for (key, _) in FILTER_REASONS {
            assert!(filter_enabled(&m, key), "{key}");
        }
        assert_eq!(m.blur.min_sharpness, 77.0);
        assert!(!filter_enabled(&p, "blur"));
    }

    #[test]
    fn test_would_be_used() {
        let verdict = |key, enabled, passed| FilterVerdict {
            key,
            enabled,
            passed,
            reason: None,
            detail: None,
        };
        let ok = vec![
            verdict("blur", true, Some(true)),
            verdict("head_pose", true, None),
            verdict("crop", false, Some(false)),
        ];
        assert!(would_be_used(&ok, &None));
        assert!(!would_be_used(&ok, &Some("no eyes".to_string())));
        let blurry = vec![verdict("blur", true, Some(false))];
        assert!(!would_be_used(&blurry, &None));
    }

    #[test]
    fn test_problem_text() {
        assert_eq!(
            problem_text("landmarks_failed", Some("no face")),
            "The eyes could not be found in this photo (no face)."
        );
        assert_eq!(problem_text("alignment", None), "The face could not be aligned.");
    }

    #[test]
    fn test_spread() {
        let assets = vec![
            asset("c", Some("2022-01-01")),
            asset("a", Some("2020-01-01")),
            asset("d", Some("2023-01-01")),
            asset("b", Some("2021-01-01")),
        ];
        let ids = |s: Vec<Sample>| s.into_iter().map(|x| x.id).collect::<Vec<_>>();
        assert_eq!(ids(spread(assets, 2)), vec!["a", "c"]);
        let all = spread(vec![asset("x", None), asset("y", Some("2020"))], 10);
        assert_eq!(all.len(), 2);
        assert_eq!(all[0].id, "x");
        assert!(spread(Vec::new(), 5).is_empty());
    }

    #[test]
    fn test_cache_keeps_at_most_max() {
        for i in 0..CACHE_MAX + 3 {
            store_photo(
                format!("test-{i}"),
                Arc::new(CachedPhoto {
                    face: face(),
                    image: DynamicImage::new_rgb8(1, 1),
                    original: String::new(),
                    face_box: face_box(&face()),
                    loaded_at: Instant::now(),
                }),
            );
        }
        let map = cache().lock().unwrap();
        assert!(map.len() <= CACHE_MAX);
        assert!(map.contains_key(&format!("test-{}", CACHE_MAX + 2)));
    }
}
