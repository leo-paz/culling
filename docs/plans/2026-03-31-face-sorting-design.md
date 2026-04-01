# Face-Based Photo Sorting — Design

## Problem

Sort ~528 wedding photos into folders grouped by person, using face detection and unsupervised clustering. Photos with multiple people appear in multiple folders. Photos with no detectable faces go into a separate folder.

## Pipeline

```
Input folder → DETECT → FILTER → EMBED + CLUSTER → ORGANIZE → Output folders
```

## Stack

- **InsightFace** (`buffalo_l` model pack): RetinaFace detection + ArcFace 512-d embeddings
- **scikit-learn DBSCAN**: Unsupervised clustering with cosine distance
- **Python CLI**: Single script `sort_faces.py`

## Detection & Filtering

InsightFace's `app.get()` returns faces with bounding boxes, confidence scores, 512-d embeddings, and 5-point landmarks.

A face must pass all three gates:

| Filter | Threshold | Purpose |
|--------|-----------|---------|
| Detection confidence | > 0.5 | Drops false positives, occluded faces |
| Face size | > 80px shortest side | Drops tiny background faces |
| Landmark quality | Both eyes + nose | Drops extreme profiles, partial faces |

All thresholds configurable via CLI flags.

## Clustering

1. Collect 512-d ArcFace embeddings from all faces that pass filtering
2. L2-normalize embeddings
3. DBSCAN with `eps=0.55`, `min_samples=2`, `metric='cosine'`
4. Each cluster = one person. Noise points (label -1) → `unclustered/`

Representative thumbnail: highest-confidence face in each cluster, cropped to 150x150px.

## Output Structure

```
output/
├── face_01/
│   ├── thumbnail.jpg
│   ├── DSCF5245.JPG
│   └── DSCF5301.JPG
├── face_02/
│   ├── thumbnail.jpg
│   └── DSCF5260.JPG
├── no_faces/
│   └── DSCF5250.JPG
└── unclustered/
    └── DSCF5299.JPG
```

Photos are **copied** (not symlinked) into folders. A photo with N detected faces can appear in up to N person folders.

## CLI Interface

```bash
python sort_faces.py ~/desktop/photography/"bryan wedding week" --output ./sorted
```

| Flag | Default | Purpose |
|------|---------|---------|
| `--output` | `./sorted_faces` | Output directory |
| `--min-confidence` | `0.5` | Detection confidence threshold |
| `--min-face-size` | `80` | Minimum face size in pixels |
| `--eps` | `0.55` | DBSCAN clustering distance |
| `--min-samples` | `2` | Minimum photos per cluster |

## Dependencies

- `insightface`
- `onnxruntime`
- `scikit-learn`
- `numpy`
- `opencv-python-headless`
- `tqdm`
- `Pillow`

## Performance

Expected ~2-5 minutes for 528 photos on Apple Silicon via ONNX Runtime.
