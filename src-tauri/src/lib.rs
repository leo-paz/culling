mod commands;
mod config;
mod error;
mod grader;
mod models;
mod organizer;
pub mod pipeline;
mod project;
mod scanner;
mod thumbnailer;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            commands::import_folder,
            commands::get_project,
            commands::list_projects,
            commands::update_grade,
            commands::start_face_detection,
            commands::start_auto_grade,
            commands::generate_thumbnails,
            commands::get_thumbnail_path,
            commands::check_models,
            commands::export_photos,
            commands::get_config,
            commands::update_config,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
