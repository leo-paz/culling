//! Model file management for face detection and embedding.
//!
//! Models are expected at `~/.culling/models/`:
//! - `det_10g.onnx`   — SCRFD face detector (from InsightFace buffalo_l)
//! - `w600k_r50.onnx`  — ArcFace face embedder (from InsightFace buffalo_l)
//!
//! To install models manually, download `buffalo_l.zip` from the InsightFace
//! GitHub releases, extract the ONNX files, and place them in `~/.culling/models/`.

use crate::project::data_dir;
use std::path::PathBuf;

/// Directory where model files are stored (~/.culling/models/).
pub fn models_dir() -> Result<PathBuf, String> {
    Ok(data_dir()?.join("models"))
}

/// Path to the SCRFD face detector model.
pub fn detector_model_path() -> Result<PathBuf, String> {
    Ok(models_dir()?.join("det_10g.onnx"))
}

/// Path to the ArcFace face embedder model.
pub fn embedder_model_path() -> Result<PathBuf, String> {
    Ok(models_dir()?.join("w600k_r50.onnx"))
}

/// Check if all required model files are present on disk.
pub fn models_available() -> Result<bool, String> {
    Ok(detector_model_path()?.exists() && embedder_model_path()?.exists())
}
