//! FFmpeg wrapper for video compilation.

use crate::config::VideoConfig;
use crate::error::{Error, Result};
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::Command;
use tokio_util::sync::CancellationToken;

/// Compile images into a timelapse video.
///
/// # Arguments
/// * `input_dir` - Directory containing JPEG images (sorted alphabetically for order)
/// * `output_path` - Path for the output video file
/// * `config` - Video configuration
/// * `music_dir` - Directory containing the music tracks referenced by `config.music_tracks`
/// * `cancel_token` - Optional cancellation token to kill the ffmpeg process
/// * `progress_callback` - Called with (current_frame, total_frames)
pub async fn compile_timelapse<F>(
    input_dir: &Path,
    output_path: &Path,
    config: &VideoConfig,
    music_dir: &Path,
    cancel_token: Option<&CancellationToken>,
    mut progress_callback: F,
) -> Result<()>
where
    F: FnMut(u32, u32),
{
    // Collect and sort image files for reliable ordering.
    // Filenames are timestamp-based, so alphabetical sort = chronological order.
    let mut image_files: Vec<_> = Vec::new();
    let mut entries = tokio::fs::read_dir(input_dir).await?;

    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();
        if path.extension().map(|e| e == "jpg").unwrap_or(false) {
            image_files.push(path);
        }
    }

    if image_files.is_empty() {
        return Err(Error::VideoCompilation(
            "No images found in input directory".to_string(),
        ));
    }

    // Sort alphabetically (timestamp-based names ensure chronological order)
    image_files.sort();
    let image_count = image_files.len() as u32;

    tracing::info!("Compiling {} images into video", image_count);

    // Resolve music tracks and their total length (if any were selected)
    let music = resolve_music(music_dir, &config.music_tracks).await?;
    let framerate = compute_framerate(image_count, music.total_secs, config);
    let audio = (!music.paths.is_empty()).then(|| AudioPlan {
        filter: build_audio_filter(
            music.paths.len(),
            image_count as f64 / framerate,
            music.total_secs,
            config.music_fade_out_secs as f64,
        ),
        inputs: music.paths.clone(),
    });

    if audio.is_some() {
        tracing::info!(
            "Video framerate: {:.3} fps, {} music track(s), {:.1}s of music",
            framerate,
            music.paths.len(),
            music.total_secs
        );
    } else {
        tracing::info!("Video framerate: {:.3} fps, no music", framerate);
    }

    // Create concat file for ffmpeg (more reliable than glob pattern).
    // Format: file '/path/to/image.jpg'\nduration 0.0667\n...
    let frame_duration = 1.0 / framerate;
    let concat_path = input_dir.join(".concat.txt");

    let mut concat_content = String::new();
    for (i, path) in image_files.iter().enumerate() {
        // Use just the filename since concat file is in the same directory as images
        let filename = path
            .file_name()
            .map(|f| f.to_string_lossy())
            .unwrap_or_default()
            .replace('\'', "'\\''");
        concat_content.push_str(&format!("file '{}'\n", filename));

        // Don't add duration after the last frame
        if i < image_files.len() - 1 {
            concat_content.push_str(&format!("duration {:.6}\n", frame_duration));
        }
    }

    // Write concat file
    let mut file = tokio::fs::File::create(&concat_path).await?;
    file.write_all(concat_content.as_bytes()).await?;
    file.flush().await?;

    // Use a closure to ensure concat file is cleaned up on all paths
    let result = run_ffmpeg(
        &concat_path,
        output_path,
        config,
        framerate,
        audio.as_ref(),
        cancel_token,
        image_count,
        &mut progress_callback,
    )
    .await;

    // Always clean up concat file, even on error
    let _ = tokio::fs::remove_file(&concat_path).await;

    result
}

/// Audio inputs and the filter graph that mixes them into the video.
struct AudioPlan {
    inputs: Vec<PathBuf>,
    filter: String,
}

/// Music tracks resolved to paths, with their total duration.
struct ResolvedMusic {
    paths: Vec<PathBuf>,
    total_secs: f64,
}

/// Resolve the selected track names inside `music_dir` and probe their length.
async fn resolve_music(music_dir: &Path, tracks: &[String]) -> Result<ResolvedMusic> {
    let mut paths = Vec::with_capacity(tracks.len());
    let mut total_secs = 0.0;
    for name in tracks {
        if name.is_empty() || name.contains("..") || name.contains('/') || name.contains('\\') {
            return Err(Error::VideoCompilation(format!(
                "Invalid music track name '{}'",
                name
            )));
        }
        let path = music_dir.join(name);
        if tokio::fs::metadata(&path).await.is_err() {
            return Err(Error::VideoCompilation(format!(
                "Music track '{}' not found in {}",
                name,
                music_dir.display()
            )));
        }
        total_secs += probe_duration(&path).await?;
        paths.push(path);
    }
    Ok(ResolvedMusic { paths, total_secs })
}

/// Framerate for the video.
///
/// With "fit to music" enabled and music present, the framerate is chosen so the
/// video lasts as long as the music (`images / music length`), clamped to the
/// configured minimum and maximum. Otherwise the fixed framerate is used.
pub fn compute_framerate(image_count: u32, music_secs: f64, config: &VideoConfig) -> f64 {
    if config.fit_to_music && music_secs > 0.0 && image_count > 0 {
        let min = config.min_framerate as f64;
        let max = (config.max_framerate as f64).max(min);
        (image_count as f64 / music_secs).clamp(min, max)
    } else {
        config.framerate as f64
    }
}

/// Build the `-filter_complex` graph for the music.
///
/// Audio inputs start at ffmpeg input index 1 (index 0 is the image sequence).
/// Each track is normalized to 48 kHz stereo, the tracks are concatenated in
/// order, cut to the video length and faded out at the end.
pub fn build_audio_filter(
    track_count: usize,
    video_secs: f64,
    music_secs: f64,
    fade_secs: f64,
) -> String {
    let mut filter = String::new();
    for i in 0..track_count {
        filter.push_str(&format!(
            "[{}:a]aresample=48000,aformat=sample_fmts=fltp:channel_layouts=stereo[a{}];",
            i + 1,
            i
        ));
    }
    for i in 0..track_count {
        filter.push_str(&format!("[a{}]", i));
    }
    let end = video_secs.min(music_secs).max(0.0);
    let fade = fade_secs.clamp(0.0, end);
    filter.push_str(&format!(
        "concat=n={}:v=0:a=1,atrim=0:{:.3},asetpts=PTS-STARTPTS",
        track_count, end
    ));
    if fade > 0.0 {
        filter.push_str(&format!(",afade=t=out:st={:.3}:d={:.3}", end - fade, fade));
    }
    filter.push_str("[aout]");
    filter
}

/// Run the ffmpeg process and handle progress/cancellation.
#[allow(clippy::too_many_arguments)]
async fn run_ffmpeg<F>(
    concat_path: &Path,
    output_path: &Path,
    config: &VideoConfig,
    framerate: f64,
    audio: Option<&AudioPlan>,
    cancel_token: Option<&CancellationToken>,
    image_count: u32,
    progress_callback: &mut F,
) -> Result<()>
where
    F: FnMut(u32, u32),
{
    // Build ffmpeg command using concat demuxer
    let mut cmd = Command::new("ffmpeg");
    cmd.arg("-y") // Overwrite output
        .arg("-f")
        .arg("concat")
        .arg("-safe")
        .arg("0") // Allow absolute paths
        .arg("-i")
        .arg(concat_path);

    if let Some(audio) = audio {
        for input in &audio.inputs {
            cmd.arg("-i").arg(input);
        }
        cmd.arg("-filter_complex")
            .arg(&audio.filter)
            .arg("-map")
            .arg("0:v")
            .arg("-map")
            .arg("[aout]")
            .arg("-c:a")
            .arg("aac")
            .arg("-b:a")
            .arg("192k");
    }

    cmd.arg("-r")
        .arg(format!("{:.4}", framerate))
        .arg("-c:v")
        .arg(&config.codec)
        .arg("-crf")
        .arg(config.crf.to_string())
        .arg("-pix_fmt")
        .arg("yuv420p")
        .arg("-progress")
        .arg("pipe:1")
        .arg(output_path)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut child = cmd
        .spawn()
        .map_err(|e| Error::FFmpeg(format!("Failed to spawn ffmpeg (is it installed?): {}", e)))?;

    // Capture stdout for progress and stderr for errors
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| Error::FFmpeg("Failed to capture ffmpeg stdout".to_string()))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| Error::FFmpeg("Failed to capture ffmpeg stderr".to_string()))?;

    // Read stderr in background to avoid blocking
    let stderr_handle = tokio::spawn(async move {
        let mut stderr_reader = BufReader::new(stderr).lines();
        let mut stderr_output = String::new();
        while let Ok(Some(line)) = stderr_reader.next_line().await {
            stderr_output.push_str(&line);
            stderr_output.push('\n');
        }
        stderr_output
    });

    // Parse progress from stdout, with cancellation support
    let mut reader = BufReader::new(stdout).lines();

    // Create a dummy token for when no cancel_token is provided
    let owned_token = CancellationToken::new();
    let token = cancel_token.unwrap_or(&owned_token);

    loop {
        tokio::select! {
            _ = token.cancelled() => {
                // Kill the ffmpeg process on cancellation
                let _ = child.kill().await;
                return Err(Error::Cancelled);
            }
            line = reader.next_line() => {
                match line? {
                    Some(line) => {
                        if line.starts_with("frame=") {
                            if let Some(frame_str) = line.strip_prefix("frame=") {
                                if let Ok(frame) = frame_str.trim().parse::<u32>() {
                                    let clamped_frame = frame.min(image_count);
                                    progress_callback(clamped_frame, image_count);
                                }
                            }
                        }
                    }
                    None => break, // EOF
                }
            }
        }
    }

    let status = child.wait().await?;
    let stderr_output = stderr_handle.await.unwrap_or_default();

    if !status.success() {
        // Get last few lines of stderr for the error message
        let error_lines: Vec<&str> = stderr_output.lines().rev().take(10).collect();
        let error_detail = error_lines.into_iter().rev().collect::<Vec<_>>().join("\n");

        return Err(Error::FFmpeg(format!(
            "ffmpeg exited with status {}\n{}",
            status, error_detail
        )));
    }

    progress_callback(image_count, image_count);
    tracing::info!("Video compilation complete: {:?}", output_path);

    Ok(())
}

/// Check if ffmpeg is available.
pub async fn check_ffmpeg() -> Result<String> {
    let output = Command::new("ffmpeg")
        .arg("-version")
        .output()
        .await
        .map_err(|e| Error::FFmpeg(format!("ffmpeg not found: {}", e)))?;

    if !output.status.success() {
        return Err(Error::FFmpeg("ffmpeg version check failed".to_string()));
    }

    let version_output = String::from_utf8_lossy(&output.stdout);
    let version = version_output
        .lines()
        .next()
        .unwrap_or("unknown")
        .to_string();

    Ok(version)
}

/// File extensions recognized as music tracks.
pub const AUDIO_EXTENSIONS: &[&str] = &["mp3", "m4a", "aac", "wav", "flac", "ogg", "opus"];

/// A music file available for mixing into the video.
#[derive(Debug, Clone, Serialize)]
pub struct MusicTrack {
    /// File name inside the music directory.
    pub name: String,
    /// Track length in seconds (None if ffprobe could not read it).
    pub duration_secs: Option<f64>,
}

/// Get the duration of a media file in seconds using ffprobe.
pub async fn probe_duration(path: &Path) -> Result<f64> {
    let output = Command::new("ffprobe")
        .arg("-v")
        .arg("error")
        .arg("-show_entries")
        .arg("format=duration")
        .arg("-of")
        .arg("default=noprint_wrappers=1:nokey=1")
        .arg(path)
        .output()
        .await
        .map_err(|e| Error::FFmpeg(format!("Failed to run ffprobe: {}", e)))?;

    if !output.status.success() {
        return Err(Error::FFmpeg(format!(
            "ffprobe failed for {}: {}",
            path.display(),
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }

    String::from_utf8_lossy(&output.stdout)
        .trim()
        .parse::<f64>()
        .map_err(|_| Error::FFmpeg(format!("Could not read duration of {}", path.display())))
}

/// List music files in `music_dir` (non-recursive), sorted by name.
/// Returns an empty list if the directory does not exist.
pub async fn list_music(music_dir: &Path) -> Result<Vec<MusicTrack>> {
    let mut tracks = Vec::new();
    let mut entries = match tokio::fs::read_dir(music_dir).await {
        Ok(entries) => entries,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(tracks),
        Err(e) => return Err(e.into()),
    };

    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();
        let is_audio = path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| AUDIO_EXTENSIONS.contains(&e.to_ascii_lowercase().as_str()))
            .unwrap_or(false);
        let is_file = entry
            .file_type()
            .await
            .map(|t| t.is_file())
            .unwrap_or(false);
        if !is_audio || !is_file {
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_string();
        let duration_secs = probe_duration(&path).await.ok();
        tracks.push(MusicTrack {
            name,
            duration_secs,
        });
    }

    tracks.sort_by_key(|t| t.name.to_lowercase());
    Ok(tracks)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn framerate_fixed_without_fit() {
        let config = VideoConfig::default();
        assert_eq!(compute_framerate(300, 60.0, &config), 20.0);
    }

    #[test]
    fn framerate_fits_music_within_bounds() {
        let config = VideoConfig {
            fit_to_music: true,
            min_framerate: 5.0,
            max_framerate: 30.0,
            ..VideoConfig::default()
        };
        // 600 images over 60s of music = 10 fps
        assert!((compute_framerate(600, 60.0, &config) - 10.0).abs() < 1e-9);
        // 6000 images over 60s = 100 fps -> clamped to max
        assert_eq!(compute_framerate(6000, 60.0, &config), 30.0);
        // 60 images over 60s = 1 fps -> clamped to min
        assert_eq!(compute_framerate(60, 60.0, &config), 5.0);
        // no music -> fixed framerate
        assert_eq!(compute_framerate(600, 0.0, &config), 20.0);
    }

    #[test]
    fn audio_filter_single_track_fades_at_video_end() {
        let f = build_audio_filter(1, 30.0, 120.0, 3.0);
        assert!(f.starts_with("[1:a]aresample=48000"));
        assert!(f.contains("concat=n=1:v=0:a=1,atrim=0:30.000"));
        assert!(f.contains("afade=t=out:st=27.000:d=3.000"));
        assert!(f.ends_with("[aout]"));
    }

    #[test]
    fn audio_filter_multiple_tracks_short_music() {
        // Music shorter than the video: fade at the end of the music
        let f = build_audio_filter(2, 100.0, 50.0, 5.0);
        assert!(f.contains("[2:a]aresample"));
        assert!(f.contains("[a0][a1]concat=n=2"));
        assert!(f.contains("atrim=0:50.000"));
        assert!(f.contains("afade=t=out:st=45.000:d=5.000"));
    }

    #[test]
    fn audio_filter_no_fade() {
        let f = build_audio_filter(1, 10.0, 20.0, 0.0);
        assert!(!f.contains("afade"));
    }
}
