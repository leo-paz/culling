use crate::project::{Grade, Project};
use serde::Serialize;
use tauri::ipc::Channel;

#[derive(Clone, Serialize)]
pub struct ProgressPayload {
    pub current: usize,
    pub total: usize,
    pub message: String,
}

#[tauri::command]
pub async fn import_folder(path: String) -> Result<Project, String> {
    Err("Not implemented yet".into())
}

#[tauri::command]
pub async fn get_project(id: String) -> Result<Project, String> {
    Err("Not implemented yet".into())
}

#[tauri::command]
pub async fn list_projects() -> Result<Vec<Project>, String> {
    Err("Not implemented yet".into())
}

#[tauri::command]
pub async fn update_grade(
    project_id: String,
    photo_path: String,
    grade: Grade,
) -> Result<(), String> {
    Err("Not implemented yet".into())
}

#[tauri::command]
pub async fn start_face_detection(
    project_id: String,
    on_progress: Channel<ProgressPayload>,
) -> Result<Project, String> {
    Err("Not implemented yet".into())
}

#[tauri::command]
pub async fn start_auto_grade(
    project_id: String,
    on_progress: Channel<ProgressPayload>,
) -> Result<Project, String> {
    Err("Not implemented yet".into())
}
