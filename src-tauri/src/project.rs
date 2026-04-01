use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub id: String,
    pub name: String,
    pub source_dir: PathBuf,
    pub photos: Vec<Photo>,
    pub clusters: Vec<Cluster>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Photo {
    pub path: PathBuf,
    pub filename: String,
    pub grade: Grade,
    pub grade_source: GradeSource,
    pub faces: Vec<FaceDetection>,
    pub aesthetic_score: Option<f32>,
    pub sharpness_score: Option<f32>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Default)]
pub enum Grade {
    #[default]
    Ungraded,
    Bad,
    Ok,
    Good,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Default)]
pub enum GradeSource {
    #[default]
    Manual,
    Auto,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FaceDetection {
    pub bbox: [f32; 4],
    pub confidence: f32,
    pub embedding: Vec<f32>,
    pub cluster_id: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Cluster {
    pub id: usize,
    pub label: String,
    pub representative_photo: PathBuf,
    pub representative_bbox: [f32; 4],
    pub photo_count: usize,
}

impl Project {
    pub fn new(name: String, source_dir: PathBuf) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name,
            source_dir,
            photos: Vec::new(),
            clusters: Vec::new(),
        }
    }
}
