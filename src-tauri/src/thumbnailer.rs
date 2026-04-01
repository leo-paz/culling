use image::imageops::FilterType;
use std::fs;
use std::path::{Path, PathBuf};

use crate::project::data_dir;

/// Get the thumbnail directory for a project
pub fn thumbnail_dir(project_id: &str) -> Result<PathBuf, String> {
    Ok(data_dir()?.join("thumbnails").join(project_id))
}

/// Get the thumbnail path for a specific photo
pub fn thumbnail_path(project_id: &str, filename: &str) -> Result<PathBuf, String> {
    Ok(thumbnail_dir(project_id)?.join(filename))
}

/// Generate a thumbnail for a single photo. Returns the thumbnail path.
pub fn generate_thumbnail(
    photo_path: &Path,
    project_id: &str,
    filename: &str,
) -> Result<PathBuf, String> {
    let thumb_dir = thumbnail_dir(project_id)?;
    fs::create_dir_all(&thumb_dir).map_err(|e| e.to_string())?;

    let output_path = thumb_dir.join(filename);

    // Skip if already exists
    if output_path.exists() {
        return Ok(output_path);
    }

    let img = image::open(photo_path)
        .map_err(|e| format!("Failed to open {}: {}", photo_path.display(), e))?;

    // Resize to fit within 300x300, maintaining aspect ratio
    let thumb = img.resize(300, 300, FilterType::Lanczos3);

    thumb
        .save(&output_path)
        .map_err(|e| format!("Failed to save thumbnail: {}", e))?;

    Ok(output_path)
}

/// Generate thumbnails for all photos in a project.
/// Returns the number of thumbnails generated.
/// Uses a callback for progress reporting.
pub fn generate_all_thumbnails<F>(
    photos: &[(PathBuf, String)],
    project_id: &str,
    on_progress: F,
) -> Result<usize, String>
where
    F: Fn(usize, usize) + Send + Sync,
{
    let total = photos.len();
    let mut generated = 0;

    for (i, (path, filename)) in photos.iter().enumerate() {
        match generate_thumbnail(path, project_id, filename) {
            Ok(_) => generated += 1,
            Err(e) => eprintln!(
                "Warning: failed to generate thumbnail for {}: {}",
                filename, e
            ),
        }
        on_progress(i + 1, total);
    }

    Ok(generated)
}
