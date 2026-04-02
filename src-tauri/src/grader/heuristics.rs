//! Heuristic image quality assessment: sharpness, exposure.

use crate::config::GradingConfig;
use crate::error::CullingError;
use image::DynamicImage;
use std::path::Path;

pub struct HeuristicResult {
    pub sharpness: f32,
    pub is_blurry: bool,
    pub is_overexposed: bool,
    pub is_underexposed: bool,
    pub is_bad: bool, // true if any heuristic flags it
}

/// Run all heuristic checks on an image loaded from disk.
pub fn analyze(image_path: &Path, config: &GradingConfig) -> Result<HeuristicResult, CullingError> {
    let img = image::open(image_path)?;
    analyze_image(&img, config)
}

/// Run all heuristic checks on a pre-loaded image (avoids redundant disk I/O).
pub fn analyze_image(img: &DynamicImage, config: &GradingConfig) -> Result<HeuristicResult, CullingError> {
    let sharpness = compute_sharpness(img);
    let (is_overexposed, is_underexposed) = check_exposure(img, config.exposure_clip_threshold);
    let is_blurry = sharpness < config.sharpness_threshold;

    Ok(HeuristicResult {
        sharpness,
        is_blurry,
        is_overexposed,
        is_underexposed,
        is_bad: is_blurry || is_overexposed || is_underexposed,
    })
}

/// Compute sharpness using Laplacian variance.
///
/// 1. Convert image to grayscale
/// 2. Resize so the longest side is ~500px (maintaining aspect ratio)
/// 3. Apply 3x3 Laplacian kernel: [[0,1,0],[1,-4,1],[0,1,0]]
/// 4. Compute variance of filtered values
/// 5. Low variance = blurry (threshold ~100)
fn compute_sharpness(img: &DynamicImage) -> f32 {
    let gray = img.to_luma8();

    // Resize maintaining aspect ratio so longest side is 500px
    let (orig_w, orig_h) = (gray.width(), gray.height());
    let longest = orig_w.max(orig_h) as f32;
    let scale = 500.0 / longest;
    let new_w = ((orig_w as f32 * scale).round() as u32).max(3);
    let new_h = ((orig_h as f32 * scale).round() as u32).max(3);

    let resized =
        image::imageops::resize(&gray, new_w, new_h, image::imageops::FilterType::Nearest);
    let (w, h) = (resized.width() as i32, resized.height() as i32);

    let mut sum = 0.0f64;
    let mut sum_sq = 0.0f64;
    let mut count = 0u64;

    // Apply Laplacian kernel: [[0,1,0],[1,-4,1],[0,1,0]]
    for y in 1..(h - 1) {
        for x in 1..(w - 1) {
            let center = resized.get_pixel(x as u32, y as u32)[0] as f64 * -4.0;
            let top = resized.get_pixel(x as u32, (y - 1) as u32)[0] as f64;
            let bottom = resized.get_pixel(x as u32, (y + 1) as u32)[0] as f64;
            let left = resized.get_pixel((x - 1) as u32, y as u32)[0] as f64;
            let right = resized.get_pixel((x + 1) as u32, y as u32)[0] as f64;
            let laplacian = center + top + bottom + left + right;
            sum += laplacian;
            sum_sq += laplacian * laplacian;
            count += 1;
        }
    }

    if count == 0 {
        return 0.0;
    }

    let mean = sum / count as f64;
    let variance = (sum_sq / count as f64) - (mean * mean);
    variance as f32
}

/// Check exposure by analyzing the luminance histogram.
///
/// Returns (is_overexposed, is_underexposed).
/// If more than `clip_threshold` fraction of pixels are in the top 10 bins (246-255), flagged as overexposed.
/// If more than `clip_threshold` fraction of pixels are in the bottom 10 bins (0-9), flagged as underexposed.
fn check_exposure(img: &DynamicImage, clip_threshold: f64) -> (bool, bool) {
    let gray = img.to_luma8();
    let total = gray.width() as f64 * gray.height() as f64;

    if total == 0.0 {
        return (false, false);
    }

    let mut histogram = [0u64; 256];
    for pixel in gray.pixels() {
        histogram[pixel[0] as usize] += 1;
    }

    let underexposed: f64 = histogram[..10].iter().sum::<u64>() as f64 / total;
    let overexposed: f64 = histogram[246..].iter().sum::<u64>() as f64 / total;

    (overexposed > clip_threshold, underexposed > clip_threshold)
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{GrayImage, Luma};

    #[test]
    fn uniform_image_is_blurry() {
        // A solid gray image has zero Laplacian variance
        let img = DynamicImage::ImageLuma8(GrayImage::from_pixel(100, 100, Luma([128])));
        let sharpness = compute_sharpness(&img);
        assert!(
            sharpness < 1.0,
            "Uniform image should have near-zero sharpness, got {}",
            sharpness
        );
    }

    #[test]
    fn high_contrast_image_is_sharp() {
        // Checkerboard pattern has high Laplacian variance
        let mut img = GrayImage::new(100, 100);
        for y in 0..100 {
            for x in 0..100 {
                let val = if (x + y) % 2 == 0 { 255 } else { 0 };
                img.put_pixel(x, y, Luma([val]));
            }
        }
        let sharpness = compute_sharpness(&DynamicImage::ImageLuma8(img));
        assert!(
            sharpness > 100.0,
            "Checkerboard should be sharp, got {}",
            sharpness
        );
    }

    #[test]
    fn overexposed_image_detected() {
        let img = DynamicImage::ImageLuma8(GrayImage::from_pixel(100, 100, Luma([255])));
        let (over, under) = check_exposure(&img, 0.3);
        assert!(over, "All-white image should be flagged as overexposed");
        assert!(!under);
    }

    #[test]
    fn underexposed_image_detected() {
        let img = DynamicImage::ImageLuma8(GrayImage::from_pixel(100, 100, Luma([0])));
        let (over, under) = check_exposure(&img, 0.3);
        assert!(!over);
        assert!(
            under,
            "All-black image should be flagged as underexposed"
        );
    }

    #[test]
    fn normal_exposure_not_flagged() {
        let img = DynamicImage::ImageLuma8(GrayImage::from_pixel(100, 100, Luma([128])));
        let (over, under) = check_exposure(&img, 0.3);
        assert!(!over);
        assert!(!under);
    }
}
