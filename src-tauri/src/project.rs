use serde::{Deserialize, Serialize};
use std::fs;
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

/// Get the base directory for culling data (~/.culling/)
pub fn data_dir() -> Result<PathBuf, String> {
    let home = dirs::home_dir().ok_or("Could not find home directory")?;
    Ok(home.join(".culling"))
}

/// Get the projects directory (~/.culling/projects/)
pub fn projects_dir() -> Result<PathBuf, String> {
    Ok(data_dir()?.join("projects"))
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

    /// Save project to disk
    pub fn save(&self) -> Result<(), String> {
        let dir = projects_dir()?;
        fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        let path = dir.join(format!("{}.json", self.id));
        let json = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        fs::write(path, json).map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Load project from disk by ID
    pub fn load(id: &str) -> Result<Self, String> {
        let path = projects_dir()?.join(format!("{}.json", id));
        let json = fs::read_to_string(&path)
            .map_err(|e| format!("Could not load project: {}", e))?;
        serde_json::from_str(&json).map_err(|e| format!("Invalid project file: {}", e))
    }

    /// List all saved projects
    pub fn list_all() -> Result<Vec<Self>, String> {
        let dir = projects_dir()?;
        if !dir.exists() {
            return Ok(Vec::new());
        }
        let mut projects = Vec::new();
        for entry in fs::read_dir(&dir).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            if entry
                .path()
                .extension()
                .map_or(false, |ext| ext == "json")
            {
                let json = fs::read_to_string(entry.path()).map_err(|e| e.to_string())?;
                if let Ok(project) = serde_json::from_str::<Project>(&json) {
                    projects.push(project);
                }
            }
        }
        Ok(projects)
    }
}
