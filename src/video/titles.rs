//! Title cards: an animated intro and outro around the timelapse.
//!
//! A card is a background (an Immich photo, an uploaded image or a plain
//! colour) with a slow zoom, a darkening layer and up to three text blocks
//! (title, subtitle, longer text) that fade in one after the other. The card
//! fades in from and out to black. Each card becomes one ffmpeg input plus a
//! filter chain that ends in a labelled stream, ready for the `concat` filter.

use crate::config::{hex_color, plain_file_name, ApiConfig, TitleCardConfig};
use crate::error::{Error, Result};
use crate::immich_api::ImmichClient;
use serde::Serialize;
use std::collections::HashSet;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use tokio::process::Command;

/// Folder for uploaded title images (persistent, inside the config mount).
pub const TITLE_IMAGES_DIR: &str = "config/title-images";

/// File extensions accepted as title images.
pub const TITLE_IMAGE_EXTENSIONS: &[&str] = &["jpg", "jpeg", "png", "webp"];

/// Folders searched for fonts (recursively).
const FONT_DIRS: &[&str] = &["/usr/share/fonts", "fonts", "config/fonts"];

/// Font used when the configured one is missing.
const DEFAULT_FONT: &str = "DejaVuSans-Bold.ttf";

/// How far the background zooms in over the length of the card.
const ZOOM_AMOUNT: f64 = 0.12;

/// Zoom held during a camera pan: leaves room to move the frame.
const PAN_ZOOM: f64 = 1.2;

/// Extra zoom added over the length of a pan, so it doesn't look flat.
const PAN_ZOOM_GROWTH: f64 = 0.04;

/// Pans `random` picks from.
const PANS: &[&str] = &["pan_right", "pan_left", "pan_down", "pan_up"];

/// `auto`: zoom at the start and at the end (on the face).
const AUTO_ZOOM_START: f64 = 1.15;
const AUTO_ZOOM_END: f64 = 1.45;
/// `auto` without a face: zoom at the end of the pan.
const AUTO_ZOOM_END_NO_FACE: f64 = 1.3;

/// What `auto` needs to know about the background.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Shot {
    /// The photo is higher than wide (after EXIF rotation).
    pub portrait: bool,
    /// Middle of the largest face, 0..1 in the prepared background.
    pub focus: Option<(f64, f64)>,
}

/// Longest edge of a background prepared for zooming.
const MAX_ZOOM_EDGE: u32 = 3840;

/// A font that can be chosen for the titles.
#[derive(Debug, Clone, Serialize)]
pub struct FontInfo {
    /// File name, stored in the config.
    pub file: String,
    /// Readable name, e.g. "DejaVu Sans Bold".
    pub name: String,
}

fn collect_fonts(dir: &Path, depth: u32, out: &mut Vec<PathBuf>) {
    if depth > 6 {
        return;
    }
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_fonts(&path, depth + 1, out);
        } else if path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| matches!(e.to_ascii_lowercase().as_str(), "ttf" | "otf"))
            .unwrap_or(false)
        {
            out.push(path);
        }
    }
}

fn font_files() -> Vec<PathBuf> {
    let mut out = Vec::new();
    for dir in FONT_DIRS {
        collect_fonts(Path::new(dir), 0, &mut out);
    }
    out
}

/// "DejaVuSans-BoldOblique.ttf" → "DejaVu Sans Bold Oblique".
pub fn font_display_name(file: &str) -> String {
    let stem = file.rsplit_once('.').map(|(s, _)| s).unwrap_or(file);
    let mut out = String::new();
    let mut prev_lower = false;
    for c in stem.chars() {
        if c == '-' || c == '_' || c == ' ' {
            if !out.ends_with(' ') {
                out.push(' ');
            }
            prev_lower = false;
            continue;
        }
        if c.is_uppercase() && prev_lower {
            out.push(' ');
        }
        prev_lower = c.is_lowercase() || c.is_ascii_digit();
        out.push(c);
    }
    out.replace("Deja Vu", "DejaVu").trim().to_string()
}

/// All fonts found, sorted by name (each file name once).
pub fn list_fonts() -> Vec<FontInfo> {
    let mut seen = HashSet::new();
    let mut fonts: Vec<FontInfo> = font_files()
        .into_iter()
        .filter_map(|p| p.file_name().map(|f| f.to_string_lossy().to_string()))
        .filter(|f| plain_file_name(f) && seen.insert(f.clone()))
        .map(|file| FontInfo {
            name: font_display_name(&file),
            file,
        })
        .collect();
    fonts.sort_by_key(|f| f.name.to_lowercase());
    fonts
}

/// Path of a font by file name.
pub fn find_font(file: &str) -> Option<PathBuf> {
    if !plain_file_name(file) {
        return None;
    }
    font_files()
        .into_iter()
        .find(|p| p.file_name().map(|f| f == file).unwrap_or(false))
}

/// Font for a card: the configured one, else the default, else any font.
fn resolve_font(file: &str) -> Result<PathBuf> {
    find_font(file)
        .or_else(|| find_font(DEFAULT_FONT))
        .or_else(|| font_files().into_iter().next())
        .ok_or_else(|| Error::VideoCompilation("No font found for the titles".to_string()))
}

/// Whether a file name is an accepted title image.
pub fn is_title_image(name: &str) -> bool {
    plain_file_name(name)
        && name
            .rsplit_once('.')
            .map(|(_, ext)| TITLE_IMAGE_EXTENSIONS.contains(&ext.to_ascii_lowercase().as_str()))
            .unwrap_or(false)
}

/// Path of an uploaded title image.
pub fn title_image_path(name: &str) -> Option<PathBuf> {
    is_title_image(name).then(|| Path::new(TITLE_IMAGES_DIR).join(name))
}

/// Uploaded title images, sorted by name.
pub async fn list_title_images() -> Result<Vec<String>> {
    let mut names = Vec::new();
    let mut entries = match tokio::fs::read_dir(TITLE_IMAGES_DIR).await {
        Ok(entries) => entries,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(names),
        Err(e) => return Err(e.into()),
    };
    while let Some(entry) = entries.next_entry().await? {
        let name = entry.file_name().to_string_lossy().to_string();
        let is_file = entry.file_type().await.map(|t| t.is_file()).unwrap_or(false);
        if is_file && is_title_image(&name) {
            names.push(name);
        }
    }
    names.sort_by_key(|n| n.to_lowercase());
    Ok(names)
}

/// A temporary folder that is removed when dropped.
pub struct WorkDir(PathBuf);

impl WorkDir {
    pub fn new() -> Result<Self> {
        let path = std::env::temp_dir().join(format!("timelapse-titles-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&path)?;
        Ok(Self(path))
    }

    pub fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for WorkDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// Raw bytes of the card's background image, if it has one.
async fn load_background(
    card: &TitleCardConfig,
    api: &ApiConfig,
) -> Result<Option<(Vec<u8>, Option<(f64, f64)>)>> {
    if let Some(id) = card.asset_id.as_deref().filter(|s| !s.is_empty()) {
        let client = ImmichClient::new(api)?;
        let bytes = client.download_asset_preview(id).await?.to_vec();
        // The face only steers the movement: without it the card still works
        let face = if card.zoom && card.motion == "auto" {
            match client.get_faces(id).await {
                Ok(faces) => largest_face(&faces),
                Err(e) => {
                    tracing::warn!("Title image: faces not readable: {e}");
                    None
                }
            }
        } else {
            None
        };
        return Ok(Some((bytes, face)));
    }
    if let Some(name) = card.upload.as_deref().filter(|s| !s.is_empty()) {
        let path = title_image_path(name)
            .ok_or_else(|| Error::VideoCompilation(format!("Invalid title image '{}'", name)))?;
        let bytes = tokio::fs::read(&path).await.map_err(|e| {
            Error::VideoCompilation(format!("Title image '{}' not readable: {}", name, e))
        })?;
        return Ok(Some((bytes, None)));
    }
    Ok(None)
}

/// Middle of the largest face, 0..1 in the photo.
fn largest_face(faces: &[crate::immich_api::AssetFace]) -> Option<(f64, f64)> {
    faces
        .iter()
        .map(|f| &f.data)
        .filter(|d| d.image_width > 0 && d.image_height > 0)
        .max_by(|a, b| {
            let area = |d: &crate::immich_api::FaceData| {
                ((d.bounding_box_x2 - d.bounding_box_x1) * (d.bounding_box_y2 - d.bounding_box_y1)).abs()
            };
            area(a).total_cmp(&area(b))
        })
        .map(|d| {
            (
                ((d.bounding_box_x1 + d.bounding_box_x2) as f64 / 2.0 / d.image_width as f64).clamp(0.0, 1.0),
                ((d.bounding_box_y1 + d.bounding_box_y2) as f64 / 2.0 / d.image_height as f64).clamp(0.0, 1.0),
            )
        })
}

/// A point (0..1) of a `ow`x`oh` photo in the `bw`x`bh` background that
/// `resize_to_fill` makes from it (scaled, then cut in the middle).
fn crop_focus(p: (f64, f64), ow: u32, oh: u32, bw: u32, bh: u32) -> (f64, f64) {
    let (ow, oh, bw, bh) = (ow.max(1) as f64, oh.max(1) as f64, bw.max(1) as f64, bh.max(1) as f64);
    let scale = (bw / ow).max(bh / oh);
    let (sw, sh) = (ow * scale, oh * scale);
    (
        ((p.0 * sw - (sw - bw) / 2.0) / bw).clamp(0.0, 1.0),
        ((p.1 * sh - (sh - bh) / 2.0) / bh).clamp(0.0, 1.0),
    )
}

/// zoompan expressions `(z, x, y)` for the card's movement. `seed` picks
/// the pan for `random` (and the direction of `auto` without a face).
fn motion(card: &TitleCardConfig, shot: Shot, frames: u32, seed: u32) -> (String, String, String) {
    let cx = "iw/2-(iw/zoom/2)".to_string();
    let cy = "ih/2-(ih/zoom/2)".to_string();
    if !card.zoom {
        return ("1".to_string(), cx, cy);
    }
    let kind = match card.motion.as_str() {
        "random" => PANS[seed as usize % PANS.len()],
        other => other,
    };
    let p = format!("on/{frames}");
    if kind == "auto" {
        return auto_motion(shot, &p, seed);
    }
    let zp = format!("{PAN_ZOOM}+{PAN_ZOOM_GROWTH}*{p}");
    match kind {
        "pan_right" => (zp, format!("(iw-iw/zoom)*{p}"), cy),
        "pan_left" => (zp, format!("(iw-iw/zoom)*(1-{p})"), cy),
        "pan_down" => (zp, cx, format!("(ih-ih/zoom)*{p}")),
        "pan_up" => (zp, cx, format!("(ih-ih/zoom)*(1-{p})")),
        _ => (format!("1+{ZOOM_AMOUNT}*{p}"), cx, cy),
    }
}

/// `auto`: landscape pans sideways, portrait from the top down; with a face
/// the pan ends on it and zooms in. Eased at both ends.
fn auto_motion(shot: Shot, p: &str, seed: u32) -> (String, String, String) {
    let e = format!("({p})*({p})*(3-2*({p}))");
    // Start and end of the middle of the frame (0..1)
    let ((sx, sy), (ex, ey), z1) = match (shot.focus, shot.portrait) {
        (Some((fx, fy)), false) => ((if fx >= 0.5 { 0.0 } else { 1.0 }, 0.5), (fx, fy), AUTO_ZOOM_END),
        (Some((fx, fy)), true) => ((0.5, 0.0), (fx, fy), AUTO_ZOOM_END),
        (None, false) if seed % 2 == 1 => ((1.0, 0.5), (0.0, 0.5), AUTO_ZOOM_END_NO_FACE),
        (None, false) => ((0.0, 0.5), (1.0, 0.5), AUTO_ZOOM_END_NO_FACE),
        (None, true) => ((0.5, 0.0), (0.5, 1.0), AUTO_ZOOM_END_NO_FACE),
    };
    let z0 = AUTO_ZOOM_START;
    let z = format!("{z0}+{:.4}*{e}", z1 - z0);
    let along = |s: f64, t: f64, size: &str| {
        format!("clip(({s:.4}+{:.4}*{e})*{size}-{size}/zoom/2,0,{size}-{size}/zoom)", t - s)
    };
    (z, along(sx, ex, "iw"), along(sy, ey, "ih"))
}

/// Size the background is prepared at: twice the frame (for a smooth zoom),
/// with the long edge capped; the frame size without zoom.
fn background_size(w: u32, h: u32, zoom: bool) -> (u32, u32) {
    if !zoom {
        return (w, h);
    }
    let scale = (MAX_ZOOM_EDGE as f64 / w.max(h) as f64).clamp(1.0, 2.0);
    let even = |v: f64| ((v.round() as u32) & !1).max(2);
    (even(w as f64 * scale), even(h as f64 * scale))
}

/// Load, rotate (EXIF) and crop the background to the frame's aspect ratio.
/// Writes `dest` (JPEG) and returns the shot, or `None` without background.
pub async fn prepare_background(
    card: &TitleCardConfig,
    api: &ApiConfig,
    w: u32,
    h: u32,
    dest: &Path,
) -> Result<Option<Shot>> {
    let Some((bytes, face)) = load_background(card, api).await? else {
        return Ok(None);
    };
    let (bw, bh) = background_size(w, h, card.zoom);
    let dest = dest.to_path_buf();
    let shot = tokio::task::spawn_blocking(move || -> Result<Shot> {
        use image::ImageDecoder;
        let img_err = |e: image::ImageError| Error::ImageProcessing(format!("Title image: {}", e));
        let mut decoder = image::ImageReader::new(Cursor::new(bytes))
            .with_guessed_format()?
            .into_decoder()
            .map_err(img_err)?;
        let orientation = decoder.orientation().map_err(img_err)?;
        let mut img = image::DynamicImage::from_decoder(decoder).map_err(img_err)?;
        img.apply_orientation(orientation);
        let (ow, oh) = (img.width(), img.height());
        let shot = Shot {
            portrait: oh > ow,
            focus: face.map(|f| crop_focus(f, ow, oh, bw, bh)),
        };
        let img = img
            .resize_to_fill(bw, bh, image::imageops::FilterType::Lanczos3)
            .to_rgb8();
        img.save_with_format(&dest, image::ImageFormat::Jpeg).map_err(img_err)?;
        Ok(shot)
    })
    .await
    .map_err(|e| Error::ImageProcessing(format!("Title image task failed: {}", e)))??;
    Ok(Some(shot))
}

/// One card as ffmpeg input plus filter chain.
#[derive(Debug, Clone)]
pub struct CardInput {
    /// Arguments that add the card's input (`-i …` or `-f lavfi -i …`).
    pub args: Vec<String>,
    /// Filter chain from `[idx:v]` to `[label]`.
    pub filter: String,
    /// Length in seconds (a whole number of frames).
    pub duration: f64,
}

/// Timing of the animation for a card of `d` seconds:
/// (fade to/from black, delay between text blocks, text fade-in, first text start).
fn timing(d: f64) -> (f64, f64, f64, f64) {
    let fade = (d * 0.2).min(0.8);
    let stagger = (d * 0.08).min(0.4);
    let ramp = (d * 0.15).min(0.6);
    let start = fade * 0.5;
    (fade, stagger, ramp, start)
}

/// Moment of the still preview: all text visible, before the fade-out.
pub fn preview_time(d: f64) -> f64 {
    let (fade, ..) = timing(d);
    (d - fade - 0.05).max(0.0)
}

/// Break a paragraph into lines of at most `max_chars` characters (by words).
pub fn wrap_text(text: &str, max_chars: usize) -> Vec<String> {
    let max_chars = max_chars.max(1);
    let mut lines = Vec::new();
    let mut line = String::new();
    for word in text.split_whitespace() {
        let len = line.chars().count();
        if len > 0 && len + 1 + word.chars().count() > max_chars {
            lines.push(std::mem::take(&mut line));
        }
        if !line.is_empty() {
            line.push(' ');
        }
        line.push_str(word);
    }
    if !line.is_empty() {
        lines.push(line);
    }
    lines
}

/// A path that can go into the filter graph between single quotes.
fn filter_path(path: &Path) -> Result<String> {
    let s = path.to_string_lossy().replace('\\', "/");
    if s.contains('\'') || s.contains(':') || s.contains(',') || s.contains(';') {
        return Err(Error::VideoCompilation(format!(
            "Path not usable in the filter: {}",
            s
        )));
    }
    Ok(s)
}

/// Build one card. `background` is the prepared image (None = plain colour).
/// Text lines are written as files into `work`; `tag` keeps them apart.
#[allow(clippy::too_many_arguments)]
pub fn build_card(
    card: &TitleCardConfig,
    background: Option<(&Path, Shot)>,
    w: u32,
    h: u32,
    fps: u32,
    work: &Path,
    tag: &str,
    input_idx: usize,
    out_label: &str,
) -> Result<CardInput> {
    let fps = fps.max(1);
    let frames = ((card.duration_secs as f64 * fps as f64).round() as u32).max(1);
    let d = frames as f64 / fps as f64;
    let (fade, stagger, ramp, start) = timing(d);
    let color = hex_color(&card.color).unwrap_or("ffffff");

    // Input and background
    let (args, mut filter) = match background {
        Some((bg, shot)) => {
            let seed = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.subsec_micros())
                .unwrap_or(0);
            let (z, x, y) = motion(card, shot, frames, seed);
            let mut f = format!(
                "[{idx}:v]zoompan=z='{z}':x='{x}':y='{y}':d={frames}:s={w}x{h}:fps={fps},setsar=1,format=yuv420p",
                idx = input_idx
            );
            let dim = card.dim.clamp(0.0, 0.9);
            if dim > 0.0 {
                f.push_str(&format!(",drawbox=x=0:y=0:w=iw:h=ih:color=black@{:.2}:t=fill", dim));
            }
            (vec!["-i".to_string(), bg.to_string_lossy().to_string()], f)
        }
        None => {
            let bg = hex_color(&card.background).unwrap_or("000000");
            (
                vec![
                    "-f".to_string(),
                    "lavfi".to_string(),
                    "-i".to_string(),
                    format!("color=c=0x{bg}:s={w}x{h}:r={fps}:d={d:.3}"),
                ],
                format!("[{}:v]setsar=1,format=yuv420p", input_idx),
            )
        }
    };

    // Text: title, subtitle, longer text – each block fades in after the previous one
    let has_text = [&card.title, &card.subtitle, &card.text]
        .iter()
        .any(|s| !s.trim().is_empty());
    if has_text {
        let font = filter_path(&resolve_font(&card.font)?)?;
        let big = (h as f64 * card.size as f64 / 100.0).max(8.0);
        let sizes = [big, (big * 0.5).max(8.0), (big * 0.34).max(8.0)];
        // Subtitle and text take the title colour unless they have their own
        let colors = [
            color,
            hex_color(&card.subtitle_color).unwrap_or(color),
            hex_color(&card.text_color).unwrap_or(color),
        ];
        let max_width = w as f64 * 0.86;

        // (lines, font size) per block; the longer text keeps its line breaks
        let mut blocks: Vec<(Vec<String>, f64)> = Vec::new();
        for (i, text) in [&card.title, &card.subtitle, &card.text].iter().enumerate() {
            let size = sizes[i];
            let max_chars = (max_width / (size * 0.56)).floor().max(8.0) as usize;
            let mut lines = Vec::new();
            if !text.trim().is_empty() {
                for para in text.trim().lines() {
                    if para.trim().is_empty() {
                        lines.push(String::new());
                    } else {
                        lines.extend(wrap_text(para, max_chars));
                    }
                }
            }
            blocks.push((lines, size));
        }

        let gap = big * 0.45;
        let line_h = |s: f64| s * 1.35;
        let used: Vec<&(Vec<String>, f64)> = blocks.iter().filter(|(l, _)| !l.is_empty()).collect();
        let total: f64 = used.iter().map(|(l, s)| l.len() as f64 * line_h(*s)).sum::<f64>()
            + gap * (used.len().saturating_sub(1)) as f64;
        let mut y = ((h as f64 - total) / 2.0).max(0.0);

        let mut n = 0;
        for (block, (lines, size)) in blocks.iter().enumerate() {
            if lines.is_empty() {
                continue;
            }
            let t0 = start + stagger * block as f64;
            let color = colors[block];
            let shadow = (size / 18.0).round().max(1.0) as u32;
            for line in lines {
                if !line.is_empty() {
                    let file = work.join(format!("{}-{}.txt", tag, n));
                    std::fs::write(&file, line.as_bytes())?;
                    n += 1;
                    filter.push_str(&format!(
                        ",drawtext=fontfile='{font}':textfile='{tf}':expansion=none:fontsize={fs}:fontcolor=0x{color}:x=(w-text_w)/2:y={y}:shadowcolor=black@0.55:shadowx={shadow}:shadowy={shadow}:alpha='min(1,max(0,(t-{t0:.3})/{ramp:.3}))'",
                        tf = filter_path(&file)?,
                        fs = size.round() as u32,
                        y = y.round() as i64,
                    ));
                }
                y += line_h(*size);
            }
            y += gap;
        }
    }

    filter.push_str(&format!(
        ",trim=duration={d:.3},setpts=PTS-STARTPTS,fade=t=in:st=0:d={fade:.3},fade=t=out:st={st:.3}:d={fade:.3}[{out_label}]",
        st = (d - fade).max(0.0)
    ));

    Ok(CardInput {
        args,
        filter,
        duration: d,
    })
}

/// Prepare a card completely: background into `work`, then the ffmpeg input.
#[allow(clippy::too_many_arguments)]
pub async fn plan_card(
    card: &TitleCardConfig,
    api: &ApiConfig,
    w: u32,
    h: u32,
    fps: u32,
    work: &Path,
    tag: &str,
    input_idx: usize,
    out_label: &str,
) -> Result<CardInput> {
    let bg_path = work.join(format!("{}-bg.jpg", tag));
    let shot = prepare_background(card, api, w, h, &bg_path).await?;
    build_card(
        card,
        shot.map(|s| (bg_path.as_path(), s)),
        w,
        h,
        fps,
        work,
        tag,
        input_idx,
        out_label,
    )
}

/// Render a single JPEG of the card (all text visible) for the editor.
/// "03.05.2018 – 12.11.2026" from the first and last frame (file names start
/// with the photo's date, `2018-05-03T…`); one date if both are the same day.
pub fn date_range(frames: &[std::path::PathBuf]) -> Option<String> {
    let date = |p: &std::path::PathBuf| -> Option<String> {
        let name = p.file_name()?.to_string_lossy().to_string();
        let d = name.get(..10)?;
        let ok = d.char_indices().all(|(i, c)| if i == 4 || i == 7 { c == '-' } else { c.is_ascii_digit() });
        ok.then(|| format!("{}.{}.{}", &d[8..10], &d[5..7], &d[..4]))
    };
    let first = date(frames.first()?)?;
    let last = date(frames.last()?)?;
    Some(if first == last { first } else { format!("{} – {}", first, last) })
}

/// The card with its date line added below the longer text, if it wants one.
pub fn with_dates(card: &TitleCardConfig, range: Option<&str>) -> TitleCardConfig {
    let mut card = card.clone();
    if card.dates {
        if let Some(range) = range {
            let text = card.text.trim_end();
            card.text = if text.is_empty() { range.to_string() } else { format!("{}\n\n{}", text, range) };
        }
    }
    card
}

pub async fn render_preview(card: &TitleCardConfig, api: &ApiConfig, w: u32, h: u32) -> Result<Vec<u8>> {
    let work = WorkDir::new()?;
    let fps = 25;
    // The real dates are known only when the video is made
    let card = &with_dates(card, Some("dd.mm.yyyy – dd.mm.yyyy"));
    let plan = plan_card(card, api, w, h, fps, work.path(), "preview", 0, "card").await?;
    let at = preview_time(plan.duration);

    let output = Command::new("ffmpeg")
        .arg("-v")
        .arg("error")
        .args(&plan.args)
        .arg("-filter_complex")
        .arg(&plan.filter)
        .arg("-map")
        .arg("[card]")
        .arg("-ss")
        .arg(format!("{:.3}", at))
        .arg("-frames:v")
        .arg("1")
        .arg("-q:v")
        .arg("3")
        .arg("-f")
        .arg("image2pipe")
        .arg("-c:v")
        .arg("mjpeg")
        .arg("pipe:1")
        .stdin(Stdio::null())
        .output()
        .await
        .map_err(|e| Error::FFmpeg(format!("Failed to run ffmpeg: {}", e)))?;

    if !output.status.success() || output.stdout.is_empty() {
        return Err(Error::FFmpeg(format!(
            "Title preview failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }
    Ok(output.stdout)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn font_names_are_readable() {
        assert_eq!(font_display_name("DejaVuSans-Bold.ttf"), "DejaVu Sans Bold");
        assert_eq!(font_display_name("DejaVuSerif.ttf"), "DejaVu Serif");
        assert_eq!(font_display_name("Open_Sans-Regular.otf"), "Open Sans Regular");
    }

    #[test]
    fn wraps_by_words() {
        assert_eq!(wrap_text("eins zwei drei vier", 9), vec!["eins zwei", "drei vier"]);
        assert_eq!(wrap_text("  ", 10), Vec::<String>::new());
        assert_eq!(wrap_text("Donaudampfschiff", 5), vec!["Donaudampfschiff"]);
    }

    #[test]
    fn title_images_only_plain_names() {
        assert!(is_title_image("Strand.JPG"));
        assert!(!is_title_image("../x.jpg"));
        assert!(!is_title_image("x.gif"));
    }

    #[test]
    fn plain_colour_card_without_text() {
        let card = TitleCardConfig {
            enabled: true,
            duration_secs: 4.0,
            background: "#102030".to_string(),
            ..TitleCardConfig::default()
        };
        let dir = std::env::temp_dir();
        let c = build_card(&card, None, 640, 360, 25, &dir, "t", 2, "intro").unwrap();
        assert_eq!(c.args[0], "-f");
        assert!(c.args[3].starts_with("color=c=0x102030:s=640x360:r=25"));
        assert!(c.filter.starts_with("[2:v]"));
        assert!(c.filter.ends_with("[intro]"));
        assert!(c.filter.contains("fade=t=out:st=3.200:d=0.800"));
        assert!((c.duration - 4.0).abs() < 1e-9);
    }

    #[test]
    fn image_card_zooms_and_dims() {
        let card = TitleCardConfig {
            enabled: true,
            duration_secs: 2.0,
            motion: "zoom".to_string(),
            ..TitleCardConfig::default()
        };
        let bg = (Path::new("/tmp/bg.jpg"), Shot::default());
        let c = build_card(&card, Some(bg), 640, 360, 25, Path::new("/tmp"), "t", 1, "o").unwrap();
        assert_eq!(c.args, vec!["-i", "/tmp/bg.jpg"]);
        assert!(c.filter.contains("zoompan=z='1+0.12*on/50'"));
        assert!(c.filter.contains("color=black@0.15"));
    }

    #[test]
    fn card_motions() {
        let card = |zoom: bool, m: &str| TitleCardConfig {
            zoom,
            motion: m.to_string(),
            ..TitleCardConfig::default()
        };
        assert_eq!(motion(&card(false, "pan_right"), Shot::default(), 50, 0).0, "1");
        assert_eq!(motion(&card(true, "zoom"), Shot::default(), 50, 0).0, "1+0.12*on/50");
        let (z, x, y) = motion(&card(true, "pan_right"), Shot::default(), 50, 0);
        assert_eq!(z, "1.2+0.04*on/50");
        assert_eq!(x, "(iw-iw/zoom)*on/50");
        assert_eq!(y, "ih/2-(ih/zoom/2)");
        assert_eq!(motion(&card(true, "pan_up"), Shot::default(), 50, 0).2, "(ih-ih/zoom)*(1-on/50)");
        for seed in 0..8 {
            assert!(motion(&card(true, "random"), Shot::default(), 50, seed).0.starts_with("1.2+"));
        }
        // auto, landscape with a face on the right: from the left edge to the face
        let shot = Shot { portrait: false, focus: Some((0.7, 0.4)) };
        let (z, x, y) = motion(&card(true, "auto"), shot, 50, 0);
        assert!(z.starts_with("1.15+0.3000*"));
        assert!(x.starts_with("clip((0.0000+0.7000*"));
        assert!(y.starts_with("clip((0.5000+-0.1000*"));
        // auto, portrait without a face: top to bottom
        let shot = Shot { portrait: true, focus: None };
        let (_, x, y) = motion(&card(true, "auto"), shot, 50, 0);
        assert!(x.starts_with("clip((0.5000+0.0000*"));
        assert!(y.starts_with("clip((0.0000+1.0000*"));
        // Old configs without `motion` keep the zoom into the middle
        assert_eq!(motion(&card(true, ""), Shot::default(), 50, 0).0, "1+0.12*on/50");
    }

    #[test]
    fn focus_follows_the_crop() {
        // 4:3 photo into a 16:9 background: cut at top and bottom
        let (x, y) = crop_focus((0.5, 0.5), 4000, 3000, 1600, 900);
        assert!((x - 0.5).abs() < 1e-9 && (y - 0.5).abs() < 1e-9);
        let (_, y) = crop_focus((0.5, 0.2), 4000, 3000, 1600, 900);
        assert!((0.0..0.2).contains(&y));
        // Portrait photo into landscape: a face high up lands at the top
        let (_, y) = crop_focus((0.5, 0.1), 3000, 4000, 1600, 900);
        assert_eq!(y, 0.0);
    }

    #[test]
    fn background_size_for_zoom() {
        assert_eq!(background_size(1920, 1080, true), (3840, 2160));
        assert_eq!(background_size(512, 512, true), (1024, 1024));
        assert_eq!(background_size(3840, 2160, true), (3840, 2160));
        assert_eq!(background_size(640, 360, false), (640, 360));
    }
}
