use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::Serialize;
use tauri::ipc::Channel;
use walkdir::WalkDir;

use crate::error::CullingError;
use crate::organizer::export::{ExportOptions, GradeFilter, Organization};
use crate::project::{Cluster, FaceDetection, Grade, GradeSource, Photo, Project};
use crate::scanner::cluster::cluster_embeddings;
use crate::scanner::detector::FaceDetector;
use crate::scanner::embedder::FaceEmbedder;
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

        // Check models exist
        let det_path = crate::models::detector_model_path()?;
        let emb_path = crate::models::embedder_model_path()?;
        if !det_path.exists() || !emb_path.exists() {
            return Err(CullingError::ModelNotFound(
                "Models not found. Please download buffalo_l models to ~/.culling/models/"
                    .to_string(),
            ));
        }

        // Initialize detector and embedder
        let mut detector = FaceDetector::new(&det_path)?;
        let mut embedder = FaceEmbedder::new(&emb_path)?;

        let total = project.photos.len();

        // Phase 1: Detect faces and compute embeddings
        for i in 0..total {
            let photo_path = project.photos[i].path.clone();

            let detected = detector.detect(&photo_path, 0.5, 80)?;

            let mut face_detections = Vec::new();
            for face in &detected {
                match embedder.embed(&photo_path, &face.keypoints) {
                    Ok(embedding) => {
                        face_detections.push(FaceDetection {
                            bbox: face.bbox,
                            confidence: face.confidence,
                            embedding,
                            cluster_id: None,
                        });
                    }
                    Err(e) => {
                        eprintln!("Warning: failed to embed face: {}", e);
                    }
                }
            }

            project.photos[i].faces = face_detections;

            let _ = on_progress.send(ProgressPayload {
                current: i + 1,
                total,
                message: "Detecting faces...".into(),
            });
        }

        // Phase 2: Cluster all face embeddings
        let all_embeddings: Vec<&[f32]> = project
            .photos
            .iter()
            .flat_map(|p| p.faces.iter().map(|f| f.embedding.as_slice()))
            .collect();

        if !all_embeddings.is_empty() {
            let labels = cluster_embeddings(&all_embeddings, 0.75, 2)?;

            // Assign cluster IDs back to faces
            let mut label_idx = 0;
            for photo in &mut project.photos {
                for face in &mut photo.faces {
                    face.cluster_id = labels[label_idx];
                    label_idx += 1;
                }
            }

            // Build cluster summaries
            build_cluster_summaries(&mut project);
        }

        project.save()?;
        Ok(project)
    })
    .await
    .map_err(|e| CullingError::Other(format!("Task panicked: {}", e)))?
}

/// Build `Cluster` summaries from face detections across all photos.
///
/// For each unique cluster ID, finds the highest-confidence face as the
/// representative, counts the number of photos containing that cluster,
/// and assigns an auto-numbered label ("Person 1", "Person 2", etc.).
fn build_cluster_summaries(project: &mut Project) {
    // Collect per-cluster info: best confidence, representative photo/bbox, photo set
    struct ClusterInfo {
        best_confidence: f32,
        best_photo: PathBuf,
        best_bbox: [f32; 4],
        photo_paths: std::collections::HashSet<PathBuf>,
    }

    let mut cluster_map: HashMap<usize, ClusterInfo> = HashMap::new();

    for photo in &project.photos {
        for face in &photo.faces {
            if let Some(cid) = face.cluster_id {
                let entry = cluster_map.entry(cid).or_insert_with(|| ClusterInfo {
                    best_confidence: 0.0,
                    best_photo: PathBuf::new(),
                    best_bbox: [0.0; 4],
                    photo_paths: std::collections::HashSet::new(),
                });

                entry.photo_paths.insert(photo.path.clone());

                if face.confidence > entry.best_confidence {
                    entry.best_confidence = face.confidence;
                    entry.best_photo = photo.path.clone();
                    entry.best_bbox = face.bbox;
                }
            }
        }
    }

    // Sort cluster IDs for deterministic ordering
    let mut cluster_ids: Vec<usize> = cluster_map.keys().copied().collect();
    cluster_ids.sort();

    project.clusters = cluster_ids
        .into_iter()
        .enumerate()
        .map(|(label_num, cid)| {
            let info = &cluster_map[&cid];
            Cluster {
                id: cid,
                label: format!("Person {}", label_num + 1),
                representative_photo: info.best_photo.clone(),
                representative_bbox: info.best_bbox,
                photo_count: info.photo_paths.len(),
            }
        })
        .collect();
}

#[tauri::command]
pub async fn check_models() -> Result<bool, CullingError> {
    crate::models::models_available()
}

#[tauri::command]
pub async fn start_auto_grade(
    project_id: String,
    on_progress: Channel<ProgressPayload>,
) -> Result<Project, CullingError> {
    tokio::task::spawn_blocking(move || {
        let mut project = Project::load(&project_id)?;
        let total = project.photos.len();

        for i in 0..total {
            // Only auto-grade photos that are currently Ungraded
            if project.photos[i].grade != Grade::Ungraded {
                let _ = on_progress.send(ProgressPayload {
                    current: i + 1,
                    total,
                    message: "Grading photos...".into(),
                });
                continue;
            }

            let photo_path = project.photos[i].path.clone();

            // Run heuristics
            let heuristic = match crate::grader::heuristics::analyze(&photo_path) {
                Ok(result) => result,
                Err(e) => {
                    eprintln!(
                        "Warning: heuristic analysis failed for {:?}: {}",
                        photo_path, e
                    );
                    let _ = on_progress.send(ProgressPayload {
                        current: i + 1,
                        total,
                        message: "Grading photos...".into(),
                    });
                    continue;
                }
            };

            project.photos[i].sharpness_score = Some(heuristic.sharpness);

            if heuristic.is_bad {
                project.photos[i].grade = Grade::Bad;
                project.photos[i].grade_source = GradeSource::Auto;
            } else {
                // Run aesthetic scoring
                let aesthetic = match crate::grader::aesthetic::score_aesthetic(&photo_path) {
                    Ok(score) => score,
                    Err(e) => {
                        eprintln!(
                            "Warning: aesthetic scoring failed for {:?}: {}",
                            photo_path, e
                        );
                        5.0 // Default to neutral score on failure
                    }
                };
                project.photos[i].aesthetic_score = Some(aesthetic);

                if aesthetic >= 5.0 {
                    project.photos[i].grade = Grade::Good;
                } else {
                    project.photos[i].grade = Grade::Ok;
                }
                project.photos[i].grade_source = GradeSource::Auto;
            }

            let _ = on_progress.send(ProgressPayload {
                current: i + 1,
                total,
                message: "Grading photos...".into(),
            });
        }

        project.save()?;
        Ok(project)
    })
    .await
    .map_err(|e| CullingError::Other(format!("Task panicked: {}", e)))?
}

#[tauri::command]
pub async fn generate_thumbnails(
    project_id: String,
    on_progress: Channel<ProgressPayload>,
) -> Result<usize, CullingError> {
    tokio::task::spawn_blocking(move || {
        let project = Project::load(&project_id)?;
        let photos: Vec<(PathBuf, String)> = project
            .photos
            .iter()
            .map(|p| (p.path.clone(), p.filename.clone()))
            .collect();

        let count =
            thumbnailer::generate_all_thumbnails(&photos, &project_id, |current, total| {
                let _ = on_progress.send(ProgressPayload {
                    current,
                    total,
                    message: "Generating thumbnails...".into(),
                });
            })?;

        Ok(count)
    })
    .await
    .map_err(|e| CullingError::Other(format!("Task panicked: {}", e)))?
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
