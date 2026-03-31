"""Sort photos into per-person folders using face detection and clustering."""

import argparse
import shutil
import sys
from pathlib import Path

import cv2
import numpy as np
from insightface.app import FaceAnalysis
from PIL import Image
from sklearn.cluster import DBSCAN
from tqdm import tqdm

IMAGE_EXTENSIONS = {".jpg", ".jpeg", ".png", ".heic", ".tif", ".tiff"}


def init_detector():
    """Initialize InsightFace detector with buffalo_l model."""
    app = FaceAnalysis(
        name="buffalo_l",
        allowed_modules=["detection", "recognition"],
        providers=["CPUExecutionProvider"],
    )
    app.prepare(ctx_id=0, det_size=(640, 640))
    return app


def detect_faces(app, image_path):
    """Detect faces in an image. Returns (faces, img) or ([], None)."""
    img = cv2.imread(str(image_path))
    if img is None:
        return [], None
    faces = app.get(img)
    return faces, img


def filter_face(face, min_confidence=0.5, min_face_size=80):
    """Filter a face by confidence, size, and landmark quality.

    Three gates:
    1. Detection score >= min_confidence
    2. Min dimension of bounding box >= min_face_size
    3. Keypoints exist and eyes+nose (indices 0, 1, 2) are within the bbox
    """
    # Gate 1: confidence
    if face.det_score < min_confidence:
        return False

    # Gate 2: minimum face size
    x1, y1, x2, y2 = face.bbox
    width = x2 - x1
    height = y2 - y1
    if min(width, height) < min_face_size:
        return False

    # Gate 3: landmark quality — eyes and nose within bbox
    if face.kps is None:
        return False
    for idx in (0, 1, 2):
        kx, ky = face.kps[idx]
        if not (x1 <= kx <= x2 and y1 <= ky <= y2):
            return False

    return True


def scan_photos(input_dir, app, min_confidence=0.5, min_face_size=80):
    """Scan all photos in input_dir, detect and filter faces.

    Returns (face_records, no_face_photos).
    """
    input_path = Path(input_dir)
    image_files = sorted(
        f
        for f in input_path.iterdir()
        if f.is_file() and f.suffix.lower() in IMAGE_EXTENSIONS
    )

    face_records = []
    no_face_photos = []

    for image_file in tqdm(image_files, desc="Scanning photos"):
        faces, img = detect_faces(app, image_file)
        if not faces:
            no_face_photos.append(image_file)
            continue

        filtered = []
        for face_index, face in enumerate(faces):
            if filter_face(face, min_confidence, min_face_size):
                record = {
                    "embedding": face.normed_embedding,
                    "det_score": float(face.det_score),
                    "bbox": face.bbox.tolist(),
                    "photo_path": image_file,
                    "face_index": face_index,
                }
                filtered.append(record)

        if filtered:
            face_records.extend(filtered)
        else:
            no_face_photos.append(image_file)

    return face_records, no_face_photos


def cluster_faces(face_records, eps=0.55, min_samples=2):
    """Cluster face embeddings with DBSCAN using cosine distance."""
    embeddings = np.array([r["embedding"] for r in face_records])
    clustering = DBSCAN(eps=eps, min_samples=min_samples, metric="cosine")
    labels = clustering.fit_predict(embeddings)
    return labels


def build_clusters(face_records, labels):
    """Build cluster dict and unclustered list from DBSCAN labels.

    Returns (clusters, unclustered) where:
    - clusters: dict of {cluster_id: [records]}
    - unclustered: list of records with label == -1
    """
    clusters = {}
    unclustered = []

    for record, label in zip(face_records, labels):
        if label == -1:
            unclustered.append(record)
        else:
            clusters.setdefault(label, []).append(record)

    return clusters, unclustered


def create_thumbnail(photo_path, bbox, output_path, size=150):
    """Create a face thumbnail with 20% padding around the bounding box."""
    img = Image.open(photo_path)
    x1, y1, x2, y2 = bbox
    width = x2 - x1
    height = y2 - y1

    # Add 20% padding
    pad_x = width * 0.2
    pad_y = height * 0.2

    crop_x1 = max(0, x1 - pad_x)
    crop_y1 = max(0, y1 - pad_y)
    crop_x2 = min(img.width, x2 + pad_x)
    crop_y2 = min(img.height, y2 + pad_y)

    cropped = img.crop((crop_x1, crop_y1, crop_x2, crop_y2))
    cropped.thumbnail((size, size))
    cropped.save(str(output_path), "JPEG", quality=90)


def organize_output(clusters, unclustered, no_face_photos, output_dir):
    """Organize photos into per-person folders with thumbnails.

    Structure:
    - face_XX/ — one per cluster, with thumbnail.jpg and copies of photos
    - unclustered/ — photos from noise points (DBSCAN label -1)
    - no_faces/ — photos with no detected faces
    """
    output_path = Path(output_dir)

    if output_path.exists():
        print(f"Error: output directory already exists: {output_path}", file=sys.stderr)
        sys.exit(1)

    output_path.mkdir(parents=True)

    # Per-person folders
    for idx, (cluster_id, records) in enumerate(
        sorted(clusters.items()), start=1
    ):
        folder_name = f"face_{idx:02d}"
        folder = output_path / folder_name
        folder.mkdir()

        # Find representative face (highest det_score)
        best = max(records, key=lambda r: r["det_score"])
        create_thumbnail(
            best["photo_path"], best["bbox"], folder / "thumbnail.jpg"
        )

        # Copy photos (deduplicate)
        seen = set()
        for record in records:
            photo = record["photo_path"]
            if photo not in seen:
                seen.add(photo)
                shutil.copy2(str(photo), str(folder / photo.name))

    # Unclustered folder
    if unclustered:
        unclustered_dir = output_path / "unclustered"
        unclustered_dir.mkdir()
        seen = set()
        for record in unclustered:
            photo = record["photo_path"]
            if photo not in seen:
                seen.add(photo)
                shutil.copy2(str(photo), str(unclustered_dir / photo.name))

    # No-faces folder
    if no_face_photos:
        no_faces_dir = output_path / "no_faces"
        no_faces_dir.mkdir()
        for photo in no_face_photos:
            shutil.copy2(str(photo), str(no_faces_dir / photo.name))


def main():
    """CLI entry point for face-based photo sorting."""
    parser = argparse.ArgumentParser(
        description="Sort photos into per-person folders using face detection and clustering."
    )
    parser.add_argument("input_dir", help="Directory containing photos to sort")
    parser.add_argument(
        "--output",
        default="./sorted_faces",
        help="Output directory (default: ./sorted_faces)",
    )
    parser.add_argument(
        "--min-confidence",
        type=float,
        default=0.5,
        help="Minimum face detection confidence (default: 0.5)",
    )
    parser.add_argument(
        "--min-face-size",
        type=int,
        default=80,
        help="Minimum face size in pixels (default: 80)",
    )
    parser.add_argument(
        "--eps",
        type=float,
        default=0.55,
        help="DBSCAN epsilon for cosine distance (default: 0.55)",
    )
    parser.add_argument(
        "--min-samples",
        type=int,
        default=2,
        help="DBSCAN minimum samples per cluster (default: 2)",
    )

    args = parser.parse_args()

    input_dir = Path(args.input_dir)
    if not input_dir.is_dir():
        print(f"Error: input directory does not exist: {input_dir}", file=sys.stderr)
        sys.exit(1)

    # Stage 1: Initialize detector
    print("Initializing face detector...")
    app = init_detector()
    print("Detector ready.\n")

    # Stage 2: Scan photos
    print("Scanning photos for faces...")
    face_records, no_face_photos = scan_photos(
        input_dir, app, args.min_confidence, args.min_face_size
    )
    print(f"  Found {len(face_records)} faces in {len(face_records)} detections")
    print(f"  {len(no_face_photos)} photos with no usable faces\n")

    if not face_records:
        print("No faces found. Nothing to cluster.")
        if no_face_photos:
            output_path = Path(args.output)
            if output_path.exists():
                print(
                    f"Error: output directory already exists: {output_path}",
                    file=sys.stderr,
                )
                sys.exit(1)
            output_path.mkdir(parents=True)
            no_faces_dir = output_path / "no_faces"
            no_faces_dir.mkdir()
            for photo in no_face_photos:
                shutil.copy2(str(photo), str(no_faces_dir / photo.name))
            print(f"Copied {len(no_face_photos)} photos to {no_faces_dir}")
        return

    # Stage 3: Cluster faces
    print("Clustering faces...")
    labels = cluster_faces(face_records, args.eps, args.min_samples)
    clusters, unclustered = build_clusters(face_records, labels)
    num_clusters = len(clusters)
    num_unclustered = len(unclustered)
    print(f"  {num_clusters} clusters found")
    print(f"  {num_unclustered} unclustered face detections\n")

    # Stage 4: Organize output
    print("Organizing output...")
    organize_output(clusters, unclustered, no_face_photos, args.output)

    # Summary
    total_clustered_photos = sum(
        len({r["photo_path"] for r in records}) for records in clusters.values()
    )
    total_unclustered_photos = len({r["photo_path"] for r in unclustered})
    print(f"\nDone! Output written to: {args.output}")
    print(f"  {num_clusters} person folders ({total_clustered_photos} photos)")
    print(f"  {total_unclustered_photos} unclustered photos")
    print(f"  {len(no_face_photos)} no-face photos")


if __name__ == "__main__":
    main()
