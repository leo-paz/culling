use std::path::{Path, PathBuf};

use serde::Serialize;
use tauri::ipc::Channel;
use walkdir::WalkDir;

use crate::config::Config;
use crate::error::CullingError;
use crate::organizer::export::{ExportOptions, GradeFilter, Organization};
use crate::pipeline::{self, ProgressFn};
use crate::project::{Grade, GradeSource, Photo, Project};
use crate::thumbnailer;

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
pub async fn import_folder(path: String) -> Result<Project, CullingError> {
    let source_dir = PathBuf::from(&path);
    if !source_dir.is_dir() {
        return Err(CullingError::NotFound(format!(
            "{} is not a directory",
            path
        )));
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
pub async fn get_project(id: String) -> Result<Project, CullingError> {
    Project::load(&id)
}

#[tauri::command]
pub async fn list_projects() -> Result<Vec<Project>, CullingError> {
    Project::list_all()
}

#[tauri::command]
pub async fn update_grade(
    project_id: String,
    photo_path: String,
    grade: Grade,
) -> Result<(), CullingError> {
    let mut project = Project::load(&project_id)?;
    let photo = project
        .photos
        .iter_mut()
        .find(|p| p.path.to_string_lossy() == photo_path)
        .ok_or_else(|| CullingError::NotFound("Photo not found in project".to_string()))?;
    photo.grade = grade;
    photo.grade_source = GradeSource::Manual;
    project.save()?;
    Ok(())
}

#[tauri::command]
pub async fn start_face_detection(
    project_id: String,
    on_progress: Channel<ProgressPayload>,
) -> Result<Project, CullingError> {
    tokio::task::spawn_blocking(move || {
        let mut project = Project::load(&project_id)?;
        let config = Config::load();

        let progress: ProgressFn = Box::new(move |current, total, message| {
            let _ = on_progress.send(ProgressPayload {
                current,
                total,
                message: message.to_string(),
            });
        });

        pipeline::run_face_detection(&mut project, &config, &progress)?;
        Ok(project)
    })
    .await
    .map_err(|e| CullingError::Other(format!("Task panicked: {}", e)))?
}

#[tauri::command]
pub async fn start_auto_grade(
    project_id: String,
    on_progress: Channel<ProgressPayload>,
) -> Result<Project, CullingError> {
    tokio::task::spawn_blocking(move || {
        let mut project = Project::load(&project_id)?;
        let config = Config::load();

        let progress: ProgressFn = Box::new(move |current, total, message| {
            let _ = on_progress.send(ProgressPayload {
                current,
                total,
                message: message.to_string(),
            });
        });

        pipeline::run_auto_grade(&mut project, &config, &progress)?;
        Ok(project)
    })
    .await
    .map_err(|e| CullingError::Other(format!("Task panicked: {}", e)))?
}

#[tauri::command]
pub async fn check_models() -> Result<bool, CullingError> {
    crate::models::models_available()
}

#[tauri::command]
pub async fn generate_thumbnails(
    project_id: String,
    on_progress: Channel<ProgressPayload>,
) -> Result<usize, CullingError> {
    let project = Project::load(&project_id)?;
    let config = Config::load();
    let max_size = config.thumbnails.max_size;
    let photos: Vec<(PathBuf, String)> = project
        .photos
        .iter()
        .map(|p| (p.path.clone(), p.filename.clone()))
        .collect();

    let count =
        thumbnailer::generate_all_thumbnails(&photos, &project_id, max_size, |current, total| {
            let _ = on_progress.send(ProgressPayload {
                current,
                total,
                message: "Generating thumbnails...".into(),
            });
        })?;

    Ok(count)
}

#[tauri::command]
pub async fn get_thumbnail_path(
    project_id: String,
    filename: String,
) -> Result<String, CullingError> {
    let path = thumbnailer::thumbnail_path(&project_id, &filename)?;
    if path.exists() {
        Ok(path.to_string_lossy().to_string())
    } else {
        Err(CullingError::NotFound("Thumbnail not found".to_string()))
    }
}

#[tauri::command]
pub async fn export_photos(
    project_id: String,
    output_dir: String,
    grade_filter: GradeFilter,
    organization: Organization,
    trash_bad: bool,
    on_progress: Channel<ProgressPayload>,
) -> Result<usize, CullingError> {
    let project = Project::load(&project_id)?;

    let options = ExportOptions {
        output_dir: PathBuf::from(output_dir),
        grade_filter,
        organization,
        trash_bad,
    };

    let count =
        crate::organizer::export::export_photos(&project, &options, |current, total| {
            let _ = on_progress.send(ProgressPayload {
                current,
                total,
                message: "Exporting photos...".into(),
            });
        })?;

    Ok(count)
}

/// Read an image file and return it as a base64 data URL.
/// This bypasses the asset protocol entirely.
#[tauri::command]
pub async fn read_image(path: String) -> Result<String, CullingError> {
    use std::fs;
    let data = fs::read(&path)?;
    let ext = std::path::Path::new(&path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("jpeg")
        .to_lowercase();
    let mime = match ext.as_str() {
        "png" => "image/png",
        "tif" | "tiff" => "image/tiff",
        _ => "image/jpeg",
    };
    use base64::Engine;
    let b64 = base64::engine::general_purpose::STANDARD.encode(&data);
    Ok(format!("data:{};base64,{}", mime, b64))
}

#[tauri::command]
pub async fn get_config() -> Config {
    Config::load()
}

#[tauri::command]
pub async fn update_config(config: Config) -> Result<(), CullingError> {
    config.save()
}
