//! Face cropping operations.
//!
//! Extracts and resizes face regions from images using bounding box data.

use crate::error::{Error, Result};
use crate::immich_api::FaceData;
use crate::pipeline::BoundingBox;
use image::imageops::FilterType;
use image::{DynamicImage, GenericImageView, RgbImage};
use std::borrow::Cow;

/// Per-edge padding fractions (0.0-1.0): left and right relative to the crop width,
/// top and bottom relative to the crop height.
#[derive(Debug, Clone, Copy)]
pub struct PaddingEdges {
    pub top: f32,
    pub bottom: f32,
    pub left: f32,
    pub right: f32,
}

impl PaddingEdges {
    /// Total fraction of crop area that is padding.
    pub fn total_fraction(&self) -> f32 {
        // Area = 1 - (1-left-right)*(1-top-bottom)
        let inner_w = (1.0 - self.left - self.right).max(0.0);
        let inner_h = (1.0 - self.top - self.bottom).max(0.0);
        1.0 - inner_w * inner_h
    }
}

/// Result of cropping a face from an image.
pub struct CropResult {
    /// The cropped image at full resolution.
    pub cropped: DynamicImage,
    /// The cropped image resized to output size.
    pub resized: DynamicImage,
    /// The face bounding box in crop coordinates.
    pub face_rect: BoundingBox,
    /// Fraction of the crop area that falls outside the original image (0.0-1.0).
    pub padding_fraction: f32,
    /// Per-edge padding fractions.
    pub padding_edges: PaddingEdges,
}

/// Crop and resize the face from an image using bounding box.
/// Returns CropResult containing cropped images and face rectangle in crop coordinates.
///
/// The crop has the aspect ratio of the output frame (`output_width` x `output_height`)
/// and is resized to it, so the padding measures the whole area the frame shows.
pub fn crop_face_with_intermediate(
    img: &DynamicImage,
    face_data: &FaceData,
    output_width: u32,
    output_height: u32,
    eye_distance: f32,
) -> Result<CropResult> {
    let (img_width, img_height) = img.dimensions();

    // Scale bounding box from metadata dimensions to actual image dimensions.
    // Immich stores bounding box as pixel coordinates relative to image_width/image_height,
    // but the loaded image may have different dimensions (e.g., if downloaded at different resolution).
    let scale_x = img_width as f32 / face_data.image_width as f32;
    let scale_y = img_height as f32 / face_data.image_height as f32;

    let x1 = (face_data.bounding_box_x1 * scale_x) as u32;
    let y1 = (face_data.bounding_box_y1 * scale_y) as u32;
    let x2 = (face_data.bounding_box_x2 * scale_x) as u32;
    let y2 = (face_data.bounding_box_y2 * scale_y) as u32;

    let face_width = x2.saturating_sub(x1);
    let face_height = y2.saturating_sub(y1);

    if face_width == 0 || face_height == 0 {
        return Err(Error::ImageProcessing(
            "Invalid face bounding box".to_string(),
        ));
    }

    // Derive the crop width from eye_distance (a fraction of the frame width) so the
    // initial crop captures the right amount of context for the alignment step.
    const IPD_RATIO: f32 = 0.37;
    const SAFETY_MARGIN: f32 = 1.30;

    let face_size = face_width.max(face_height) as f32;
    let estimated_ipd = IPD_RATIO * face_width as f32;
    let ideal_crop = (estimated_ipd / eye_distance) * SAFETY_MARGIN;
    // The short side of the crop holds at least 1.5 face sizes
    let aspect = output_height.max(1) as f32 / output_width.max(1) as f32;
    let min_width = face_size * 1.5 * (1.0 / aspect).max(1.0);
    let crop_w_f = ideal_crop.max(min_width).max(10.0);
    let crop_w = crop_w_f.round() as u32;
    let crop_h = (crop_w_f * aspect).round().max(10.0) as u32;

    // Calculate center of face
    let center_x = (x1 + x2) / 2;
    let center_y = (y1 + y2) / 2;

    // Ideal crop region centered on the face (may extend outside image bounds)
    let ideal_x1 = center_x as i32 - crop_w as i32 / 2;
    let ideal_y1 = center_y as i32 - crop_h as i32 / 2;

    // Calculate what fraction of the crop area falls outside the image bounds.
    let overlap_x1 = (ideal_x1).max(0) as u64;
    let overlap_y1 = (ideal_y1).max(0) as u64;
    let overlap_x2 = ((ideal_x1 + crop_w as i32).max(0) as u64).min(img_width as u64);
    let overlap_y2 = ((ideal_y1 + crop_h as i32).max(0) as u64).min(img_height as u64);
    let overlap_area =
        overlap_x2.saturating_sub(overlap_x1) * overlap_y2.saturating_sub(overlap_y1);
    let total_area = crop_w as u64 * crop_h as u64;
    let padding_fraction = 1.0 - (overlap_area as f32 / total_area as f32);

    // Per-edge padding as fraction of the crop width or height
    let (cw, ch) = (crop_w as f32, crop_h as f32);
    let padding_edges = PaddingEdges {
        left: (-ideal_x1).max(0) as f32 / cw,
        top: (-ideal_y1).max(0) as f32 / ch,
        right: ((ideal_x1 + crop_w as i32) - img_width as i32).max(0) as f32 / cw,
        bottom: ((ideal_y1 + crop_h as i32) - img_height as i32).max(0) as f32 / ch,
    };

    // Crop with replicate-fill: always keeps the face centered by extending
    // edge pixels at image borders instead of shifting the crop window.
    let cropped = crop_with_replicate_fill(img, ideal_x1, ideal_y1, crop_w, crop_h);

    // Resize to the output frame size
    let resized = cropped.resize_exact(output_width, output_height, FilterType::Lanczos3);

    // Face coordinates relative to the crop origin (face is always centered)
    let face_rect = BoundingBox {
        x1: (x1 as i32 - ideal_x1) as f32,
        y1: (y1 as i32 - ideal_y1) as f32,
        x2: (x2 as i32 - ideal_x1) as f32,
        y2: (y2 as i32 - ideal_y1) as f32,
    };

    Ok(CropResult {
        cropped,
        resized,
        face_rect,
        padding_fraction,
        padding_edges,
    })
}

/// Crop a region from an image, filling out-of-bounds areas with replicated edge pixels.
///
/// When the crop region extends past the image borders, edge pixels are repeated
/// (e.g., column -1 uses column 0, column -2 uses column 0, etc.).
fn crop_with_replicate_fill(
    img: &DynamicImage,
    x_offset: i32,
    y_offset: i32,
    width: u32,
    height: u32,
) -> DynamicImage {
    // Avoid allocation if the image is already RGB8
    let rgb: Cow<'_, RgbImage> = match img.as_rgb8() {
        Some(r) => Cow::Borrowed(r),
        None => Cow::Owned(img.to_rgb8()),
    };
    let (w, h) = (rgb.width() as i32, rgb.height() as i32);

    let out = RgbImage::from_fn(width, height, |px, py| {
        let sx = replicate_coord(x_offset + px as i32, w);
        let sy = replicate_coord(y_offset + py as i32, h);
        *rgb.get_pixel(sx as u32, sy as u32)
    });

    DynamicImage::ImageRgb8(out)
}

/// Clamp a coordinate into the valid range [0, len) using replicate/edge boundary.
///
/// For a dimension of length `len`:
/// - Coordinates in `[0, len)` map to themselves.
/// - Negative coordinates clamp to 0.
/// - Coordinates >= len clamp to len-1.
fn replicate_coord(c: i32, len: i32) -> i32 {
    if len <= 1 {
        return 0;
    }
    c.clamp(0, len - 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn face_in(width: u32, height: u32, x1: f32, y1: f32, x2: f32, y2: f32) -> FaceData {
        FaceData {
            bounding_box_x1: x1,
            bounding_box_y1: y1,
            bounding_box_x2: x2,
            bounding_box_y2: y2,
            image_width: width,
            image_height: height,
        }
    }

    #[test]
    fn test_crop_follows_aspect_ratio() {
        let img = DynamicImage::ImageRgb8(RgbImage::new(1000, 1000));
        let face = face_in(1000, 1000, 450.0, 450.0, 550.0, 550.0);
        let result = crop_face_with_intermediate(&img, &face, 1080, 608, 0.15).unwrap();
        assert_eq!(result.resized.width(), 1080);
        assert_eq!(result.resized.height(), 608);
        let (cw, ch) = (result.cropped.width(), result.cropped.height());
        let ratio = cw as f32 / ch as f32;
        assert!((ratio - 1080.0 / 608.0).abs() < 0.02, "crop {}x{}", cw, ch);
        // The short side still holds 1.5 face sizes
        assert!(ch >= 150);
    }

    #[test]
    fn test_padding_counts_the_whole_frame() {
        // Face at the top edge: a tall 9:16 crop reaches further above the photo
        let img = DynamicImage::ImageRgb8(RgbImage::new(1000, 1000));
        let face = face_in(1000, 1000, 450.0, 0.0, 550.0, 100.0);
        let square = crop_face_with_intermediate(&img, &face, 512, 512, 0.15).unwrap();
        let tall = crop_face_with_intermediate(&img, &face, 288, 512, 0.15).unwrap();
        assert!(square.padding_fraction > 0.0);
        assert!(tall.padding_fraction > square.padding_fraction);
        assert!(tall.padding_edges.top > 0.0);
        assert_eq!(tall.padding_edges.left, 0.0);
    }
}
