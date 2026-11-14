//! Timestamp overlay step.
//!
//! Overlays date information on the processed image, either with the
//! built-in 5x7 pixel font or with a TrueType font from the font folders.

use crate::config::{Config, DateFormat, TextPosition, TimestampConfig};
use crate::pipeline::{draw_scaled_text, PipelineContext, ProcessingStep, StepOutcome};
use ab_glyph::{FontArc, PxScale};
use async_trait::async_trait;
use chrono::{Datelike, NaiveDate};
use image::{DynamicImage, Rgb, RgbImage};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

/// Font used when the text needs letters and no font is chosen.
const FALLBACK_FONT: &str = "DejaVuSans-Bold.ttf";

/// Cap height of a typical font as share of its em size.
const CAP_HEIGHT: f32 = 0.72;

const MONTHS_DE: [&str; 12] = [
    "Januar",
    "Februar",
    "März",
    "April",
    "Mai",
    "Juni",
    "Juli",
    "August",
    "September",
    "Oktober",
    "November",
    "Dezember",
];
const MONTHS_DE_SHORT: [&str; 12] = [
    "Jan.", "Feb.", "März", "Apr.", "Mai", "Juni", "Juli", "Aug.", "Sept.", "Okt.", "Nov.", "Dez.",
];
const MONTHS_EN: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];
const MONTHS_EN_SHORT: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];
const WEEKDAYS_DE: [&str; 7] = [
    "Montag",
    "Dienstag",
    "Mittwoch",
    "Donnerstag",
    "Freitag",
    "Samstag",
    "Sonntag",
];
const WEEKDAYS_EN: [&str; 7] = [
    "Monday",
    "Tuesday",
    "Wednesday",
    "Thursday",
    "Friday",
    "Saturday",
    "Sunday",
];

/// Overlays timestamp on the image.
///
/// This step renders date information (year, month, day) on the final image
/// at the specified position.
pub struct TimestampStep;

impl TimestampStep {
    /// Parse a timestamp string ("2024-01-15", "2024-01-15T12:34:56.789Z")
    /// into (year, month, day).
    fn parse_timestamp(timestamp: &str) -> Option<(i32, u32, u32)> {
        let date_part = timestamp.split('T').next()?;
        let parts: Vec<&str> = date_part.split('-').collect();
        if parts.len() < 3 {
            return None;
        }
        let y = parts[0].trim().parse().ok()?;
        let m: u32 = parts[1].trim().parse().ok()?;
        let d: u32 = parts[2].trim().get(..2).unwrap_or(parts[2]).parse().ok()?;
        if !(1..=12).contains(&m) || !(1..=31).contains(&d) {
            return None;
        }
        Some((y, m, d))
    }

    /// Format the date according to the chosen format and parts.
    fn format_date(y: i32, m: u32, d: u32, config: &TimestampConfig) -> String {
        let en = config.lang == "en";
        let numeric = |order: &str, sep: &str, pad: bool, short_year: bool| {
            let parts: Vec<String> = order
                .chars()
                .filter_map(|c| match c {
                    'd' if config.day => Some(if pad {
                        format!("{d:02}")
                    } else {
                        d.to_string()
                    }),
                    'm' if config.month => Some(if pad {
                        format!("{m:02}")
                    } else {
                        m.to_string()
                    }),
                    'y' if config.year => Some(if short_year {
                        format!("{:02}", y.rem_euclid(100))
                    } else {
                        y.to_string()
                    }),
                    _ => None,
                })
                .collect();
            parts.join(sep)
        };
        let named = |short: bool, with_day: bool| {
            let months = match (en, short) {
                (false, false) => &MONTHS_DE,
                (false, true) => &MONTHS_DE_SHORT,
                (true, false) => &MONTHS_EN,
                (true, true) => &MONTHS_EN_SHORT,
            };
            let mut parts = Vec::new();
            if with_day && config.day {
                parts.push(if en { d.to_string() } else { format!("{d}.") });
            }
            if config.month {
                parts.push(months[(m - 1) as usize].to_string());
            }
            if config.year {
                parts.push(y.to_string());
            }
            parts.join(" ")
        };
        match config.format {
            DateFormat::Iso => numeric("ymd", "-", true, false),
            DateFormat::DmyDot => numeric("dmy", ".", true, false),
            DateFormat::DmyDotShort => numeric("dmy", ".", false, false),
            DateFormat::DmyDotYy => numeric("dmy", ".", true, true),
            DateFormat::DmyDash => numeric("dmy", "-", true, false),
            DateFormat::DmySlash => numeric("dmy", "/", true, false),
            DateFormat::MdyDash => numeric("mdy", "-", true, false),
            DateFormat::MdySlash => numeric("mdy", "/", true, false),
            DateFormat::Long => named(false, true),
            DateFormat::LongShort => named(true, true),
            DateFormat::MonthYear => named(false, false),
            DateFormat::Weekday => {
                let text = named(false, true);
                match NaiveDate::from_ymd_opt(y, m, d) {
                    Some(date) if config.day && config.month && config.year => {
                        let i = date.weekday().num_days_from_monday() as usize;
                        let wd = if en { WEEKDAYS_EN[i] } else { WEEKDAYS_DE[i] };
                        format!("{wd}, {text}")
                    }
                    _ => text,
                }
            }
        }
    }

    /// Pixel scale of the 5x7 font for the configured font size: 1 keeps the
    /// original 7 pixel high text.
    fn font_scale(font_size: Option<f32>, img_height: u32) -> u32 {
        match font_size {
            Some(percent) => {
                let pixels = percent / 100.0 * img_height as f32;
                (pixels / CHAR_HEIGHT as f32).round().max(1.0) as u32
            }
            None => 1,
        }
    }

    /// Top-left corner of a text box of the given size.
    fn place(
        position: TextPosition,
        text_width: u32,
        text_height: u32,
        margin: u32,
        img_width: u32,
        img_height: u32,
    ) -> (u32, u32) {
        let left = margin;
        let center = img_width.saturating_sub(text_width) / 2;
        let right = img_width.saturating_sub(text_width + margin);
        let top = margin;
        let bottom = img_height.saturating_sub(text_height + margin);

        match position {
            TextPosition::TopLeft => (left, top),
            TextPosition::TopCenter => (center, top),
            TextPosition::TopRight => (right, top),
            TextPosition::BottomLeft => (left, bottom),
            TextPosition::BottomCenter => (center, bottom),
            TextPosition::BottomRight => (right, bottom),
        }
    }

    /// Position of pixel-font text: (x, y) of its top-left corner.
    fn calculate_position(
        position: TextPosition,
        text: &str,
        img_width: u32,
        img_height: u32,
        scale: u32,
    ) -> (u32, u32) {
        let text_width = text.chars().count() as u32 * CHAR_WIDTH * scale;
        Self::place(
            position,
            text_width,
            CHAR_HEIGHT * scale,
            MARGIN * scale,
            img_width,
            img_height,
        )
    }

    /// "ffffff" or "#ffffff" → colour; white when invalid.
    fn parse_color(hex: &str) -> Rgb<u8> {
        let h = hex.trim_start_matches('#');
        let byte = |i: usize| h.get(i..i + 2).and_then(|s| u8::from_str_radix(s, 16).ok());
        match (byte(0), byte(2), byte(4)) {
            (Some(r), Some(g), Some(b)) if h.len() == 6 => Rgb([r, g, b]),
            _ => Rgb([255, 255, 255]),
        }
    }

    /// Whether the pixel font can draw every character of text.
    fn pixel_font_can(text: &str) -> bool {
        text.chars()
            .all(|c| c.is_ascii_digit() || matches!(c, '-' | '.' | '/' | ' '))
    }

    /// Loaded font by file name, cached for the following frames.
    fn load_font(file: &str) -> Option<FontArc> {
        static CACHE: OnceLock<Mutex<HashMap<String, Option<FontArc>>>> = OnceLock::new();
        let cache = CACHE.get_or_init(|| Mutex::new(HashMap::new()));
        let mut map = cache.lock().ok()?;
        map.entry(file.to_string())
            .or_insert_with(|| {
                let path: PathBuf = crate::video::titles::find_font(file)?;
                let bytes = std::fs::read(path).ok()?;
                FontArc::try_from_vec(bytes).ok()
            })
            .clone()
    }

    /// Draw the date with the pixel font.
    fn draw_pixel(rgb: &mut RgbImage, text: &str, config: &TimestampConfig, color: Rgb<u8>) {
        let (w, h) = (rgb.width(), rgb.height());
        let scale = Self::font_scale(config.font_size, h);
        let (x, y) = Self::calculate_position(config.position, text, w, h, scale);
        if config.shadow {
            draw_scaled_text(rgb, x + scale, y + scale, text, Rgb([0, 0, 0]), scale);
        }
        draw_scaled_text(rgb, x, y, text, color, scale);
    }

    /// Draw the date with a TrueType font. The cap height matches the pixel
    /// font's height for the same font size.
    fn draw_font(
        rgb: &mut RgbImage,
        text: &str,
        config: &TimestampConfig,
        color: Rgb<u8>,
        font: &FontArc,
    ) {
        let (w, h) = (rgb.width(), rgb.height());
        let cap = (Self::font_scale(config.font_size, h) * CHAR_HEIGHT) as f32;
        let scale = PxScale::from(cap / CAP_HEIGHT);
        let (tw, th) = imageproc::drawing::text_size(scale, font, text);
        let margin = (cap / CHAR_HEIGHT as f32 * MARGIN as f32).round() as u32;
        let (x, y) = Self::place(config.position, tw, th, margin, w, h);
        if config.shadow {
            let off = (cap / 14.0).round().max(1.0) as i32;
            imageproc::drawing::draw_text_mut(
                rgb,
                Rgb([0, 0, 0]),
                x as i32 + off,
                y as i32 + off,
                scale,
                font,
                text,
            );
        }
        imageproc::drawing::draw_text_mut(rgb, color, x as i32, y as i32, scale, font, text);
    }
}

/// Width of a font character including spacing, at scale 1.
const CHAR_WIDTH: u32 = 6;
/// Height of a font character at scale 1.
const CHAR_HEIGHT: u32 = 7;
/// Distance of the text from the frame edges at scale 1.
const MARGIN: u32 = 8;

#[async_trait]
impl ProcessingStep for TimestampStep {
    fn id(&self) -> &'static str {
        "timestamp"
    }

    fn name(&self) -> &'static str {
        "Timestamp Overlay"
    }

    async fn execute(&self, mut ctx: PipelineContext, config: &Config) -> StepOutcome {
        let step_config = &config.processing.timestamp;

        let image = match ctx.take_image("timestamp overlay") {
            Ok(img) => img,
            Err(e) => return StepOutcome::Error { ctx, error: e },
        };

        let Some((y, m, d)) = Self::parse_timestamp(&ctx.timestamp) else {
            // Failed to parse timestamp - continue without overlay
            ctx.image = Some(image);
            return StepOutcome::Continue(ctx);
        };

        let date_str = Self::format_date(y, m, d, step_config);
        if date_str.is_empty() {
            ctx.image = Some(image);
            return StepOutcome::Continue(ctx);
        }

        let mut rgb = image.to_rgb8();
        let color = Self::parse_color(&step_config.color);

        // The pixel font only knows digits and a few signs: names need a real font
        let font_file = if !step_config.font.is_empty() {
            Some(step_config.font.as_str())
        } else if !Self::pixel_font_can(&date_str) {
            Some(FALLBACK_FONT)
        } else {
            None
        };
        match font_file.and_then(Self::load_font) {
            Some(font) => Self::draw_font(&mut rgb, &date_str, step_config, color, &font),
            None => Self::draw_pixel(&mut rgb, &date_str, step_config, color),
        }

        ctx.image = Some(DynamicImage::ImageRgb8(rgb));
        StepOutcome::Continue(ctx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::immich_api::FaceData;
    use image::{DynamicImage, RgbImage};

    fn make_ctx_with_image(timestamp: &str) -> PipelineContext {
        let face_data = FaceData {
            bounding_box_x1: 0.0,
            bounding_box_y1: 0.0,
            bounding_box_x2: 10.0,
            bounding_box_y2: 10.0,
            image_width: 10,
            image_height: 10,
        };
        let img = RgbImage::new(512, 512);
        PipelineContext::new("test".to_string(), timestamp.to_string(), face_data)
            .with_image(DynamicImage::ImageRgb8(img))
    }

    fn cfg(format: DateFormat, day: bool, month: bool, year: bool) -> TimestampConfig {
        TimestampConfig {
            format,
            day,
            month,
            year,
            ..TimestampConfig::default()
        }
    }

    #[test]
    fn test_parse_timestamp() {
        assert_eq!(
            TimestampStep::parse_timestamp("2024-01-15"),
            Some((2024, 1, 15))
        );
        assert_eq!(
            TimestampStep::parse_timestamp("2024-01-15T12:34:56.789Z"),
            Some((2024, 1, 15))
        );
        assert_eq!(TimestampStep::parse_timestamp("invalid"), None);
    }

    #[test]
    fn test_format_numeric() {
        let f = |fmt| TimestampStep::format_date(2024, 5, 7, &cfg(fmt, true, true, true));
        assert_eq!(f(DateFormat::Iso), "2024-05-07");
        assert_eq!(f(DateFormat::DmyDot), "07.05.2024");
        assert_eq!(f(DateFormat::DmyDotShort), "7.5.2024");
        assert_eq!(f(DateFormat::DmyDotYy), "07.05.24");
        assert_eq!(f(DateFormat::DmyDash), "07-05-2024");
        assert_eq!(f(DateFormat::DmySlash), "07/05/2024");
        assert_eq!(f(DateFormat::MdyDash), "05-07-2024");
        assert_eq!(f(DateFormat::MdySlash), "05/07/2024");
    }

    #[test]
    fn test_format_parts() {
        let c = cfg(DateFormat::Iso, true, true, false);
        assert_eq!(TimestampStep::format_date(2024, 1, 15, &c), "01-15");
        let c = cfg(DateFormat::DmyDot, false, true, true);
        assert_eq!(TimestampStep::format_date(2024, 1, 15, &c), "01.2024");
        let c = cfg(DateFormat::Iso, false, false, false);
        assert_eq!(TimestampStep::format_date(2024, 1, 15, &c), "");
    }

    #[test]
    fn test_format_names() {
        let f = |fmt| TimestampStep::format_date(2024, 5, 17, &cfg(fmt, true, true, true));
        assert_eq!(f(DateFormat::Long), "17. Mai 2024");
        assert_eq!(f(DateFormat::MonthYear), "Mai 2024");
        assert_eq!(f(DateFormat::Weekday), "Freitag, 17. Mai 2024");
        let c = cfg(DateFormat::LongShort, true, true, true);
        assert_eq!(TimestampStep::format_date(2024, 9, 3, &c), "3. Sept. 2024");
        let mut c = cfg(DateFormat::Weekday, true, true, true);
        c.lang = "en".to_string();
        assert_eq!(
            TimestampStep::format_date(2024, 5, 17, &c),
            "Friday, 17 May 2024"
        );
    }

    #[test]
    fn test_parse_color() {
        assert_eq!(TimestampStep::parse_color("#ff8000"), Rgb([255, 128, 0]));
        assert_eq!(TimestampStep::parse_color("bad"), Rgb([255, 255, 255]));
    }

    #[test]
    fn test_pixel_font_can() {
        assert!(TimestampStep::pixel_font_can("17.05.2024"));
        assert!(TimestampStep::pixel_font_can("05/17/2024"));
        assert!(!TimestampStep::pixel_font_can("17. Mai 2024"));
    }

    #[tokio::test]
    async fn test_timestamp_overlay() {
        let step = TimestampStep;
        let ctx = make_ctx_with_image("2024-01-15");

        let mut config = Config::default();
        config.processing.timestamp.enabled = true;

        match step.execute(ctx, &config).await {
            StepOutcome::Continue(ctx) => assert!(ctx.image.is_some()),
            _ => panic!("Expected Continue"),
        }
    }

    #[tokio::test]
    async fn test_timestamp_overlay_no_components() {
        let step = TimestampStep;
        let ctx = make_ctx_with_image("2024-01-15");

        let mut config = Config::default();
        config.processing.timestamp.year = false;
        config.processing.timestamp.month = false;
        config.processing.timestamp.day = false;

        match step.execute(ctx, &config).await {
            StepOutcome::Continue(ctx) => assert!(ctx.image.is_some()),
            _ => panic!("Expected Continue"),
        }
    }

    #[test]
    fn test_calculate_position_top_left() {
        let (x, y) =
            TimestampStep::calculate_position(TextPosition::TopLeft, "2024-01-15", 512, 512, 1);
        assert_eq!((x, y), (8, 8));
    }

    #[test]
    fn test_calculate_position_bottom_right() {
        // 10 chars * 6px = 60px wide, 7px tall: x = 512 - 60 - 8, y = 512 - 7 - 8
        let (x, y) =
            TimestampStep::calculate_position(TextPosition::BottomRight, "2024-01-15", 512, 512, 1);
        assert_eq!((x, y), (444, 497));
    }

    #[test]
    fn test_calculate_position_centered() {
        let top =
            TimestampStep::calculate_position(TextPosition::TopCenter, "2024-01-15", 512, 512, 1);
        assert_eq!(top, (226, 8));
        let bottom = TimestampStep::calculate_position(
            TextPosition::BottomCenter,
            "2024-01-15",
            512,
            288,
            1,
        );
        assert_eq!(bottom, (226, 288 - 7 - 8));
    }

    #[test]
    fn test_calculate_position_scaled() {
        // Scale 3: 10 chars * 18px = 180px wide, 21px tall, 24px margin
        let (x, y) =
            TimestampStep::calculate_position(TextPosition::BottomRight, "2024-01-15", 512, 512, 3);
        assert_eq!((x, y), (512 - 180 - 24, 512 - 21 - 24));
    }

    #[test]
    fn test_font_scale() {
        assert_eq!(TimestampStep::font_scale(None, 1080), 1);
        assert_eq!(TimestampStep::font_scale(Some(4.0), 1050), 6);
        assert_eq!(TimestampStep::font_scale(Some(0.5), 100), 1);
    }
}
