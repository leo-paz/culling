//! Core processing pipelines for face detection, grading, and export.
//!
//! This module contains the business logic that orchestrates detection,
//! embedding, clustering, and grading. It is independent of Tauri and
//! can be tested and used without the IPC layer.

use crate::config::Config;
use crate::error::CullingError;
use crate::models;
use crate::project::{Cluster, FaceDetection, Grade, GradeSource, Project};
use crate::scanner::cluster::cluster_embeddings;
use crate::scanner::detector::FaceDetector;
use crate::scanner::embedder::FaceEmbedder;

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

/// Callback type for reporting progress from pipelines.
pub type ProgressFn = Box<dyn Fn(usize, usize, &str) + Send + Sync>;

/// Run face detection and clustering on all photos in a project.
pub fn run_face_detection(
    project: &mut Project,
    config: &Config,
    on_progress: &ProgressFn,
) -> Result<(), CullingError> {
    let det_path = models::detector_model_path()?;
    let emb_path = models::embedder_model_path()?;

    if !det_path.exists() || !emb_path.exists() {
        return Err(CullingError::ModelNotFound(
            "Models not found. Download buffalo_l models to ~/.culling/models/".into(),
        ));
    }

    let mut detector = FaceDetector::new(&det_path)?;
    let mut embedder = FaceEmbedder::new(&emb_path)?;

    let total = project.photos.len();

    // Phase 1: Detect and embed
    for i in 0..total {
        let photo_path = project.photos[i].path.clone();
        let detected = detector.detect(
            &photo_path,
            config.detection.min_confidence,
            config.detection.min_face_size,
        )?;

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
                    eprintln!(
                        "Warning: failed to embed face in {}: {}",
                        photo_path.display(),
                        e
                    );
                }
            }
        }

        project.photos[i].faces = face_detections;
        on_progress(i + 1, total, "Detecting faces...");
    }

    // Phase 2: Cluster
    let all_embeddings: Vec<&[f32]> = project
        .photos
        .iter()
        .flat_map(|p| p.faces.iter().map(|f| f.embedding.as_slice()))
        .collect();

    if !all_embeddings.is_empty() {
        let labels = cluster_embeddings(
            &all_embeddings,
            config.clustering.eps,
            config.clustering.min_samples,
        )?;

        let mut label_idx = 0;
        for photo in &mut project.photos {
            for face in &mut photo.faces {
                face.cluster_id = labels[label_idx];
                label_idx += 1;
            }
        }

        build_cluster_summaries(project);
    }

    project.save()?;
    Ok(())
}

/// Run auto-grading on all ungraded photos.
pub fn run_auto_grade(
    project: &mut Project,
    config: &Config,
    on_progress: &ProgressFn,
) -> Result<(), CullingError> {
    let total = project.photos.len();

    for i in 0..total {
        if project.photos[i].grade != Grade::Ungraded {
            on_progress(i + 1, total, "Grading photos...");
            continue;
        }

        let photo_path = project.photos[i].path.clone();

        let heuristic =
            match crate::grader::heuristics::analyze(&photo_path, &config.grading) {
                Ok(result) => result,
                Err(e) => {
                    eprintln!(
                        "Warning: heuristic analysis failed for {:?}: {}",
                        photo_path, e
                    );
                    on_progress(i + 1, total, "Grading photos...");
                    continue;
                }
            };

        project.photos[i].sharpness_score = Some(heuristic.sharpness);

        if heuristic.is_bad {
            project.photos[i].grade = Grade::Bad;
            project.photos[i].grade_source = GradeSource::Auto;
        } else {
            let aesthetic = match crate::grader::aesthetic::score_aesthetic(&photo_path) {
                Ok(score) => score,
                Err(e) => {
                    eprintln!(
                        "Warning: aesthetic scoring failed for {:?}: {}",
                        photo_path, e
                    );
                    5.0
                }
            };
            project.photos[i].aesthetic_score = Some(aesthetic);

            if aesthetic >= config.grading.aesthetic_good_threshold {
                project.photos[i].grade = Grade::Good;
            } else {
                project.photos[i].grade = Grade::Ok;
            }
            project.photos[i].grade_source = GradeSource::Auto;
        }

        on_progress(i + 1, total, "Grading photos...");
    }

    project.save()?;
    Ok(())
}

/// Build cluster summaries from face detections across all photos.
pub fn build_cluster_summaries(project: &mut Project) {
    struct ClusterInfo {
        best_confidence: f32,
        best_photo: PathBuf,
        best_bbox: [f32; 4],
        photo_paths: HashSet<PathBuf>,
    }

    let mut cluster_map: HashMap<usize, ClusterInfo> = HashMap::new();

    for photo in &project.photos {
        for face in &photo.faces {
            if let Some(cid) = face.cluster_id {
                let entry = cluster_map.entry(cid).or_insert_with(|| ClusterInfo {
                    best_confidence: 0.0,
                    best_photo: PathBuf::new(),
                    best_bbox: [0.0; 4],
                    photo_paths: HashSet::new(),
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
