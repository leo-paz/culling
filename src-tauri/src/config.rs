use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

use crate::error::CullingError;
use crate::project::data_dir;

/// Application configuration with all tunable thresholds.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub detection: DetectionConfig,
    pub clustering: ClusteringConfig,
    pub grading: GradingConfig,
    pub thumbnails: ThumbnailConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectionConfig {
    /// Minimum face detection confidence (0.0 - 1.0)
    pub min_confidence: f32,
    /// Minimum face size in pixels (shortest bbox side)
    pub min_face_size: u32,
    /// NMS IoU overlap threshold
    pub nms_threshold: f32,
    /// Detection input size (width, height)
    pub input_size: (u32, u32),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusteringConfig {
    /// DBSCAN epsilon (distance threshold)
    pub eps: f64,
    /// DBSCAN minimum samples per cluster
    pub min_samples: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GradingConfig {
    /// Laplacian variance below this = blurry
    pub sharpness_threshold: f32,
    /// Percentage of clipped pixels to flag exposure issues (0.0 - 1.0)
    pub exposure_clip_threshold: f64,
    /// Aesthetic score >= this = Good, below = Ok
    pub aesthetic_good_threshold: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThumbnailConfig {
    /// Max thumbnail dimension in pixels
    pub max_size: u32,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            detection: DetectionConfig {
                min_confidence: 0.5,
                min_face_size: 80,
                nms_threshold: 0.4,
                input_size: (640, 640),
            },
            clustering: ClusteringConfig {
                eps: 0.75,
                min_samples: 2,
            },
            grading: GradingConfig {
                sharpness_threshold: 100.0,
                exposure_clip_threshold: 0.3,
                aesthetic_good_threshold: 5.0,
            },
            thumbnails: ThumbnailConfig {
                max_size: 300,
            },
        }
    }
}

impl Config {
    /// Load config from ~/.culling/config.json, or return defaults if not found.
    pub fn load() -> Self {
        let path = match config_path() {
            Ok(p) => p,
            Err(_) => return Self::default(),
        };
        match fs::read_to_string(&path) {
            Ok(json) => serde_json::from_str(&json).unwrap_or_default(),
            Err(_) => Self::default(),
        }
    }

    /// Save config to disk.
    pub fn save(&self) -> Result<(), CullingError> {
        let path = config_path()?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(self)?;
        fs::write(path, json)?;
        Ok(())
    }
}

fn config_path() -> Result<PathBuf, CullingError> {
    Ok(data_dir()?.join("config.json"))
}
