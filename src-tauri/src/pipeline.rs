//! Core processing pipelines for enrichment, face detection, grading, and export.
//!
//! This module contains the business logic that orchestrates scanning,
//! grading, detection, embedding, and clustering. It is independent of Tauri
//! and can be tested and used without the IPC layer.

use crate::config::Config;
use crate::error::CullingError;
use crate::models;
use crate::project::{Cluster, FaceDetection, Grade, GradeSource, Photo, Project};
use crate::scanner::cluster::cluster_embeddings;
use crate::scanner::detector::FaceDetector;
use crate::scanner::embedder::FaceEmbedder;

use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};
use std::io::Read;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

const IMAGE_EXTENSIONS: &[&str] = &["jpg", "jpeg", "png", "heic", "tif", "tiff"];

fn is_image_file(path: &std::path::Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| IMAGE_EXTENSIONS.contains(&ext.to_lowercase().as_str()))
        .unwrap_or(false)
}

/// Compute a fast content hash (first 64KB) for change detection.
/// Uses DefaultHasher to avoid adding a crypto dependency.
pub fn compute_content_hash(path: &std::path::Path) -> Result<String, CullingError> {
    let mut file = std::fs::File::open(path)?;
    let mut buffer = vec![0u8; 65536];
    let bytes_read = file.read(&mut buffer)?;
    buffer.truncate(bytes_read);

    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    buffer.hash(&mut hasher);
    Ok(format!("{:016x}", hasher.finish()))
}

/// Progress callback that takes (stage_name, current, total).
pub type EnrichmentProgressFn = Box<dyn Fn(&str, usize, usize) + Send + Sync>;

/// Callback emitted when a single photo's grade is determined.
/// Takes (photo_path, grade, grade_source).
pub type PhotoGradedFn = Box<dyn Fn(&str, &str, &str) + Send + Sync>;

/// Scan source folder and sync with project state.
/// Returns the number of new photos added.
pub fn scan_for_changes(project: &mut Project) -> Result<usize, CullingError> {
    let source_dir = project.source_dir.clone();
    if !source_dir.exists() {
        return Ok(0);
    }

    // Get current files in folder
    let current_files: HashSet<PathBuf> = walkdir::WalkDir::new(&source_dir)
        .max_depth(1)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file() && is_image_file(e.path()))
        .map(|e| e.path().to_path_buf())
        .collect();

    // Find new files (in folder but not in project)
    let existing_paths: HashSet<PathBuf> = project.photos.iter().map(|p| p.path.clone()).collect();
    let mut new_count = 0;

    for file_path in &current_files {
        if !existing_paths.contains(file_path) {
            let filename = file_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown")
                .to_string();
            let hash = compute_content_hash(file_path).ok();
            project.photos.push(Photo {
                path: file_path.clone(),
                filename,
                grade: Grade::default(),
                grade_source: GradeSource::default(),
                faces: Vec::new(),
                aesthetic_score: None,
                sharpness_score: None,
                grade_reason: None,
                content_hash: hash,
                graded_at: None,
                faces_detected_at: None,
            });
            new_count += 1;
        }
    }

    // Check for changed files (content_hash differs)
    for photo in &mut project.photos {
        if !current_files.contains(&photo.path) {
            continue; // Missing file, skip
        }
        if let Ok(new_hash) = compute_content_hash(&photo.path) {
            if photo.content_hash.as_deref() != Some(&new_hash) {
                // File changed — reset processing timestamps
                photo.content_hash = Some(new_hash);
                if photo.grade_source != GradeSource::Manual {
                    photo.graded_at = None;
                    photo.grade = Grade::default();
                    photo.grade_source = GradeSource::default();
                    photo.aesthetic_score = None;
                    photo.sharpness_score = None;
                }
                photo.faces_detected_at = None;
                photo.faces.clear();
            }
        }
    }

    // Sort photos by filename
    project.photos.sort_by(|a, b| a.filename.cmp(&b.filename));

    Ok(new_count)
}

/// Run the full enrichment pipeline on a project.
/// Only processes photos that need it (incremental).
pub fn run_enrichment(
    project: &mut Project,
    config: &Config,
    on_progress: &EnrichmentProgressFn,
    on_photo_graded: &PhotoGradedFn,
) -> Result<(), CullingError> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    // Stage 1: Auto-grade ungraded photos
    let needs_grading: Vec<usize> = project
        .photos
        .iter()
        .enumerate()
        .filter(|(_, p)| p.graded_at.is_none() && p.grade_source != GradeSource::Manual)
        .map(|(i, _)| i)
        .collect();

    // Try to use pre-generated thumbnails for grading (much faster than decoding full images).
    // Thumbnails are ~300px and already on disk from the thumbnail generation stage.
    let thumb_dir = crate::thumbnailer::thumbnail_dir(&project.id)?;

    let grade_total = needs_grading.len();
    for (progress_idx, photo_idx) in needs_grading.iter().enumerate() {
        let photo = &project.photos[*photo_idx];
        let photo_path = photo.path.clone();
        let filename = photo.filename.clone();

        // Prefer thumbnail (fast: ~300px already decoded) over full image (slow: 26MP decode)
        let thumb_path = thumb_dir.join(&filename);
        let img = if thumb_path.exists() {
            match image::open(&thumb_path) {
                Ok(img) => img,
                Err(_) => match image::open(&photo_path) {
                    Ok(img) => img.resize(800, 800, image::imageops::FilterType::Triangle),
                    Err(e) => {
                        eprintln!("Warning: failed to open {:?}: {}", photo_path, e);
                        project.photos[*photo_idx].graded_at = Some(now);
                        on_progress("grading", progress_idx + 1, grade_total);
                        continue;
                    }
                },
            }
        } else {
            match image::open(&photo_path) {
                Ok(img) => img.resize(800, 800, image::imageops::FilterType::Triangle),
                Err(e) => {
                    eprintln!("Warning: failed to open {:?}: {}", photo_path, e);
                    project.photos[*photo_idx].graded_at = Some(now);
                    on_progress("grading", progress_idx + 1, grade_total);
                    continue;
                }
            }
        };

        // Run heuristics and aesthetic on the (small) image
        let heuristic = crate::grader::heuristics::analyze_image(&img, &config.grading)?;
        project.photos[*photo_idx].sharpness_score = Some(heuristic.sharpness);

        let (grade, reason) = if heuristic.is_bad {
            let mut reasons = Vec::new();
            if heuristic.is_blurry {
                reasons.push(format!("Blurry (sharpness: {:.0})", heuristic.sharpness));
            }
            if heuristic.is_overexposed {
                reasons.push("Overexposed".to_string());
            }
            if heuristic.is_underexposed {
                reasons.push("Underexposed".to_string());
            }
            (Grade::Bad, reasons.join(", "))
        } else {
            let aesthetic = crate::grader::aesthetic::score_aesthetic_image(&img);
            project.photos[*photo_idx].aesthetic_score = Some(aesthetic);
            if aesthetic >= config.grading.aesthetic_good_threshold {
                (Grade::Good, format!("Aesthetic score: {:.1}/10", aesthetic))
            } else {
                (Grade::Ok, format!("Aesthetic score: {:.1}/10", aesthetic))
            }
        };

        project.photos[*photo_idx].grade = grade;
        project.photos[*photo_idx].grade_source = GradeSource::Auto;
        project.photos[*photo_idx].grade_reason = Some(reason.clone());
        project.photos[*photo_idx].graded_at = Some(now);

        // Emit per-photo grade event so the UI updates in real-time
        let grade_str = match grade {
            Grade::Bad => "Bad",
            Grade::Ok => "Ok",
            Grade::Good => "Good",
            Grade::Ungraded => "Ungraded",
        };
        on_photo_graded(
            &photo_path.to_string_lossy(),
            grade_str,
            "Auto",
        );

        on_progress("grading", progress_idx + 1, grade_total);

        // Save every 10 photos for crash recovery
        if (progress_idx + 1) % 5 == 0 {
            let _ = project.save();
        }
    }
    if grade_total > 0 {
        project.save()?;
    }

    // Stage 2: Face detection — auto-download models if needed
    match crate::models::ensure_models(|msg, current, total| {
        on_progress(msg, current as usize, total as usize);
    }) {
        Ok(_) => {}
        Err(e) => {
            // Download failed (offline, etc.) — skip face detection silently
            eprintln!("Model download skipped: {}", e);
        }
    }

    let det_path = models::detector_model_path()?;
    let emb_path = models::embedder_model_path()?;

    eprintln!("[enrichment] Stage 2: det_path={:?} exists={}, emb_path={:?} exists={}", det_path, det_path.exists(), emb_path, emb_path.exists());

    if det_path.exists() && emb_path.exists() {
        let needs_detection: Vec<usize> = project
            .photos
            .iter()
            .enumerate()
            .filter(|(_, p)| p.faces_detected_at.is_none())
            .map(|(i, _)| i)
            .collect();

        eprintln!("[enrichment] Need to detect faces in {} photos", needs_detection.len());

        if !needs_detection.is_empty() {
            eprintln!("[enrichment] Loading SCRFD detector...");
            let mut detector = match FaceDetector::new(&det_path) {
                Ok(d) => { eprintln!("[enrichment] SCRFD loaded OK"); d },
                Err(e) => { eprintln!("[enrichment] SCRFD failed: {}", e); return Err(e); }
            };
            eprintln!("[enrichment] Loading ArcFace embedder...");
            let mut embedder = match FaceEmbedder::new(&emb_path) {
                Ok(e) => { eprintln!("[enrichment] ArcFace loaded OK"); e },
                Err(e) => { eprintln!("[enrichment] ArcFace failed: {}", e); return Err(e); }
            };

            // Use 1280px working copies generated during the thumbnail stage.
            // These are ~200KB JPEGs (vs 25MB originals) but have enough resolution
            // for high-quality face detection and embedding.
            let work_dir = crate::thumbnailer::working_dir(&project.id)?;

            let detect_total = needs_detection.len();
            for (progress_idx, photo_idx) in needs_detection.iter().enumerate() {
                let filename = project.photos[*photo_idx].filename.clone();
                let photo_path = project.photos[*photo_idx].path.clone();

                let t0 = std::time::Instant::now();

                // Use 1280px working copy; fall back to full image if missing
                let detect_path = {
                    let work = work_dir.join(&filename);
                    if work.exists() { work } else { photo_path.clone() }
                };

                eprintln!("[enrichment] Processing {}/{}: {} (path: {:?})",
                    progress_idx + 1, detect_total, filename, detect_path);

                let detected = match detector.detect(
                    &detect_path,
                    config.detection.min_confidence,
                    config.detection.min_face_size,
                ) {
                    Ok(d) => d,
                    Err(e) => {
                        eprintln!("Warning: face detection failed for {}: {}", filename, e);
                        project.photos[*photo_idx].faces_detected_at = Some(now);
                        on_progress("faces", progress_idx + 1, detect_total);
                        continue;
                    }
                };

                // Cap faces per photo — more than 30 is almost certainly false positives
                let detected = if detected.len() > 30 {
                    eprintln!("[enrichment] {} has {} detections — capping to top 30 by confidence",
                        filename, detected.len());
                    let mut sorted = detected;
                    sorted.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap_or(std::cmp::Ordering::Equal));
                    sorted.truncate(30);
                    sorted
                } else {
                    detected
                };

                // Use the same image for embedding (coordinates match detection output)
                let mut face_detections = Vec::new();
                for face in &detected {
                    if let Ok(embedding) = embedder.embed(&detect_path, &face.keypoints) {
                        face_detections.push(FaceDetection {
                            bbox: face.bbox,
                            confidence: face.confidence,
                            embedding,
                            cluster_id: None,
                        });
                    }
                }

                project.photos[*photo_idx].faces = face_detections;
                project.photos[*photo_idx].faces_detected_at = Some(now);

                eprintln!("[enrichment] {} done in {:.1}s — {} faces",
                    filename, t0.elapsed().as_secs_f64(), project.photos[*photo_idx].faces.len());

                on_progress("faces", progress_idx + 1, detect_total);

                // Save every 5 photos for crash recovery + polling visibility
                if (progress_idx + 1) % 5 == 0 {
                    let _ = project.save();
                }
            }

            // Re-cluster ALL faces (not just new ones) since clusters depend on the full set
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
        }
    }

    Ok(())
}

/// Special cluster IDs for virtual categories.
/// Real person clusters use IDs 0..N from DBSCAN.
/// These use high IDs to avoid collision.
pub const CLUSTER_ID_GROUPS: usize = usize::MAX - 1;
pub const CLUSTER_ID_NO_PEOPLE: usize = usize::MAX - 2;

/// Build cluster summaries from face detections across all photos.
/// Includes virtual clusters for "Groups" (multiple faces) and "No People" (no faces).
pub fn build_cluster_summaries(project: &mut Project) {
    struct ClusterInfo {
        best_confidence: f32,
        best_photo: PathBuf,
        best_bbox: [f32; 4],
        photo_paths: HashSet<PathBuf>,
    }

    let mut cluster_map: HashMap<usize, ClusterInfo> = HashMap::new();
    let mut group_photos: HashSet<PathBuf> = HashSet::new();
    let mut no_people_photos: HashSet<PathBuf> = HashSet::new();

    for photo in &project.photos {
        if photo.faces.is_empty() {
            // Only count as "no people" if face detection has actually run
            if photo.faces_detected_at.is_some() {
                no_people_photos.insert(photo.path.clone());
            }
            continue;
        }

        // Track if this photo has multiple different people (group shot)
        let unique_clusters: HashSet<_> = photo.faces.iter()
            .filter_map(|f| f.cluster_id)
            .collect();
        if unique_clusters.len() > 1 {
            group_photos.insert(photo.path.clone());
        }

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

    let mut clusters: Vec<Cluster> = cluster_ids
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

    // Add virtual "Groups" cluster
    if !group_photos.is_empty() {
        let first_group = group_photos.iter().next().cloned().unwrap_or_default();
        clusters.push(Cluster {
            id: CLUSTER_ID_GROUPS,
            label: "Groups".to_string(),
            representative_photo: first_group,
            representative_bbox: [0.0; 4],
            photo_count: group_photos.len(),
        });
    }

    // Add virtual "No People" cluster
    if !no_people_photos.is_empty() {
        let first_no_people = no_people_photos.iter().next().cloned().unwrap_or_default();
        clusters.push(Cluster {
            id: CLUSTER_ID_NO_PEOPLE,
            label: "Landscapes".to_string(),
            representative_photo: first_no_people,
            representative_bbox: [0.0; 4],
            photo_count: no_people_photos.len(),
        });
    }

    project.clusters = clusters;
}
