//! Application configuration.
//!
//! Configuration can be loaded from:
//! 1. TOML file (config.toml)
//! 2. Environment variables (prefixed with IMMICH_)
//! 3. Programmatic overrides

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::error::{Error, Result};

/// Default path to the config file, relative to the working directory.
pub const CONFIG_PATH: &str = "config/config.toml";

/// Main configuration struct.
///
/// This is the single source of truth for all configuration.
/// Pass this explicitly to functions that need it - no globals.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    /// Immich API configuration.
    pub api: ApiConfig,

    /// Face processing parameters.
    pub processing: ProcessingConfig,

    /// Video output settings.
    pub video: VideoConfig,

    /// Output directory for processed images and video.
    pub output_dir: PathBuf,

    /// Directory containing music files that can be mixed into the video
    /// (mounted at `/app/music` in Docker). Overridable via `MUSIC_DIR`.
    pub music_dir: PathBuf,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            api: ApiConfig::default(),
            processing: ProcessingConfig::default(),
            video: VideoConfig::default(),
            output_dir: PathBuf::from("output"),
            music_dir: PathBuf::from("music"),
        }
    }
}

/// Immich API connection settings.
#[derive(Clone, Serialize, Deserialize)]
pub struct ApiConfig {
    /// API key for authentication.
    pub api_key: String,

    /// Base URL of the Immich instance (e.g., "http://192.168.1.94:2283/api").
    pub base_url: String,

    /// Request timeout in seconds.
    #[serde(default = "default_timeout")]
    pub timeout_secs: u64,
}

impl std::fmt::Debug for ApiConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ApiConfig")
            .field("api_key", &"[REDACTED]")
            .field("base_url", &self.base_url)
            .field("timeout_secs", &self.timeout_secs)
            .finish()
    }
}

fn default_timeout() -> u64 {
    30
}

impl Default for ApiConfig {
    fn default() -> Self {
        Self {
            api_key: String::new(),
            base_url: String::new(),
            timeout_secs: default_timeout(),
        }
    }
}

// ============================================================================
// Step Configurations
// ============================================================================

/// Face resolution validation configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FaceResolutionConfig {
    /// Whether face resolution validation is enabled.
    pub enabled: bool,

    /// Minimum face width/height in pixels.
    /// Faces smaller than this will be skipped.
    pub min_size: u32,
}

impl Default for FaceResolutionConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            min_size: 80,
        }
    }
}

impl FaceResolutionConfig {
    /// Validate the configuration values.
    pub fn validate(&self) -> Result<()> {
        if self.enabled && self.min_size == 0 {
            return Err(Error::Config(
                "Face resolution min_size must be greater than 0".to_string(),
            ));
        }
        Ok(())
    }
}

/// Brightness validation configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrightnessConfig {
    /// Whether brightness validation is enabled.
    pub enabled: bool,

    /// Minimum acceptable brightness (0.0-1.0).
    /// Images darker than this will be skipped.
    pub min_brightness: f32,

    /// Maximum acceptable brightness (0.0-1.0).
    /// Images brighter than this will be skipped.
    pub max_brightness: f32,
}

impl Default for BrightnessConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            min_brightness: 0.1,
            max_brightness: 0.9,
        }
    }
}

impl BrightnessConfig {
    /// Validate the configuration values.
    pub fn validate(&self) -> Result<()> {
        if !self.enabled {
            return Ok(());
        }
        if self.min_brightness < 0.0 || self.min_brightness > 1.0 {
            return Err(Error::Config(
                "Brightness min_brightness must be between 0.0 and 1.0".to_string(),
            ));
        }
        if self.max_brightness < 0.0 || self.max_brightness > 1.0 {
            return Err(Error::Config(
                "Brightness max_brightness must be between 0.0 and 1.0".to_string(),
            ));
        }
        if self.min_brightness >= self.max_brightness {
            return Err(Error::Config(
                "Brightness min_brightness must be less than max_brightness".to_string(),
            ));
        }
        Ok(())
    }
}

/// Blur detection configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlurConfig {
    /// Whether blur detection is enabled.
    pub enabled: bool,

    /// Minimum Laplacian variance threshold.
    /// Images with Laplacian variance below this are considered blurry.
    /// Typical values: < 100 = blurry, 100–500 = borderline, > 500 = sharp.
    pub min_sharpness: f32,
}

impl Default for BlurConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            min_sharpness: 75.0,
        }
    }
}

impl BlurConfig {
    /// Validate the configuration values.
    pub fn validate(&self) -> Result<()> {
        if self.enabled && self.min_sharpness < 0.0 {
            return Err(Error::Config(
                "Blur min_sharpness must be non-negative".to_string(),
            ));
        }
        Ok(())
    }
}

/// Crop padding filter configuration.
///
/// Skips images where too much of the crop region falls outside the original
/// image bounds (filled by edge-pixel replication).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CropConfig {
    /// Whether crop padding filtering is enabled.
    pub enabled: bool,

    /// Maximum allowed padding percentage (0.0-100.0).
    /// Images where the replicated-fill area exceeds this will be skipped.
    pub max_padding_percent: f32,
}

impl Default for CropConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_padding_percent: 30.0,
        }
    }
}

impl CropConfig {
    /// Validate the configuration values.
    pub fn validate(&self) -> Result<()> {
        if self.enabled && !(0.0..=100.0).contains(&self.max_padding_percent) {
            return Err(Error::Config(
                "Crop max_padding_percent must be between 0.0 and 100.0".to_string(),
            ));
        }
        Ok(())
    }
}

/// Aspect ratio of the output frames, written as width:height.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum AspectRatio {
    /// Square frames (the original format).
    #[default]
    #[serde(rename = "1:1")]
    Square,
    /// Portrait 4:5, as used by Instagram posts.
    #[serde(rename = "4:5")]
    Portrait4x5,
    /// Landscape 5:4.
    #[serde(rename = "5:4")]
    Landscape5x4,
    /// Widescreen 16:9.
    #[serde(rename = "16:9")]
    Landscape16x9,
    /// Vertical 9:16, as used by phone stories and reels.
    #[serde(rename = "9:16")]
    Portrait9x16,
}

impl AspectRatio {
    /// The width and height parts of the ratio, e.g. `(16, 9)`.
    pub fn parts(self) -> (u32, u32) {
        match self {
            Self::Square => (1, 1),
            Self::Portrait4x5 => (4, 5),
            Self::Landscape5x4 => (5, 4),
            Self::Landscape16x9 => (16, 9),
            Self::Portrait9x16 => (9, 16),
        }
    }
}

/// Output/resize configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputConfig {
    /// Output image size in pixels: the long edge of the frame (width and height
    /// for square frames).
    pub size: u32,

    /// Aspect ratio of the frames. The long edge is `size`.
    #[serde(default)]
    pub aspect_ratio: AspectRatio,

    /// Keep intermediate images (original, cropped) for inspection.
    pub keep_intermediates: bool,

    /// Number of versioned output folders to keep per person (0 = unlimited).
    /// Older versions are removed after a successful run. Default 5, so the
    /// disk does not fill up unnoticed.
    #[serde(default = "default_keep_versions")]
    pub keep_versions: u32,

    /// Also save the downloaded photo of every frame in `originals/`.
    #[serde(default)]
    pub keep_originals: bool,

    /// Keep the processed frames in `images/` after the video was made.
    /// Without a video they are the result and always stay.
    #[serde(default = "default_true")]
    pub keep_frames: bool,
}

impl Default for OutputConfig {
    fn default() -> Self {
        Self {
            size: 512,
            aspect_ratio: AspectRatio::Square,
            keep_intermediates: false,
            keep_versions: default_keep_versions(),
            keep_originals: false,
            keep_frames: true,
        }
    }
}

impl OutputConfig {
    /// Frame width and height in pixels. The long edge is `size`, the short edge
    /// follows the aspect ratio. Both are even, as the video encoders need it.
    pub fn dimensions(&self) -> (u32, u32) {
        let (w, h) = self.aspect_ratio.parts();
        let long = u64::from(self.size);
        let short = |a: u32, b: u32| {
            let exact = (long * u64::from(a) + u64::from(b) / 2) / u64::from(b);
            u32::try_from(exact).unwrap_or(u32::MAX)
        };
        let even = |v: u32| (v & !1).max(2);
        if w >= h {
            (even(self.size), even(short(h, w)))
        } else {
            (even(short(w, h)), even(self.size))
        }
    }

    /// Validate the configuration values.
    pub fn validate(&self) -> Result<()> {
        if self.size < 64 {
            return Err(Error::Config(
                "Output size must be at least 64 pixels".to_string(),
            ));
        }
        if self.size > 4096 {
            return Err(Error::Config(
                "Output size must be at most 4096 pixels".to_string(),
            ));
        }
        Ok(())
    }
}

/// Head pose estimation configuration.
///
/// Uses DMHead ONNX model to estimate head pose angles and filter
/// non-front-facing faces.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeadPoseConfig {
    /// Whether head pose filtering is enabled.
    pub enabled: bool,

    /// Maximum allowed yaw angle (left/right turn) in degrees.
    pub max_yaw: f32,

    /// Maximum allowed pitch angle (up/down tilt) in degrees.
    pub max_pitch: f32,
}

impl Default for HeadPoseConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_yaw: 20.0,
            max_pitch: 20.0,
        }
    }
}

impl HeadPoseConfig {
    /// Validate the configuration values.
    pub fn validate(&self) -> Result<()> {
        if self.enabled {
            if self.max_yaw < 0.0 || self.max_yaw > 90.0 {
                return Err(Error::Config(
                    "Head pose max_yaw must be between 0 and 90 degrees".to_string(),
                ));
            }
            if self.max_pitch < 0.0 || self.max_pitch > 90.0 {
                return Err(Error::Config(
                    "Head pose max_pitch must be between 0 and 90 degrees".to_string(),
                ));
            }
        }
        Ok(())
    }
}

/// Eye filter configuration for blink detection.
///
/// Uses Eye Aspect Ratio (EAR) computed from facial landmarks to detect
/// closed eyes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EyeFilterConfig {
    /// Whether eye filtering is enabled.
    pub enabled: bool,

    /// Minimum Eye Aspect Ratio (EAR) threshold.
    /// Eyes with EAR below this are considered closed.
    pub min_ear: f32,
}

impl Default for EyeFilterConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            min_ear: 0.12,
        }
    }
}

impl EyeFilterConfig {
    /// Validate the configuration values.
    pub fn validate(&self) -> Result<()> {
        if self.enabled && !(0.0..=0.5).contains(&self.min_ear) {
            return Err(Error::Config(
                "Eye filter min_ear must be between 0.0 and 0.5".to_string(),
            ));
        }
        Ok(())
    }
}

/// Face alignment configuration for landmark-based alignment.
///
/// Aligns faces based on eye positions detected from facial landmarks,
/// ensuring consistent eye placement across all images for smoother timelapses.
///
/// Uses a single `eye_distance` parameter to control framing. Eye positions are derived by
/// estimating the visual face center (see `FACE_CENTER_OFFSET_IN_IPD`) from standard face
/// proportions, then placing that point at the image center with a small downward offset to
/// show more neck.
///
/// Note: Face alignment is always enabled and is a core part of the pipeline.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlignmentConfig {
    /// Distance between left and right eye centers as a fraction of output
    /// image width (0.0-1.0). Default 0.15 means the eyes span 15% of the
    /// output width. Larger values zoom in (bigger face), smaller values
    /// zoom out (more background).
    pub eye_distance: f32,
}

impl Default for AlignmentConfig {
    fn default() -> Self {
        Self { eye_distance: 0.15 }
    }
}

impl AlignmentConfig {
    /// How far below the eye line the face center sits, in units of
    /// inter-pupillary distance (IPD). Value of 0.5 means the face center
    /// is 0.5 × IPD below the eyes.
    ///
    /// From face proportions: the vertical midpoint between hairline and chin
    /// is ~0.4× IPD below the eyes; adding a small neck bias brings it to
    /// 0.5× IPD. The alignment formula places this point at image center (y=0.5):
    ///
    ///   eye_y = 0.5 - eye_distance * FACE_CENTER_OFFSET_IN_IPD
    const FACE_CENTER_BELOW_EYES: f32 = 0.5;

    /// Validate the configuration values.
    pub fn validate(&self) -> Result<()> {
        if self.eye_distance <= 0.0 || self.eye_distance >= 1.0 {
            return Err(Error::Config(
                "Alignment eye_distance must be between 0.0 and 1.0 (exclusive)".to_string(),
            ));
        }
        Ok(())
    }

    /// Target X position for the left eye as a fraction of output width.
    /// Eyes are centered horizontally: `left_eye_x = (1 - eye_distance) / 2`.
    pub fn left_eye_x(&self) -> f32 {
        (1.0 - self.eye_distance) / 2.0
    }

    /// Target Y position for both eyes as a fraction of output height.
    /// Derived by placing the estimated face center at the image center:
    /// `eye_y = 0.5 - eye_distance * FACE_CENTER_BELOW_EYES`.
    pub fn left_eye_y(&self) -> f32 {
        0.5 - self.eye_distance * Self::FACE_CENTER_BELOW_EYES
    }
}

/// Position for timestamp text overlay.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum TextPosition {
    #[default]
    TopLeft,
    TopCenter,
    TopRight,
    BottomLeft,
    BottomCenter,
    BottomRight,
}

/// Timestamp overlay configuration.
///
/// Overlays date information on the processed image.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimestampConfig {
    /// Whether timestamp overlay is enabled.
    pub enabled: bool,

    /// Position of the timestamp text.
    pub position: TextPosition,

    /// Whether to display the year.
    pub year: bool,

    /// Whether to display the month.
    pub month: bool,

    /// Whether to display the day.
    pub day: bool,

    /// Height of the text in percent of the frame height. `None` keeps the
    /// original fixed size of 7 pixels.
    #[serde(default)]
    pub font_size: Option<f32>,

    /// How the date is written.
    #[serde(default)]
    pub format: DateFormat,

    /// Font file (as listed by `/api/fonts`). Empty keeps the built-in pixel font.
    #[serde(default)]
    pub font: String,

    /// Text colour as hex "rrggbb".
    #[serde(default = "default_date_color")]
    pub color: String,

    /// Dark shadow behind the text.
    #[serde(default = "default_true")]
    pub shadow: bool,

    /// Language of month and weekday names ("de" or "en").
    #[serde(default = "default_date_lang")]
    pub lang: String,
}

fn default_date_color() -> String {
    "ffffff".to_string()
}

fn default_date_lang() -> String {
    "de".to_string()
}

/// How the date in the frames is written (example: 17 May 2024).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum DateFormat {
    /// 2024-05-17
    #[default]
    Iso,
    /// 17.05.2024
    DmyDot,
    /// 17.5.2024
    DmyDotShort,
    /// 17.05.24
    DmyDotYy,
    /// 17-05-2024
    DmyDash,
    /// 17/05/2024
    DmySlash,
    /// 05-17-2024
    MdyDash,
    /// 05/17/2024
    MdySlash,
    /// 17. Mai 2024
    Long,
    /// 17. Mai 2024 / 3. Sept. 2024
    LongShort,
    /// Mai 2024
    MonthYear,
    /// Freitag, 17. Mai 2024
    Weekday,
}

impl Default for TimestampConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            position: TextPosition::BottomLeft,
            year: true,
            month: true,
            day: true,
            font_size: None,
            format: DateFormat::Iso,
            font: String::new(),
            color: default_date_color(),
            shadow: true,
            lang: default_date_lang(),
        }
    }
}

impl TimestampConfig {
    /// Smallest and largest font size in percent of the frame height.
    pub const FONT_SIZE_RANGE: std::ops::RangeInclusive<f32> = 0.5..=20.0;

    /// Validate the configuration values.
    pub fn validate(&self) -> Result<()> {
        if let Some(size) = self.font_size {
            if !Self::FONT_SIZE_RANGE.contains(&size) {
                return Err(Error::Config(
                    "Timestamp font_size must be between 0.5 and 20 percent".to_string(),
                ));
            }
        }
        let c = self.color.trim_start_matches('#');
        if c.len() != 6 || !c.chars().all(|ch| ch.is_ascii_hexdigit()) {
            return Err(Error::Config(
                "Timestamp color must be a hex colour like ffffff".to_string(),
            ));
        }
        Ok(())
    }
}

/// Time range granularity for photo limiting.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum TimeRange {
    #[default]
    Day,
    Week,
    Month,
}

/// Time interval configuration.
///
/// Limits the number of successfully processed photos per time range
/// (day/week/month) to avoid over-representation of busy periods.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeIntervalConfig {
    /// Whether photo limiting is enabled.
    pub enabled: bool,

    /// Maximum number of photos per time range.
    pub max_photos: u32,

    /// Time range granularity.
    pub time_range: TimeRange,
}

impl Default for TimeIntervalConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_photos: 5,
            time_range: TimeRange::Day,
        }
    }
}

impl TimeIntervalConfig {
    /// Validate the configuration values.
    pub fn validate(&self) -> Result<()> {
        if self.enabled && self.max_photos == 0 {
            return Err(Error::Config(
                "Time interval max_photos must be greater than 0".to_string(),
            ));
        }
        if self.enabled && self.max_photos > 100 {
            return Err(Error::Config(
                "Time interval max_photos must be at most 100".to_string(),
            ));
        }
        Ok(())
    }
}

// ============================================================================
// Main Processing Configuration
// ============================================================================

/// Face processing parameters.
///
/// Each pipeline step has its own sub-configuration with an `enabled` flag.
/// This allows fine-grained control over which steps run and their parameters.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ProcessingConfig {
    /// Number of parallel workers for processing.
    pub max_workers: usize,

    /// Whether to download Immich preview images (1440p JPEG) instead of originals.
    /// Preview images are much faster to decode and sufficient for most use cases.
    /// Disable only if you need full original resolution (e.g., very small faces in high-res photos).
    #[serde(default = "default_true")]
    pub use_preview: bool,

    /// Face resolution validation settings.
    #[serde(default)]
    pub face_resolution: FaceResolutionConfig,

    /// Crop padding filter settings.
    #[serde(default)]
    pub crop: CropConfig,

    /// Blur detection settings.
    #[serde(default)]
    pub blur: BlurConfig,

    /// Brightness validation settings.
    #[serde(default)]
    pub brightness: BrightnessConfig,

    /// Head pose estimation settings.
    #[serde(default)]
    pub head_pose: HeadPoseConfig,

    /// Eye filter settings (blink detection).
    #[serde(default)]
    pub eye_filter: EyeFilterConfig,

    /// Output image settings.
    #[serde(default)]
    pub output: OutputConfig,

    /// Face alignment settings (landmark-based).
    #[serde(default)]
    pub alignment: AlignmentConfig,

    /// Timestamp overlay settings.
    #[serde(default)]
    pub timestamp: TimestampConfig,

    /// Time interval settings.
    #[serde(default)]
    pub time_interval: TimeIntervalConfig,
}

impl Default for ProcessingConfig {
    fn default() -> Self {
        Self {
            max_workers: 1,
            use_preview: true,
            face_resolution: FaceResolutionConfig::default(),
            crop: CropConfig::default(),
            blur: BlurConfig::default(),
            brightness: BrightnessConfig::default(),
            head_pose: HeadPoseConfig::default(),
            eye_filter: EyeFilterConfig::default(),
            output: OutputConfig::default(),
            alignment: AlignmentConfig::default(),
            timestamp: TimestampConfig::default(),
            time_interval: TimeIntervalConfig::default(),
        }
    }
}

impl ProcessingConfig {
    /// Validate all step configurations.
    pub fn validate(&self) -> Result<()> {
        if self.max_workers == 0 {
            return Err(Error::Config(
                "max_workers must be greater than 0".to_string(),
            ));
        }
        self.face_resolution.validate()?;
        self.crop.validate()?;
        self.blur.validate()?;
        self.brightness.validate()?;
        self.head_pose.validate()?;
        self.eye_filter.validate()?;
        self.output.validate()?;
        self.alignment.validate()?;
        self.timestamp.validate()?;
        self.time_interval.validate()?;
        Ok(())
    }
}

fn default_true() -> bool {
    true
}

fn default_keep_versions() -> u32 {
    5
}

// ============================================================================
// Video Configuration
// ============================================================================

/// Video compilation settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct VideoConfig {
    /// Whether to compile video after processing.
    pub enabled: bool,

    /// Output video framerate.
    pub framerate: u32,

    /// Video codec (e.g., "libx264").
    pub codec: String,

    /// Constant Rate Factor for quality (0-51, lower = better, 18-28 recommended).
    pub crf: u32,

    /// Music tracks (file names inside the music directory) mixed into the
    /// video, played one after another in this order. Empty = no audio.
    pub music_tracks: Vec<String>,

    /// Length of the audio fade-out at the end of the video, in seconds.
    pub music_fade_out_secs: f32,

    /// Fit the video to the music: framerate = image count / music length,
    /// clamped to `[min_framerate, max_framerate]`. Ignored without music.
    pub fit_to_music: bool,

    /// Fixed video length: framerate = image count / `target_duration_secs`,
    /// clamped to `[min_framerate, max_framerate]`. Takes precedence over
    /// `fit_to_music`.
    pub fit_to_duration: bool,

    /// Wanted video length in seconds, used with `fit_to_duration`.
    pub target_duration_secs: f32,

    /// Lowest framerate allowed when fitting the video to the music or a length.
    pub min_framerate: f32,

    /// Highest framerate allowed when fitting the video to the music or a length.
    pub max_framerate: f32,

    /// Title card shown before the timelapse.
    pub intro: TitleCardConfig,

    /// Closing card shown after the timelapse.
    pub outro: TitleCardConfig,
}

/// An animated title card (intro or outro): a background image, or a plain
/// colour, with a title, subtitle and an optional longer text.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct TitleCardConfig {
    /// Whether the card is added to the video.
    pub enabled: bool,
    /// Immich asset used as background image.
    pub asset_id: Option<String>,
    /// Uploaded background image (file name in the title image folder).
    /// Used when `asset_id` is empty.
    pub upload: Option<String>,
    /// Length of the card in seconds.
    pub duration_secs: f32,
    pub title: String,
    pub subtitle: String,
    /// Longer text below the subtitle, may span several lines.
    pub text: String,
    /// Font file name (e.g. `DejaVuSans-Bold.ttf`), see `/api/fonts`.
    pub font: String,
    /// Colour of the title as `#rrggbb`.
    pub color: String,
    /// Colour of the subtitle as `#rrggbb`; empty = like the title.
    pub subtitle_color: String,
    /// Colour of the longer text as `#rrggbb`; empty = like the title.
    pub text_color: String,
    /// Background colour as `#rrggbb`, used without an image.
    pub background: String,
    /// Title height in percent of the frame height; subtitle and text are smaller.
    pub size: f32,
    /// Darkening of the background image so the text stays readable (0 – 0.9).
    pub dim: f32,
    /// Slow movement of the background image (off = still image).
    pub zoom: bool,
    /// Kind of movement when `zoom` is on: `auto` (pan along the photo's
    /// format and zoom towards the face), `zoom` (into the middle),
    /// `pan_right`, `pan_left`, `pan_down`, `pan_up` (camera pan, like in
    /// Memories) or `random` (one of the pans).
    pub motion: String,
    /// Add a line with the date of the oldest and the newest photo below the text.
    pub dates: bool,
}

impl Default for TitleCardConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            asset_id: None,
            upload: None,
            duration_secs: 5.0,
            title: String::new(),
            subtitle: String::new(),
            text: String::new(),
            font: "DejaVuSans-Bold.ttf".to_string(),
            color: "#ffffff".to_string(),
            subtitle_color: String::new(),
            text_color: String::new(),
            background: "#000000".to_string(),
            size: 8.0,
            dim: 0.15,
            zoom: true,
            motion: "auto".to_string(),
            dates: false,
        }
    }
}

/// `#rrggbb` → `rrggbb`, None for anything else.
pub fn hex_color(value: &str) -> Option<&str> {
    let hex = value.strip_prefix('#')?;
    (hex.len() == 6 && hex.chars().all(|c| c.is_ascii_hexdigit())).then_some(hex)
}

/// A plain file name: no folders, no leading dot, no control characters.
pub fn plain_file_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 200
        && !name.starts_with('.')
        && !name.contains("..")
        && !name.contains('/')
        && !name.contains('\\')
        && !name.contains('\'')
        && !name.chars().any(|c| c.is_control())
}

impl TitleCardConfig {
    /// Validate the card; `which` is `intro` or `outro` for the messages.
    pub fn validate(&self, which: &str) -> Result<()> {
        let err = |m: &str| Err(Error::Config(format!("{}: {}", which, m)));
        if !(1.0..=30.0).contains(&self.duration_secs) {
            return err("length must be between 1 and 30 seconds");
        }
        if !(2.0..=25.0).contains(&self.size) {
            return err("text size must be between 2 and 25 %");
        }
        if !(0.0..=0.9).contains(&self.dim) {
            return err("darkening must be between 0 and 0.9");
        }
        if hex_color(&self.color).is_none() || hex_color(&self.background).is_none() {
            return err("colours must be given as #rrggbb");
        }
        if !self.font.is_empty() && !plain_file_name(&self.font) {
            return err("invalid font name");
        }
        if let Some(id) = &self.asset_id {
            if id.is_empty()
                || id.len() > 64
                || !id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
            {
                return err("invalid asset id");
            }
        }
        if let Some(name) = &self.upload {
            if !plain_file_name(name) {
                return err("invalid image name");
            }
        }
        if self.title.len() > 300 || self.subtitle.len() > 300 || self.text.len() > 3000 {
            return err("text is too long");
        }
        Ok(())
    }
}

impl Default for VideoConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            framerate: 20,
            codec: "libx264".to_string(),
            crf: 23,
            music_tracks: Vec::new(),
            music_fade_out_secs: 3.0,
            fit_to_music: false,
            fit_to_duration: false,
            target_duration_secs: 30.0,
            min_framerate: 5.0,
            max_framerate: 30.0,
            intro: TitleCardConfig::default(),
            outro: TitleCardConfig::default(),
        }
    }
}

/// Valid video codecs that can be used with FFmpeg.
const VALID_CODECS: &[&str] = &["libx264", "libx265", "libvpx", "libvpx-vp9", "libaom-av1"];

impl VideoConfig {
    /// Validate the configuration values.
    pub fn validate(&self) -> Result<()> {
        if self.framerate == 0 {
            return Err(Error::Config(
                "Video framerate must be greater than 0".to_string(),
            ));
        }
        if self.framerate > 120 {
            return Err(Error::Config(
                "Video framerate must be at most 120".to_string(),
            ));
        }
        if self.crf > 51 {
            return Err(Error::Config(
                "Video CRF must be between 0 and 51".to_string(),
            ));
        }
        if !VALID_CODECS.contains(&self.codec.as_str()) {
            return Err(Error::Config(format!(
                "Video codec must be one of: {}, got '{}'",
                VALID_CODECS.join(", "),
                self.codec
            )));
        }
        if !(0.0..=30.0).contains(&self.music_fade_out_secs) {
            return Err(Error::Config(
                "Music fade-out must be between 0 and 30 seconds".to_string(),
            ));
        }
        if !(1.0..=3600.0).contains(&self.target_duration_secs) {
            return Err(Error::Config(
                "Video length must be between 1 and 3600 seconds".to_string(),
            ));
        }
        let fps_ok = |v: f32| v > 0.0 && v <= 120.0;
        if !fps_ok(self.min_framerate) || !fps_ok(self.max_framerate) {
            return Err(Error::Config(
                "Min/max framerate must be between 0 (exclusive) and 120".to_string(),
            ));
        }
        if self.min_framerate > self.max_framerate {
            return Err(Error::Config(
                "Min framerate must not be greater than max framerate".to_string(),
            ));
        }
        for track in &self.music_tracks {
            if track.is_empty()
                || track.contains("..")
                || track.contains('/')
                || track.contains('\\')
            {
                return Err(Error::Config(format!(
                    "Invalid music track name '{}'",
                    track
                )));
            }
        }
        self.intro.validate("Intro")?;
        self.outro.validate("Outro")?;
        Ok(())
    }
}

// ============================================================================
// Persistence
// ============================================================================

/// Persistable configuration (excludes sensitive API credentials).
///
/// This struct contains only the settings that can safely be written to disk.
/// API credentials should always come from environment variables.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistableConfig {
    pub processing: ProcessingConfig,
    pub video: VideoConfig,
}

impl From<&Config> for PersistableConfig {
    fn from(config: &Config) -> Self {
        Self {
            processing: config.processing.clone(),
            video: config.video.clone(),
        }
    }
}

// ============================================================================
// Config Loading and Validation
// ============================================================================

impl Config {
    /// Load configuration from a TOML file.
    pub fn from_file(path: impl AsRef<std::path::Path>) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: Config = toml::from_str(&content).map_err(|e| Error::Config(e.to_string()))?;
        Ok(config)
    }

    /// Load configuration from environment variables.
    ///
    /// Recognized variables:
    /// - IMMICH_API_KEY - API key for Immich
    /// - IMMICH_BASE_URL - Base URL of Immich instance
    pub fn from_env() -> Self {
        Config::default().with_env()
    }

    /// Merge environment variables into an existing config.
    pub fn with_env(mut self) -> Self {
        if let Ok(key) = std::env::var("IMMICH_API_KEY") {
            self.api.api_key = key;
        }
        if let Ok(url) = std::env::var("IMMICH_BASE_URL") {
            self.api.base_url = url;
        }
        if let Ok(dir) = std::env::var("MUSIC_DIR") {
            if !dir.is_empty() {
                self.music_dir = PathBuf::from(dir);
            }
        }
        self
    }

    /// Validate the entire configuration.
    pub fn validate(&self) -> Result<()> {
        // API validation
        if self.api.api_key.is_empty() {
            return Err(Error::Config("API key is required".to_string()));
        }
        if self.api.base_url.is_empty() {
            return Err(Error::Config("Base URL is required".to_string()));
        }

        // Delegate to sub-config validation
        self.processing.validate()?;
        self.video.validate()?;

        Ok(())
    }

    /// Save processing and video configuration to a TOML file.
    ///
    /// Only saves `processing` and `video` sections - API credentials are
    /// intentionally excluded as they should come from environment variables.
    pub fn save_to_file(&self, path: impl AsRef<std::path::Path>) -> Result<()> {
        let path = path.as_ref();
        let persistable = PersistableConfig::from(self);
        let content = toml::to_string_pretty(&persistable)
            .map_err(|e| Error::Config(format!("Failed to serialize config: {}", e)))?;

        // Write to a temp file then atomically rename to avoid partial writes
        let tmp_path = path.with_extension("toml.tmp");
        std::fs::write(&tmp_path, &content).map_err(|e| {
            if e.kind() == std::io::ErrorKind::PermissionDenied {
                Error::Config(format!(
                    "Permission denied writing config to '{}'. \
                    If running in Docker, mount a writable volume at the config directory \
                    (e.g. -v /host/config:/app/config).",
                    tmp_path.display()
                ))
            } else {
                Error::Io(e)
            }
        })?;
        std::fs::rename(&tmp_path, path).map_err(|e| {
            if e.kind() == std::io::ErrorKind::PermissionDenied {
                Error::Config(format!(
                    "Permission denied saving config to '{}'. \
                    If running in Docker, mount a writable volume at the config directory \
                    (e.g. -v /host/config:/app/config).",
                    path.display()
                ))
            } else {
                Error::Io(e)
            }
        })?;
        Ok(())
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.processing.output.size, 512);
        assert_eq!(config.processing.face_resolution.min_size, 80);
        assert_eq!(config.video.framerate, 20);
    }

    #[test]
    fn test_validation_missing_api_key() {
        let config = Config::default();
        let result = config.validate();
        assert!(result.is_err());
    }

    #[test]
    fn test_brightness_validation() {
        let mut config = BrightnessConfig {
            enabled: true,
            min_brightness: 0.1,
            max_brightness: 0.9,
        };

        // Valid config
        assert!(config.validate().is_ok());

        // Invalid: min >= max
        config.min_brightness = 0.9;
        config.max_brightness = 0.1;
        assert!(config.validate().is_err());

        // Invalid: out of range
        config.min_brightness = -0.1;
        config.max_brightness = 0.9;
        assert!(config.validate().is_err());

        // Disabled config skips validation
        config.enabled = false;
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_video_validation() {
        let mut config = VideoConfig::default();

        // Valid
        assert!(config.validate().is_ok());

        // Invalid CRF
        config.crf = 52;
        assert!(config.validate().is_err());

        // Invalid framerate
        config.crf = 23;
        config.framerate = 0;
        assert!(config.validate().is_err());

        // Min framerate above max framerate
        config.framerate = 20;
        config.min_framerate = 40.0;
        config.max_framerate = 30.0;
        assert!(config.validate().is_err());

        // Path traversal in music track name
        config.min_framerate = 5.0;
        config.music_tracks = vec!["../secret.mp3".to_string()];
        assert!(config.validate().is_err());
        config.music_tracks = vec!["song.mp3".to_string()];
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_output_validation() {
        let mut config = OutputConfig::default();

        // Valid
        assert!(config.validate().is_ok());

        // Too small
        config.size = 32;
        assert!(config.validate().is_err());

        // Too large
        config.size = 8192;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_output_dimensions() {
        let mut config = OutputConfig::default();
        assert_eq!(config.dimensions(), (512, 512));

        config.size = 1080;
        config.aspect_ratio = AspectRatio::Landscape16x9;
        assert_eq!(config.dimensions(), (1080, 608)); // 607.5 rounds to 608
        config.aspect_ratio = AspectRatio::Portrait9x16;
        assert_eq!(config.dimensions(), (608, 1080));
        config.aspect_ratio = AspectRatio::Portrait4x5;
        assert_eq!(config.dimensions(), (864, 1080));
        config.aspect_ratio = AspectRatio::Landscape5x4;
        assert_eq!(config.dimensions(), (1080, 864));

        // Odd sizes become even for the video encoders
        config.size = 513;
        config.aspect_ratio = AspectRatio::Square;
        assert_eq!(config.dimensions(), (512, 512));
        config.size = 100;
        config.aspect_ratio = AspectRatio::Landscape16x9;
        assert_eq!(config.dimensions(), (100, 56));
    }

    #[test]
    fn test_new_fields_default_when_missing() {
        let output: OutputConfig =
            serde_json::from_str(r#"{"size": 512, "keep_intermediates": false}"#).unwrap();
        assert_eq!(output.aspect_ratio, AspectRatio::Square);
        assert!(output.keep_frames && !output.keep_originals);
        let json = r#"{"size": 512, "aspect_ratio": "9:16", "keep_intermediates": false}"#;
        let output: OutputConfig = serde_json::from_str(json).unwrap();
        assert_eq!(output.aspect_ratio, AspectRatio::Portrait9x16);

        let json = r#"{"enabled": true, "position": "bottom_center",
            "year": true, "month": true, "day": false}"#;
        let ts: TimestampConfig = serde_json::from_str(json).unwrap();
        assert_eq!(ts.position, TextPosition::BottomCenter);
        assert_eq!(ts.font_size, None);
        assert!(ts.validate().is_ok());
    }

    #[test]
    fn test_font_size_validation() {
        let mut ts = TimestampConfig {
            font_size: Some(4.0),
            ..TimestampConfig::default()
        };
        assert!(ts.validate().is_ok());
        ts.font_size = Some(0.1);
        assert!(ts.validate().is_err());
        ts.font_size = Some(25.0);
        assert!(ts.validate().is_err());
    }

    #[test]
    fn test_save_and_load_config() {
        // Create a config with custom values
        let mut config = Config::default();
        config.processing.output.size = 256;
        config.processing.face_resolution.min_size = 100;
        config.video.framerate = 30;
        config.video.crf = 18;

        // Save to a temp file
        let temp_dir = std::env::temp_dir();
        let temp_path = temp_dir.join("test_config_new.toml");

        config
            .save_to_file(&temp_path)
            .expect("Failed to save config");

        // Verify file was created and contains expected content
        let content = std::fs::read_to_string(&temp_path).expect("Failed to read config");
        assert!(content.contains("size = 256"));
        assert!(content.contains("min_size = 100"));
        assert!(content.contains("framerate = 30"));
        assert!(content.contains("crf = 18"));

        // Verify API credentials are NOT in the file
        assert!(!content.contains("api_key"));
        assert!(!content.contains("base_url"));

        // Clean up
        let _ = std::fs::remove_file(&temp_path);
    }
}
