//! ArcFace face embedding using ONNX Runtime.
//! Performs face alignment (similarity transform) then runs w600k_r50.onnx.

use crate::error::CullingError;
use image::{DynamicImage, GenericImageView, Rgb, RgbImage};
use nalgebra::{DMatrix, DVector};
use ndarray::Array4;
use ort::session::Session;
use ort::value::TensorRef;
use std::path::Path;

/// ArcFace canonical template landmarks for 112x112 aligned face.
/// Order: left_eye, right_eye, nose, left_mouth, right_mouth.
const ARCFACE_TEMPLATE: [[f64; 2]; 5] = [
    [38.2946, 51.6963],
    [73.5318, 51.5014],
    [56.0252, 71.7366],
    [41.5493, 92.3655],
    [70.7299, 92.2041],
];

pub struct FaceEmbedder {
    session: Session,
}

impl FaceEmbedder {
    pub fn new(model_path: &Path) -> Result<Self, CullingError> {
        let session = Session::builder()
            .map_err(|e| CullingError::Inference(format!("Failed to create session builder: {}", e)))?
            .commit_from_file(model_path)
            .map_err(|e| CullingError::Inference(format!("Failed to load model: {}", e)))?;

        Ok(Self { session })
    }

    /// Align and embed a face. Returns 512-d L2-normalized embedding.
    pub fn embed(
        &mut self,
        image_path: &Path,
        keypoints: &[[f32; 2]; 5],
    ) -> Result<Vec<f32>, CullingError> {
        // 1. Load the original image
        let img = image::open(image_path)?;

        // 2. Compute similarity transform from detected keypoints to ArcFace template
        let src: [[f64; 2]; 5] = [
            [keypoints[0][0] as f64, keypoints[0][1] as f64],
            [keypoints[1][0] as f64, keypoints[1][1] as f64],
            [keypoints[2][0] as f64, keypoints[2][1] as f64],
            [keypoints[3][0] as f64, keypoints[3][1] as f64],
            [keypoints[4][0] as f64, keypoints[4][1] as f64],
        ];
        let forward = estimate_similarity_transform(&src, &ARCFACE_TEMPLATE)?;

        // 3. Compute the inverse transform (template coords -> original image coords)
        let inverse = invert_similarity_transform(&forward);

        // 4. Warp to 112x112 aligned face
        let aligned = warp_affine(&img, &inverse, (112, 112));

        // 5. Normalize pixels to [-1, 1] and build NCHW tensor
        let mut tensor = Array4::<f32>::zeros((1, 3, 112, 112));
        for y in 0..112_usize {
            for x in 0..112_usize {
                let pixel = aligned.get_pixel(x as u32, y as u32);
                tensor[[0, 0, y, x]] = (pixel[0] as f32 - 127.5) / 127.5;
                tensor[[0, 1, y, x]] = (pixel[1] as f32 - 127.5) / 127.5;
                tensor[[0, 2, y, x]] = (pixel[2] as f32 - 127.5) / 127.5;
            }
        }

        // 6. Run ArcFace inference
        let input_ref = TensorRef::from_array_view(tensor.view())
            .map_err(|e| CullingError::Inference(format!("Failed to create tensor ref: {}", e)))?;

        let outputs = self
            .session
            .run(ort::inputs![input_ref])
            .map_err(|e| CullingError::Inference(format!("ArcFace inference failed: {}", e)))?;

        // 7. Extract 512-d output
        let (_shape, embedding_data) = outputs[0]
            .try_extract_tensor::<f32>()
            .map_err(|e| CullingError::Inference(format!("Failed to extract embedding: {}", e)))?;

        let mut embedding: Vec<f32> = embedding_data.iter().copied().collect();

        // 8. L2-normalize the embedding
        let norm = embedding.iter().map(|v| v * v).sum::<f32>().sqrt();
        if norm > 0.0 {
            for v in &mut embedding {
                *v /= norm;
            }
        }

        Ok(embedding)
    }
}

/// Estimate a 2D similarity transform (4 DOF: scale*cos, scale*sin, tx, ty)
/// that maps `src` points to `dst` points.
///
/// Returns a 2x3 forward affine matrix:
///   [a, -b, tx]
///   [b,  a, ty]
fn estimate_similarity_transform(
    src: &[[f64; 2]; 5],
    dst: &[[f64; 2]; 5],
) -> Result<[[f64; 3]; 2], CullingError> {
    let n = src.len();
    // Build 2N x 4 matrix A and 2N vector b
    let mut a_mat = DMatrix::<f64>::zeros(2 * n, 4);
    let mut b_vec = DVector::<f64>::zeros(2 * n);

    for i in 0..n {
        let (sx, sy) = (src[i][0], src[i][1]);
        let (dx, dy) = (dst[i][0], dst[i][1]);

        // Row 2i:   [sx, -sy, 1, 0] * [a, b, tx, ty]^T = dx
        a_mat[(2 * i, 0)] = sx;
        a_mat[(2 * i, 1)] = -sy;
        a_mat[(2 * i, 2)] = 1.0;
        a_mat[(2 * i, 3)] = 0.0;
        b_vec[2 * i] = dx;

        // Row 2i+1: [sy, sx, 0, 1] * [a, b, tx, ty]^T = dy
        a_mat[(2 * i + 1, 0)] = sy;
        a_mat[(2 * i + 1, 1)] = sx;
        a_mat[(2 * i + 1, 2)] = 0.0;
        a_mat[(2 * i + 1, 3)] = 1.0;
        b_vec[2 * i + 1] = dy;
    }

    // Solve via least squares: x = (A^T A)^{-1} A^T b
    let at = a_mat.transpose();
    let ata = &at * &a_mat;
    let atb = &at * &b_vec;

    let ata_inv = ata.try_inverse().ok_or_else(|| {
        CullingError::Inference(
            "Failed to invert A^T*A matrix in similarity transform".to_string(),
        )
    })?;

    let x = ata_inv * atb;

    let a = x[0];
    let b = x[1];
    let tx = x[2];
    let ty = x[3];

    Ok([[a, -b, tx], [b, a, ty]])
}

/// Invert a 2x3 similarity transform matrix.
///
/// Given forward: [a, -b, tx; b, a, ty]
/// The inverse is: [a, b, -a*tx - b*ty; -b, a, b*tx - a*ty] / (a^2 + b^2)
fn invert_similarity_transform(fwd: &[[f64; 3]; 2]) -> [[f64; 3]; 2] {
    let a = fwd[0][0];
    let tx = fwd[0][2];
    let b = fwd[1][0]; // this is b
    let ty = fwd[1][2];

    let det = a * a + b * b;

    // Inverse of [a, -b; b, a] is [a, b; -b, a] / (a^2 + b^2)
    let inv_a = a / det;
    let inv_neg_b = b / det; // corresponds to the (0,1) element
    let inv_b = -b / det; // corresponds to the (1,0) element
    let inv_a2 = a / det; // corresponds to the (1,1) element

    // Inverse translation: -R_inv * t
    let inv_tx = -(inv_a * tx + inv_neg_b * ty);
    let inv_ty = -(inv_b * tx + inv_a2 * ty);

    [[inv_a, inv_neg_b, inv_tx], [inv_b, inv_a2, inv_ty]]
}

/// Apply an affine warp to produce an output image.
///
/// `transform` maps output pixel coordinates to source pixel coordinates
/// (i.e., it is the inverse of the forward geometric transform).
fn warp_affine(
    img: &DynamicImage,
    transform: &[[f64; 3]; 2],
    output_size: (u32, u32),
) -> RgbImage {
    let (out_w, out_h) = output_size;
    let mut output = RgbImage::new(out_w, out_h);

    for y in 0..out_h {
        for x in 0..out_w {
            let src_x =
                transform[0][0] * x as f64 + transform[0][1] * y as f64 + transform[0][2];
            let src_y =
                transform[1][0] * x as f64 + transform[1][1] * y as f64 + transform[1][2];

            let pixel = bilinear_interpolate(img, src_x, src_y);
            output.put_pixel(x, y, pixel);
        }
    }

    output
}

/// Bilinear interpolation sampling from a DynamicImage.
fn bilinear_interpolate(img: &DynamicImage, x: f64, y: f64) -> Rgb<u8> {
    let (w, h) = img.dimensions();

    // Clamp to valid range
    let x = x.max(0.0).min((w as f64) - 1.001);
    let y = y.max(0.0).min((h as f64) - 1.001);

    let x0 = x.floor() as u32;
    let y0 = y.floor() as u32;
    let x1 = (x0 + 1).min(w - 1);
    let y1 = (y0 + 1).min(h - 1);

    let fx = x - x0 as f64;
    let fy = y - y0 as f64;

    let p00 = img.get_pixel(x0, y0);
    let p10 = img.get_pixel(x1, y0);
    let p01 = img.get_pixel(x0, y1);
    let p11 = img.get_pixel(x1, y1);

    let mut result = [0u8; 3];
    for c in 0..3 {
        let v = (1.0 - fx) * (1.0 - fy) * p00[c] as f64
            + fx * (1.0 - fy) * p10[c] as f64
            + (1.0 - fx) * fy * p01[c] as f64
            + fx * fy * p11[c] as f64;
        result[c] = v.round().clamp(0.0, 255.0) as u8;
    }

    Rgb(result)
}
