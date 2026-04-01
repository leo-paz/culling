//! Heuristic aesthetic scoring based on image properties.
//! A real NIMA model would be more accurate, but this provides a reasonable
//! approximation using contrast and color saturation.

use image::DynamicImage;
use std::path::Path;

/// Compute a simple aesthetic score (0-10) based on image properties.
/// This is a heuristic approximation -- a real NIMA model would be better.
pub fn score_aesthetic(image_path: &Path) -> Result<f32, String> {
    let img = image::open(image_path).map_err(|e| format!("Failed to open image: {}", e))?;

    let contrast_score = compute_contrast(&img);
    let saturation_score = compute_saturation(&img);

    // Combine into 0-10 score (equal weighting)
    let score = (contrast_score * 0.5 + saturation_score * 0.5) * 10.0;
    Ok(score.clamp(0.0, 10.0))
}

/// Compute a contrast score (0-1) based on the standard deviation of luminance.
/// Higher std dev = more contrast = higher score.
/// Normalized by dividing by 128 (half of the 0-255 range) and clamping.
fn compute_contrast(img: &DynamicImage) -> f32 {
    let gray = img.to_luma8();
    let total = gray.width() as f64 * gray.height() as f64;

    if total == 0.0 {
        return 0.0;
    }

    let mut sum = 0.0f64;
    let mut sum_sq = 0.0f64;

    for pixel in gray.pixels() {
        let val = pixel[0] as f64;
        sum += val;
        sum_sq += val * val;
    }

    let mean = sum / total;
    let variance = (sum_sq / total) - (mean * mean);
    let std_dev = variance.max(0.0).sqrt();

    // Normalize: std_dev of 128 would mean maximum spread
    (std_dev / 128.0).min(1.0) as f32
}

/// Compute mean saturation in HSV space (0-1).
/// Higher saturation generally correlates with more visually appealing images.
fn compute_saturation(img: &DynamicImage) -> f32 {
    let rgb = img.to_rgb8();
    let total = rgb.width() as f64 * rgb.height() as f64;

    if total == 0.0 {
        return 0.0;
    }

    let mut saturation_sum = 0.0f64;

    for pixel in rgb.pixels() {
        let r = pixel[0] as f64 / 255.0;
        let g = pixel[1] as f64 / 255.0;
        let b = pixel[2] as f64 / 255.0;

        let max_c = r.max(g).max(b);
        let min_c = r.min(g).min(b);

        // Saturation in HSV: S = (max - min) / max, or 0 if max == 0
        let saturation = if max_c > 0.0 {
            (max_c - min_c) / max_c
        } else {
            0.0
        };

        saturation_sum += saturation;
    }

    (saturation_sum / total) as f32
}
