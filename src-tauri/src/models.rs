//! Model file management for face detection and embedding.
//!
//! Models are expected at `~/.culling/models/`:
//! - `det_10g.onnx`   — SCRFD face detector (from InsightFace buffalo_l)
//! - `w600k_r50.onnx`  — ArcFace face embedder (from InsightFace buffalo_l)
//!
//! To install models manually, download `buffalo_l.zip` from the InsightFace
//! GitHub releases, extract the ONNX files, and place them in `~/.culling/models/`.

use crate::error::CullingError;
use crate::project::data_dir;
use std::fs;
use std::io::Cursor;
use std::path::PathBuf;

const BUFFALO_L_URL: &str =
    "https://github.com/deepinsight/insightface/releases/download/v0.7/buffalo_l.zip";

/// Directory where model files are stored (~/.culling/models/).
pub fn models_dir() -> Result<PathBuf, CullingError> {
    Ok(data_dir()?.join("models"))
}

/// Path to the SCRFD face detector model.
pub fn detector_model_path() -> Result<PathBuf, CullingError> {
    Ok(models_dir()?.join("det_10g.onnx"))
}

/// Path to the ArcFace face embedder model.
pub fn embedder_model_path() -> Result<PathBuf, CullingError> {
    Ok(models_dir()?.join("w600k_r50.onnx"))
}

/// Check if all required model files are present on disk.
pub fn models_available() -> Result<bool, CullingError> {
    Ok(detector_model_path()?.exists() && embedder_model_path()?.exists())
}

/// Download and extract the buffalo_l model pack if models are missing.
/// Returns `Ok(true)` if models were downloaded, `Ok(false)` if already present.
/// The progress callback receives `(message, bytes_downloaded, total_bytes)`.
pub fn ensure_models<F>(on_progress: F) -> Result<bool, CullingError>
where
    F: Fn(&str, u64, u64) + Send + Sync,
{
    if models_available()? {
        return Ok(false);
    }

    let dir = models_dir()?;
    fs::create_dir_all(&dir)?;

    on_progress("Downloading face detection models...", 0, 0);

    // Download the zip file
    let response = reqwest::blocking::get(BUFFALO_L_URL).map_err(|e| {
        CullingError::Other(format!(
            "Failed to download models: {}. Check your internet connection.",
            e
        ))
    })?;

    let total_size = response.content_length().unwrap_or(0);
    let bytes = response
        .bytes()
        .map_err(|e| CullingError::Other(format!("Download interrupted: {}", e)))?;

    on_progress("Extracting models...", total_size, total_size);

    // Extract only the ONNX files we need
    let cursor = Cursor::new(&bytes);
    let mut archive = zip::ZipArchive::new(cursor)
        .map_err(|e| CullingError::Other(format!("Invalid zip archive: {}", e)))?;

    let needed_files = ["det_10g.onnx", "w600k_r50.onnx"];
    let archive_len = archive.len();

    for i in 0..archive_len {
        let mut file = archive
            .by_index(i)
            .map_err(|e| CullingError::Other(format!("Zip extraction error: {}", e)))?;

        let file_name = file.name().to_string();
        // The zip contains files like "buffalo_l/det_10g.onnx" — extract just the filename
        let base_name = std::path::Path::new(&file_name)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("");

        if needed_files.contains(&base_name) {
            let out_path = dir.join(base_name);
            let mut out_file = fs::File::create(&out_path)?;
            std::io::copy(&mut file, &mut out_file)?;
            on_progress(
                &format!("Extracted {}", base_name),
                i as u64,
                archive_len as u64,
            );
        }
    }

    // Verify both models exist now
    if !models_available()? {
        return Err(CullingError::ModelNotFound(
            "Download completed but expected model files were not found in the archive".into(),
        ));
    }

    Ok(true)
}
