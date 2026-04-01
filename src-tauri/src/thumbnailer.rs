use crate::error::CullingError;
use crate::project::data_dir;
use image::imageops::FilterType;
use std::fs;
use std::path::{Path, PathBuf};

/// Get the thumbnail directory for a project
pub fn thumbnail_dir(project_id: &str) -> Result<PathBuf, CullingError> {
    Ok(data_dir()?.join("thumbnails").join(project_id))
}

/// Get the thumbnail path for a specific photo
pub fn thumbnail_path(project_id: &str, filename: &str) -> Result<PathBuf, CullingError> {
    Ok(thumbnail_dir(project_id)?.join(filename))
}

/// Generate a thumbnail for a single photo. Returns the thumbnail path.
pub fn generate_thumbnail(
    photo_path: &Path,
    project_id: &str,
    filename: &str,
    max_size: u32,
) -> Result<PathBuf, CullingError> {
    let thumb_dir = thumbnail_dir(project_id)?;
    fs::create_dir_all(&thumb_dir)?;

    let output_path = thumb_dir.join(filename);

    // Skip if already exists
    if output_path.exists() {
        return Ok(output_path);
    }

    let img = image::open(photo_path)?;

    // Resize to fit within max_size x max_size, maintaining aspect ratio
    let thumb = img.resize(max_size, max_size, FilterType::Lanczos3);

    thumb.save(&output_path)?;

    Ok(output_path)
}

/// Generate thumbnails for all photos in a project.
/// Returns the number of thumbnails generated.
/// Uses a callback for progress reporting.
pub fn generate_all_thumbnails<F>(
    photos: &[(PathBuf, String)],
    project_id: &str,
    max_size: u32,
    on_progress: F,
) -> Result<usize, CullingError>
where
    F: Fn(usize, usize) + Send + Sync,
{
    let total = photos.len();
    let mut generated = 0;

    for (i, (path, filename)) in photos.iter().enumerate() {
        match generate_thumbnail(path, project_id, filename, max_size) {
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
