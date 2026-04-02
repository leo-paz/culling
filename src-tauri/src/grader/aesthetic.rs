//! Heuristic aesthetic scoring based on image properties.
//! A real NIMA model would be more accurate, but this provides a reasonable
//! approximation using contrast and color saturation.

use image::DynamicImage;

/// Compute a simple aesthetic score (0-10) from a pre-loaded image.
/// Avoids redundant disk I/O when the image is already in memory.
pub fn score_aesthetic_image(img: &DynamicImage) -> f32 {
    let contrast_score = compute_contrast(img);
    let saturation_score = compute_saturation(img);
    ((contrast_score * 0.5 + saturation_score * 0.5) * 10.0).clamp(0.0, 10.0)
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

#[cfg(test)]
mod tests {
    use super::*;
    use image::{GrayImage, Luma, Rgb, RgbImage};

    #[test]
    fn uniform_image_has_zero_contrast() {
        let img = DynamicImage::ImageLuma8(GrayImage::from_pixel(50, 50, Luma([128])));
        let score = compute_contrast(&img);
        assert!(
            score < 0.01,
            "Uniform gray should have ~0 contrast, got {}",
            score
        );
    }

    #[test]
    fn grayscale_image_has_zero_saturation() {
        let img = DynamicImage::ImageRgb8(RgbImage::from_pixel(50, 50, Rgb([128, 128, 128])));
        let score = compute_saturation(&img);
        assert!(
            score < 0.01,
            "Gray image should have ~0 saturation, got {}",
            score
        );
    }

    #[test]
    fn saturated_image_has_high_saturation() {
        let img = DynamicImage::ImageRgb8(RgbImage::from_pixel(50, 50, Rgb([255, 0, 0])));
        let score = compute_saturation(&img);
        assert!(
            score > 0.9,
            "Pure red should have high saturation, got {}",
            score
        );
    }
}
