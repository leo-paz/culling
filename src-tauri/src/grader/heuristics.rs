//! Heuristic image quality assessment: sharpness, exposure.

use image::DynamicImage;
use std::path::Path;

pub struct HeuristicResult {
    pub sharpness: f32,
    pub is_blurry: bool,
    pub is_overexposed: bool,
    pub is_underexposed: bool,
    pub is_bad: bool, // true if any heuristic flags it
}

/// Run all heuristic checks on an image.
pub fn analyze(image_path: &Path) -> Result<HeuristicResult, String> {
    let img = image::open(image_path).map_err(|e| format!("Failed to open image: {}", e))?;

    let sharpness = compute_sharpness(&img);
    let (is_overexposed, is_underexposed) = check_exposure(&img);
    let is_blurry = sharpness < 100.0;

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
/// If >30% of pixels are in the top 10 bins (246-255), flagged as overexposed.
/// If >30% of pixels are in the bottom 10 bins (0-9), flagged as underexposed.
fn check_exposure(img: &DynamicImage) -> (bool, bool) {
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

    (overexposed > 0.3, underexposed > 0.3)
}
