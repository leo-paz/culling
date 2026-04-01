//! SCRFD face detection using ONNX Runtime.
//! Loads det_10g.onnx from InsightFace buffalo_l model pack.

use image::{DynamicImage, GenericImageView, RgbImage};
use ndarray::Array4;
use ort::session::Session;
use ort::value::TensorRef;
use std::path::Path;

/// A detected face with bounding box, confidence, and 5 keypoints.
#[derive(Debug, Clone)]
pub struct DetectedFace {
    /// Bounding box [x1, y1, x2, y2] in original image coordinates
    pub bbox: [f32; 4],
    /// Detection confidence score
    pub confidence: f32,
    /// 5 facial keypoints: left_eye, right_eye, nose, left_mouth, right_mouth
    /// Each is [x, y] in original image coordinates
    pub keypoints: [[f32; 2]; 5],
}

pub struct FaceDetector {
    session: Session,
    input_size: (u32, u32), // (width, height) = (640, 640)
}

impl FaceDetector {
    /// Create a new face detector from an ONNX model file.
    pub fn new(model_path: &Path) -> Result<Self, String> {
        let session = Session::builder()
            .map_err(|e| format!("Failed to create session builder: {}", e))?
            .commit_from_file(model_path)
            .map_err(|e| format!("Failed to load model: {}", e))?;

        Ok(Self {
            session,
            input_size: (640, 640),
        })
    }

    /// Detect faces in an image.
    pub fn detect(
        &mut self,
        image_path: &Path,
        confidence_threshold: f32,
        min_face_size: u32,
    ) -> Result<Vec<DetectedFace>, String> {
        // Load image
        let img =
            image::open(image_path).map_err(|e| format!("Failed to open image: {}", e))?;

        let (orig_w, orig_h) = img.dimensions();

        // Preprocess: letterbox resize to 640x640
        let (input_tensor, scale, pad_x, pad_y) = preprocess(&img, self.input_size)?;

        // Create a TensorRef from the ndarray for zero-copy input
        let input_ref = TensorRef::from_array_view(input_tensor.view())
            .map_err(|e| format!("Failed to create tensor ref: {}", e))?;

        // Run inference
        let input_size = self.input_size;
        let outputs = self
            .session
            .run(ort::inputs![input_ref])
            .map_err(|e| format!("Inference failed: {}", e))?;

        // Post-process: decode detections
        let mut faces =
            postprocess(&outputs, input_size, scale, pad_x, pad_y, confidence_threshold)?;

        // Filter by minimum face size
        faces.retain(|f| {
            let w = f.bbox[2] - f.bbox[0];
            let h = f.bbox[3] - f.bbox[1];
            w.min(h) >= min_face_size as f32
        });

        // Clip to image bounds
        for face in &mut faces {
            face.bbox[0] = face.bbox[0].max(0.0).min(orig_w as f32);
            face.bbox[1] = face.bbox[1].max(0.0).min(orig_h as f32);
            face.bbox[2] = face.bbox[2].max(0.0).min(orig_w as f32);
            face.bbox[3] = face.bbox[3].max(0.0).min(orig_h as f32);
        }

        // Apply NMS
        let faces = nms(faces, 0.4);

        Ok(faces)
    }
}

/// Preprocess image: letterbox resize to input_size, normalize.
/// Returns (tensor, scale, pad_x, pad_y)
fn preprocess(
    img: &DynamicImage,
    input_size: (u32, u32),
) -> Result<(Array4<f32>, f32, f32, f32), String> {
    let (orig_w, orig_h) = img.dimensions();
    let (target_w, target_h) = input_size;

    // Calculate scale to fit image within target size (preserving aspect ratio)
    let scale = (target_w as f32 / orig_w as f32).min(target_h as f32 / orig_h as f32);
    let new_w = (orig_w as f32 * scale) as u32;
    let new_h = (orig_h as f32 * scale) as u32;

    // Resize (preserving aspect ratio)
    let resized = img.resize(new_w, new_h, image::imageops::FilterType::Triangle);
    let rgb = resized.to_rgb8();

    // Create padded image (black background)
    let mut padded = RgbImage::new(target_w, target_h);
    let pad_x = (target_w - new_w) / 2;
    let pad_y = (target_h - new_h) / 2;

    // Copy resized image onto padded canvas
    for y in 0..new_h {
        for x in 0..new_w {
            let pixel = rgb.get_pixel(x, y);
            padded.put_pixel(x + pad_x, y + pad_y, *pixel);
        }
    }

    // Convert to NCHW float32 tensor with normalization: (pixel - 127.5) / 128.0
    let mut tensor = Array4::<f32>::zeros((1, 3, target_h as usize, target_w as usize));
    for y in 0..target_h as usize {
        for x in 0..target_w as usize {
            let pixel = padded.get_pixel(x as u32, y as u32);
            tensor[[0, 0, y, x]] = (pixel[0] as f32 - 127.5) / 128.0; // R
            tensor[[0, 1, y, x]] = (pixel[1] as f32 - 127.5) / 128.0; // G
            tensor[[0, 2, y, x]] = (pixel[2] as f32 - 127.5) / 128.0; // B
        }
    }

    Ok((tensor, scale, pad_x as f32, pad_y as f32))
}

/// Decode SCRFD outputs into face detections.
///
/// SCRFD det_10g produces 9 output tensors for 3 feature map strides (8, 16, 32).
/// For each stride there are 3 tensors: scores, bbox_preds, kps_preds.
/// We classify them by shape: scores have last dim 1, bbox have last dim 4, kps have last dim 10.
fn postprocess(
    outputs: &ort::session::SessionOutputs<'_>,
    input_size: (u32, u32),
    scale: f32,
    pad_x: f32,
    pad_y: f32,
    confidence_threshold: f32,
) -> Result<Vec<DetectedFace>, String> {
    let feat_stride_fpn: [usize; 3] = [8, 16, 32];
    let num_anchors: usize = 2;
    let (input_h, input_w) = (input_size.1 as usize, input_size.0 as usize);

    // Collect output tensor shapes and classify them.
    // Each stride produces 3 tensors. We identify them by their last dimension:
    //   scores: last_dim == 1
    //   bbox:   last_dim == 4
    //   kps:    last_dim == 10
    // Then we group by the spatial dimension (second dim) to match strides.
    struct OutputInfo {
        index: usize,
        num_anchors_total: usize,
        #[allow(dead_code)]
        last_dim: usize,
    }

    let num_outputs = outputs.len();
    if num_outputs != 9 {
        return Err(format!(
            "Expected 9 output tensors from SCRFD, got {}",
            num_outputs
        ));
    }

    let mut score_outputs: Vec<OutputInfo> = Vec::new();
    let mut bbox_outputs: Vec<OutputInfo> = Vec::new();
    let mut kps_outputs: Vec<OutputInfo> = Vec::new();

    for i in 0..num_outputs {
        let (shape, _data) = outputs[i]
            .try_extract_tensor::<f32>()
            .map_err(|e| format!("Failed to extract output {}: {}", i, e))?;
        let dims: &[i64] = &**shape;
        if dims.len() < 2 {
            return Err(format!("Output {} has unexpected shape {:?}", i, dims));
        }
        let last_dim = *dims.last().unwrap() as usize;
        let second_dim = dims[1] as usize;

        let info = OutputInfo {
            index: i,
            num_anchors_total: second_dim,
            last_dim,
        };

        match last_dim {
            1 => score_outputs.push(info),
            4 => bbox_outputs.push(info),
            10 => kps_outputs.push(info),
            _ => {
                return Err(format!(
                    "Output {} has unexpected last dimension {} (shape {:?})",
                    i, last_dim, dims
                ))
            }
        }
    }

    if score_outputs.len() != 3 || bbox_outputs.len() != 3 || kps_outputs.len() != 3 {
        return Err(format!(
            "Expected 3 score, 3 bbox, 3 kps outputs; got {}, {}, {}",
            score_outputs.len(),
            bbox_outputs.len(),
            kps_outputs.len()
        ));
    }

    // Sort each group by num_anchors_total descending
    // (stride 8 has most anchors, stride 32 has fewest)
    score_outputs.sort_by(|a, b| b.num_anchors_total.cmp(&a.num_anchors_total));
    bbox_outputs.sort_by(|a, b| b.num_anchors_total.cmp(&a.num_anchors_total));
    kps_outputs.sort_by(|a, b| b.num_anchors_total.cmp(&a.num_anchors_total));

    let mut faces = Vec::new();

    for (stride_idx, &stride) in feat_stride_fpn.iter().enumerate() {
        let score_info = &score_outputs[stride_idx];
        let bbox_info = &bbox_outputs[stride_idx];
        let kps_info = &kps_outputs[stride_idx];

        let (_shape, scores_data) = outputs[score_info.index]
            .try_extract_tensor::<f32>()
            .map_err(|e| format!("Failed to extract scores: {}", e))?;
        let (_shape, bbox_data) = outputs[bbox_info.index]
            .try_extract_tensor::<f32>()
            .map_err(|e| format!("Failed to extract bboxes: {}", e))?;
        let (_shape, kps_data) = outputs[kps_info.index]
            .try_extract_tensor::<f32>()
            .map_err(|e| format!("Failed to extract keypoints: {}", e))?;

        let feat_h = input_h / stride;
        let feat_w = input_w / stride;

        // Generate anchor centers and decode detections
        for y in 0..feat_h {
            for x in 0..feat_w {
                let anchor_cx = (x as f32 + 0.5) * stride as f32;
                let anchor_cy = (y as f32 + 0.5) * stride as f32;

                for a in 0..num_anchors {
                    let pos = y * feat_w * num_anchors + x * num_anchors + a;

                    // Score (apply sigmoid)
                    let score = sigmoid(scores_data[pos]);
                    if score < confidence_threshold {
                        continue;
                    }

                    // Decode bbox: [left, top, right, bottom] distances from anchor
                    let bbox_offset = pos * 4;
                    let left = bbox_data[bbox_offset] * stride as f32;
                    let top = bbox_data[bbox_offset + 1] * stride as f32;
                    let right = bbox_data[bbox_offset + 2] * stride as f32;
                    let bottom = bbox_data[bbox_offset + 3] * stride as f32;

                    let x1 = anchor_cx - left;
                    let y1 = anchor_cy - top;
                    let x2 = anchor_cx + right;
                    let y2 = anchor_cy + bottom;

                    // Map back to original image coordinates
                    let x1 = (x1 - pad_x) / scale;
                    let y1 = (y1 - pad_y) / scale;
                    let x2 = (x2 - pad_x) / scale;
                    let y2 = (y2 - pad_y) / scale;

                    // Decode keypoints
                    let mut keypoints = [[0.0f32; 2]; 5];
                    let kps_offset = pos * 10;
                    for k in 0..5 {
                        let kp_x = (anchor_cx
                            + kps_data[kps_offset + k * 2] * stride as f32
                            - pad_x)
                            / scale;
                        let kp_y = (anchor_cy
                            + kps_data[kps_offset + k * 2 + 1] * stride as f32
                            - pad_y)
                            / scale;
                        keypoints[k] = [kp_x, kp_y];
                    }

                    faces.push(DetectedFace {
                        bbox: [x1, y1, x2, y2],
                        confidence: score,
                        keypoints,
                    });
                }
            }
        }
    }

    Ok(faces)
}

fn sigmoid(x: f32) -> f32 {
    1.0 / (1.0 + (-x).exp())
}

/// Non-maximum suppression: keep highest-confidence faces and suppress overlapping ones.
fn nms(mut faces: Vec<DetectedFace>, iou_threshold: f32) -> Vec<DetectedFace> {
    faces.sort_by(|a, b| {
        b.confidence
            .partial_cmp(&a.confidence)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let mut keep = Vec::new();
    let mut suppressed = vec![false; faces.len()];

    for i in 0..faces.len() {
        if suppressed[i] {
            continue;
        }
        keep.push(faces[i].clone());

        for j in (i + 1)..faces.len() {
            if suppressed[j] {
                continue;
            }
            if iou(&faces[i].bbox, &faces[j].bbox) > iou_threshold {
                suppressed[j] = true;
            }
        }
    }

    keep
}

/// Compute intersection over union of two bounding boxes.
fn iou(a: &[f32; 4], b: &[f32; 4]) -> f32 {
    let x1 = a[0].max(b[0]);
    let y1 = a[1].max(b[1]);
    let x2 = a[2].min(b[2]);
    let y2 = a[3].min(b[3]);

    let inter = (x2 - x1).max(0.0) * (y2 - y1).max(0.0);
    let area_a = (a[2] - a[0]) * (a[3] - a[1]);
    let area_b = (b[2] - b[0]) * (b[3] - b[1]);

    inter / (area_a + area_b - inter + 1e-6)
}
