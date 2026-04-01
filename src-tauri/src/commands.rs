use crate::project::{Grade, GradeSource, Photo, Project};
use crate::thumbnailer;
use serde::Serialize;
use std::path::{Path, PathBuf};
use tauri::ipc::Channel;
use walkdir::WalkDir;

const IMAGE_EXTENSIONS: &[&str] = &["jpg", "jpeg", "png", "heic", "tif", "tiff"];

fn is_image_file(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| IMAGE_EXTENSIONS.contains(&ext.to_lowercase().as_str()))
        .unwrap_or(false)
}

#[derive(Clone, Serialize)]
pub struct ProgressPayload {
    pub current: usize,
    pub total: usize,
    pub message: String,
}

#[tauri::command]
pub async fn import_folder(path: String) -> Result<Project, String> {
    let source_dir = PathBuf::from(&path);
    if !source_dir.is_dir() {
        return Err(format!("{} is not a directory", path));
    }

    let folder_name = source_dir
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("Untitled")
        .to_string();

    let mut project = Project::new(folder_name, source_dir.clone());

    // Walk the directory (non-recursive — only top level)
    let mut photos: Vec<Photo> = WalkDir::new(&source_dir)
        .max_depth(1)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file() && is_image_file(e.path()))
        .map(|e| Photo {
            path: e.path().to_path_buf(),
            filename: e.file_name().to_string_lossy().to_string(),
            grade: Grade::default(),
            grade_source: GradeSource::default(),
            faces: Vec::new(),
            aesthetic_score: None,
            sharpness_score: None,
        })
        .collect();

    // Sort by filename for consistent ordering
    photos.sort_by(|a, b| a.filename.cmp(&b.filename));
    project.photos = photos;

    // Save to disk
    project.save()?;

    Ok(project)
}

#[tauri::command]
pub async fn get_project(id: String) -> Result<Project, String> {
    Project::load(&id)
}

#[tauri::command]
pub async fn list_projects() -> Result<Vec<Project>, String> {
    Project::list_all()
}

#[tauri::command]
pub async fn update_grade(
    project_id: String,
    photo_path: String,
    grade: Grade,
) -> Result<(), String> {
    let mut project = Project::load(&project_id)?;
    let photo = project
        .photos
        .iter_mut()
        .find(|p| p.path.to_string_lossy() == photo_path)
        .ok_or("Photo not found in project")?;
    photo.grade = grade;
    photo.grade_source = GradeSource::Manual;
    project.save()?;
    Ok(())
}

#[tauri::command]
pub async fn start_face_detection(
    _project_id: String,
    _on_progress: Channel<ProgressPayload>,
) -> Result<Project, String> {
    Err("Not implemented yet".into())
}

#[tauri::command]
pub async fn start_auto_grade(
    _project_id: String,
    _on_progress: Channel<ProgressPayload>,
) -> Result<Project, String> {
    Err("Not implemented yet".into())
}

#[tauri::command]
pub async fn generate_thumbnails(
    project_id: String,
    on_progress: Channel<ProgressPayload>,
) -> Result<usize, String> {
    let project = Project::load(&project_id)?;
    let photos: Vec<(PathBuf, String)> = project
        .photos
        .iter()
        .map(|p| (p.path.clone(), p.filename.clone()))
        .collect();

    let count = thumbnailer::generate_all_thumbnails(&photos, &project_id, |current, total| {
        let _ = on_progress.send(ProgressPayload {
            current,
            total,
            message: "Generating thumbnails...".into(),
        });
    })?;

    Ok(count)
}

#[tauri::command]
pub async fn get_thumbnail_path(project_id: String, filename: String) -> Result<String, String> {
    let path = thumbnailer::thumbnail_path(&project_id, &filename)?;
    if path.exists() {
        Ok(path.to_string_lossy().to_string())
    } else {
        Err("Thumbnail not found".into())
    }
}
