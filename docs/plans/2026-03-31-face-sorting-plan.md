# Face-Based Photo Sorting — Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Sort photos into person-based folders using face detection, embedding, and unsupervised clustering.

**Architecture:** Single Python CLI script. InsightFace detects/embeds faces, DBSCAN clusters them, then photos are copied into per-person folders with representative thumbnails.

**Tech Stack:** InsightFace (buffalo_l), scikit-learn, OpenCV, Pillow, tqdm, argparse

---

### Task 1: Project Setup — Dependencies and Requirements

**Files:**
- Create: `requirements.txt`
- Create: `sort_faces.py` (empty placeholder)

**Step 1: Create requirements.txt**

```txt
insightface
onnxruntime
scikit-learn
numpy<2
opencv-python-headless
tqdm
Pillow
```

Note: `numpy<2` because insightface 0.7.3 is built against NumPy 1.x.

**Step 2: Create empty sort_faces.py**

```python
"""Sort photos into folders by detected faces."""
```

**Step 3: Install dependencies**

Run:
```bash
pip install --upgrade pip setuptools wheel cython
ARCHFLAGS="-arch arm64" pip install -r requirements.txt
```

If insightface fails to build, try a pre-built wheel:
```bash
pip install insightface --no-build-isolation
```

**Step 4: Verify insightface loads**

Run:
```bash
python -c "import insightface; print(insightface.__version__)"
```

Expected: prints version (e.g., `0.7.3`)

**Step 5: Commit**

```bash
git add requirements.txt sort_faces.py
git commit -m "feat: add project setup with requirements"
```

---

### Task 2: Face Detection Module

**Files:**
- Modify: `sort_faces.py`
- Test: manual test on a single photo

**Step 1: Write the face detection function**

Add to `sort_faces.py`:

```python
"""Sort photos into folders by detected faces."""

import argparse
import sys
from pathlib import Path

import cv2
import numpy as np
from insightface.app import FaceAnalysis
from tqdm import tqdm


def init_detector():
    """Initialize InsightFace with buffalo_l model pack."""
    app = FaceAnalysis(
        name="buffalo_l",
        allowed_modules=["detection", "recognition"],
        providers=["CPUExecutionProvider"],
    )
    app.prepare(ctx_id=0, det_size=(640, 640))
    return app


def detect_faces(app, image_path):
    """Detect faces in a single image. Returns list of face dicts."""
    img = cv2.imread(str(image_path))
    if img is None:
        return []
    faces = app.get(img)
    return faces, img
```

**Step 2: Add a quick manual test block at the bottom**

```python
if __name__ == "__main__":
    # Quick test: detect faces in a single image
    import sys
    if len(sys.argv) > 1:
        app = init_detector()
        test_path = sys.argv[1]
        faces, img = detect_faces(app, test_path)
        print(f"Detected {len(faces)} faces in {test_path}")
        for i, f in enumerate(faces):
            bbox = f.bbox.astype(int)
            w, h = bbox[2] - bbox[0], bbox[3] - bbox[1]
            print(f"  Face {i}: score={f.det_score:.3f}, size={min(w,h)}px, embedding shape={f.embedding.shape}")
```

**Step 3: Run on a test photo**

Run:
```bash
python sort_faces.py ~/desktop/photography/"bryan wedding week"/DSCF5245.JPG
```

Expected: prints detected face count with scores and sizes. First run downloads buffalo_l model (~326MB).

**Step 4: Commit**

```bash
git add sort_faces.py
git commit -m "feat: add face detection with InsightFace"
```

---

### Task 3: Face Filtering

**Files:**
- Modify: `sort_faces.py`

**Step 1: Write the face filtering function**

Add after `detect_faces`:

```python
def filter_face(face, min_confidence, min_face_size):
    """Return True if face passes quality gates."""
    # Gate 1: detection confidence
    if face.det_score < min_confidence:
        return False

    # Gate 2: face size (shortest side of bounding box)
    bbox = face.bbox.astype(int)
    w = bbox[2] - bbox[0]
    h = bbox[3] - bbox[1]
    if min(w, h) < min_face_size:
        return False

    # Gate 3: landmark quality — need both eyes and nose
    # kps shape is (5, 2): left_eye, right_eye, nose, left_mouth, right_mouth
    kps = face.kps
    if kps is None:
        return False
    # Check that the 5 keypoints are within the bounding box (sanity check)
    # If landmarks are wildly outside bbox, face is likely a false positive
    for i in range(3):  # eyes and nose only
        x, y = kps[i]
        if x < bbox[0] or x > bbox[2] or y < bbox[1] or y > bbox[3]:
            return False

    return True
```

**Step 2: Update the test block to show filtering**

Replace the `__main__` block:

```python
if __name__ == "__main__":
    import sys
    if len(sys.argv) > 1:
        app = init_detector()
        test_path = sys.argv[1]
        faces, img = detect_faces(app, test_path)
        print(f"Detected {len(faces)} raw faces")
        kept = [f for f in faces if filter_face(f, 0.5, 80)]
        print(f"After filtering: {len(kept)} faces")
        for i, f in enumerate(kept):
            print(f"  Face {i}: score={f.det_score:.3f}")
```

**Step 3: Test filtering**

Run:
```bash
python sort_faces.py ~/desktop/photography/"bryan wedding week"/DSCF5245.JPG
```

Expected: shows raw count vs filtered count. Some faces should be filtered out if they're small/blurry.

**Step 4: Commit**

```bash
git add sort_faces.py
git commit -m "feat: add face quality filtering"
```

---

### Task 4: Batch Processing — Scan All Photos

**Files:**
- Modify: `sort_faces.py`

**Step 1: Write the batch scanning function**

Add after `filter_face`:

```python
IMAGE_EXTENSIONS = {".jpg", ".jpeg", ".png", ".heic", ".tif", ".tiff"}


def scan_photos(input_dir, app, min_confidence, min_face_size):
    """Scan all photos in a directory. Returns face records and no-face photo list.

    Returns:
        face_records: list of dicts with keys: embedding, det_score, bbox, photo_path, face_index
        no_face_photos: list of photo paths with zero passing faces
    """
    input_path = Path(input_dir)
    photos = sorted(
        p for p in input_path.iterdir()
        if p.suffix.lower() in IMAGE_EXTENSIONS and not p.name.startswith(".")
    )

    if not photos:
        print(f"No images found in {input_dir}")
        sys.exit(1)

    face_records = []
    no_face_photos = []

    for photo_path in tqdm(photos, desc="Scanning photos"):
        result = detect_faces(app, photo_path)
        if not result:
            no_face_photos.append(photo_path)
            continue

        faces, img = result
        kept = [f for f in faces if filter_face(f, min_confidence, min_face_size)]

        if not kept:
            no_face_photos.append(photo_path)
            continue

        for i, face in enumerate(kept):
            face_records.append({
                "embedding": face.normed_embedding,
                "det_score": float(face.det_score),
                "bbox": face.bbox.astype(int).tolist(),
                "photo_path": photo_path,
                "face_index": i,
            })

    return face_records, no_face_photos
```

Note: uses `face.normed_embedding` (already L2-normalized by InsightFace) instead of raw `face.embedding`.

**Step 2: Update main block for batch test**

```python
if __name__ == "__main__":
    import sys
    if len(sys.argv) > 1:
        app = init_detector()
        input_dir = sys.argv[1]
        face_records, no_face_photos = scan_photos(input_dir, app, 0.5, 80)
        print(f"\n{len(face_records)} faces from {len(set(r['photo_path'] for r in face_records))} photos")
        print(f"{len(no_face_photos)} photos with no faces")
```

**Step 3: Test on the full folder (this will take 2-5 minutes and download models on first run)**

Run:
```bash
python sort_faces.py ~/desktop/photography/"bryan wedding week"
```

Expected: progress bar, then summary of face count and no-face photo count.

**Step 4: Commit**

```bash
git add sort_faces.py
git commit -m "feat: add batch photo scanning with progress"
```

---

### Task 5: Clustering

**Files:**
- Modify: `sort_faces.py`

**Step 1: Write the clustering function**

Add after `scan_photos`:

```python
from sklearn.cluster import DBSCAN


def cluster_faces(face_records, eps, min_samples):
    """Cluster face embeddings with DBSCAN. Returns cluster labels (-1 = noise)."""
    if not face_records:
        return []

    embeddings = np.array([r["embedding"] for r in face_records])
    clustering = DBSCAN(eps=eps, min_samples=min_samples, metric="cosine").fit(embeddings)
    return clustering.labels_
```

**Step 2: Write the function that builds cluster-to-photo mapping**

```python
def build_clusters(face_records, labels):
    """Build mapping from cluster_id to list of face records, plus unclustered list.

    Returns:
        clusters: dict of {cluster_id: [face_records]}  (cluster_id >= 0)
        unclustered: list of face_records with label -1
    """
    clusters = {}
    unclustered = []

    for record, label in zip(face_records, labels):
        if label == -1:
            unclustered.append(record)
        else:
            clusters.setdefault(label, []).append(record)

    return clusters, unclustered
```

**Step 3: Update main block to show clustering results**

```python
if __name__ == "__main__":
    import sys
    if len(sys.argv) > 1:
        app = init_detector()
        input_dir = sys.argv[1]
        face_records, no_face_photos = scan_photos(input_dir, app, 0.5, 80)
        labels = cluster_faces(face_records, eps=0.55, min_samples=2)
        clusters, unclustered = build_clusters(face_records, labels)
        print(f"\nFound {len(clusters)} people")
        for cid, records in sorted(clusters.items()):
            unique_photos = len(set(r["photo_path"] for r in records))
            print(f"  Person {cid}: {unique_photos} photos")
        print(f"Unclustered: {len(set(r['photo_path'] for r in unclustered))} photos")
        print(f"No faces: {len(no_face_photos)} photos")
```

**Step 4: Test clustering**

Run:
```bash
python sort_faces.py ~/desktop/photography/"bryan wedding week"
```

Expected: shows number of people found, photos per person, unclustered count.

**Step 5: Commit**

```bash
git add sort_faces.py
git commit -m "feat: add DBSCAN face clustering"
```

---

### Task 6: Output — Organize Into Folders

**Files:**
- Modify: `sort_faces.py`

**Step 1: Write the thumbnail generation function**

```python
from PIL import Image


def create_thumbnail(photo_path, bbox, output_path, size=150):
    """Crop face from photo and save as thumbnail."""
    img = Image.open(photo_path)
    x1, y1, x2, y2 = bbox
    # Add 20% padding around the face for context
    w, h = x2 - x1, y2 - y1
    pad_x, pad_y = int(w * 0.2), int(h * 0.2)
    x1 = max(0, x1 - pad_x)
    y1 = max(0, y1 - pad_y)
    x2 = min(img.width, x2 + pad_x)
    y2 = min(img.height, y2 + pad_y)
    cropped = img.crop((x1, y1, x2, y2))
    cropped.thumbnail((size, size))
    cropped.save(output_path, "JPEG", quality=90)
```

**Step 2: Write the main organize function**

```python
import shutil


def organize_output(clusters, unclustered, no_face_photos, output_dir):
    """Create output folder structure and copy photos."""
    output = Path(output_dir)
    if output.exists():
        print(f"Error: output directory {output} already exists. Remove it or choose a different path.")
        sys.exit(1)

    # Create face_XX folders
    for i, (cluster_id, records) in enumerate(sorted(clusters.items()), start=1):
        folder = output / f"face_{i:02d}"
        folder.mkdir(parents=True)

        # Find best face for thumbnail (highest det_score)
        best = max(records, key=lambda r: r["det_score"])
        create_thumbnail(best["photo_path"], best["bbox"], folder / "thumbnail.jpg")

        # Copy unique photos into folder
        copied = set()
        for record in records:
            if record["photo_path"] not in copied:
                shutil.copy2(record["photo_path"], folder / record["photo_path"].name)
                copied.add(record["photo_path"])

    # Create unclustered folder
    if unclustered:
        folder = output / "unclustered"
        folder.mkdir(parents=True)
        copied = set()
        for record in unclustered:
            if record["photo_path"] not in copied:
                shutil.copy2(record["photo_path"], folder / record["photo_path"].name)
                copied.add(record["photo_path"])

    # Create no_faces folder
    if no_face_photos:
        folder = output / "no_faces"
        folder.mkdir(parents=True)
        for photo_path in no_face_photos:
            shutil.copy2(photo_path, folder / photo_path.name)
```

**Step 3: Commit**

```bash
git add sort_faces.py
git commit -m "feat: add output folder organization with thumbnails"
```

---

### Task 7: CLI — Wire Everything Together

**Files:**
- Modify: `sort_faces.py`

**Step 1: Replace the `__main__` block with the full CLI**

```python
def main():
    parser = argparse.ArgumentParser(
        description="Sort photos into folders by detected faces."
    )
    parser.add_argument("input_dir", help="Path to folder of photos")
    parser.add_argument("--output", default="./sorted_faces", help="Output directory (default: ./sorted_faces)")
    parser.add_argument("--min-confidence", type=float, default=0.5, help="Min face detection confidence (default: 0.5)")
    parser.add_argument("--min-face-size", type=int, default=80, help="Min face size in pixels (default: 80)")
    parser.add_argument("--eps", type=float, default=0.55, help="DBSCAN eps for clustering (default: 0.55)")
    parser.add_argument("--min-samples", type=int, default=2, help="Min photos per person cluster (default: 2)")
    args = parser.parse_args()

    input_path = Path(args.input_dir)
    if not input_path.is_dir():
        print(f"Error: {args.input_dir} is not a directory")
        sys.exit(1)

    # Step 1: Initialize
    print("Loading face detection model...")
    app = init_detector()

    # Step 2: Scan and detect
    face_records, no_face_photos = scan_photos(
        input_path, app, args.min_confidence, args.min_face_size
    )

    photos_with_faces = len(set(r["photo_path"] for r in face_records))
    print(f"\nDetected {len(face_records)} faces across {photos_with_faces} photos")
    print(f"After filtering: {len(face_records)} faces")

    # Step 3: Cluster
    if face_records:
        labels = cluster_faces(face_records, args.eps, args.min_samples)
        clusters, unclustered = build_clusters(face_records, labels)
    else:
        clusters, unclustered = {}, []

    # Step 4: Organize
    print(f"\nFound {len(clusters)} people (clusters)")
    for i, (cid, records) in enumerate(sorted(clusters.items()), start=1):
        unique = len(set(r["photo_path"] for r in records))
        print(f"  face_{i:02d}: {unique} photos")
    unclustered_photos = len(set(r["photo_path"] for r in unclustered))
    print(f"{len(no_face_photos)} photos → no_faces/")
    print(f"{unclustered_photos} photos → unclustered/")

    organize_output(clusters, unclustered, no_face_photos, args.output)
    print(f"\nOutput written to {args.output}/")


if __name__ == "__main__":
    main()
```

**Step 2: Test the full pipeline**

Run:
```bash
python sort_faces.py ~/desktop/photography/"bryan wedding week" --output /tmp/test_sorted
```

Expected: full pipeline runs — progress bar, summary, folders created with thumbnails and copied photos.

**Step 3: Verify output structure**

Run:
```bash
ls /tmp/test_sorted/
ls /tmp/test_sorted/face_01/
```

Expected: `face_XX/` folders with `thumbnail.jpg` + copied JPGs, `no_faces/`, possibly `unclustered/`.

**Step 4: Commit**

```bash
git add sort_faces.py
git commit -m "feat: wire up full CLI with argparse"
```

---

### Task 8: Final Review and Cleanup

**Step 1: Review the complete script**

Read through `sort_faces.py` end-to-end. Ensure:
- All imports are at the top
- No duplicate functions
- No leftover test code
- The `main()` flow matches the design doc

**Step 2: Test with tuned parameters if clustering looks off**

If clusters are merging different people:
```bash
python sort_faces.py ~/desktop/photography/"bryan wedding week" --output /tmp/test2 --eps 0.45
```

If clusters are splitting the same person:
```bash
python sort_faces.py ~/desktop/photography/"bryan wedding week" --output /tmp/test3 --eps 0.65
```

**Step 3: Final commit**

```bash
git add -A
git commit -m "feat: face-based photo sorting tool complete"
```
