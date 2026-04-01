# Culling — Desktop Photo Culling App Design

## Problem

Photographers need a fast, keyboard-driven tool to review photos, sort them by person (face detection), grade them (bad/ok/good), and export keepers. Existing tools are either expensive (Photo Mechanic), subscription-based (AfterShoot), or too slow.

## Solution

A Tauri v2 desktop app with face detection, auto-grading, and a two-view culling workflow. Open source.

## Tech Stack

| Layer | Tech |
|-------|------|
| Frontend | Svelte 5 + Tailwind v4 + shadcn-svelte |
| Backend | Rust |
| ML inference | `ort` (ONNX Runtime) — InsightFace buffalo_l + NIMA aesthetic model |
| Clustering | `linfa` DBSCAN (Euclidean on L2-normalized embeddings) |
| Image processing | `image` + `imageproc` + `nalgebra` |
| Desktop shell | Tauri v2 |

## Architecture

```
┌─────────────────────────────────────────────┐
│  Svelte 5 + Tailwind v4 + shadcn-svelte     │
│  ┌──────────┐  ┌──────────┐  ┌───────────┐  │
│  │ Timeline  │  │ People   │  │  Export    │  │
│  │ View      │  │ View     │  │  Dialog    │  │
│  └──────────┘  └──────────┘  └───────────┘  │
│            Tauri IPC (commands + channels)    │
├─────────────────────────────────────────────┤
│  Rust Backend                                │
│  ┌──────────┐  ┌──────────┐  ┌───────────┐  │
│  │ Scanner  │  │ Grader   │  │ Organizer │  │
│  │ detect   │  │ heuristic│  │ export    │  │
│  │ embed    │  │ + model  │  │ copy/trash│  │
│  │ cluster  │  │ scoring  │  │           │  │
│  └──────────┘  └──────────┘  └───────────┘  │
│  ort (ONNX) · linfa · image · imageproc     │
├─────────────────────────────────────────────┤
│  Project File (~/.culling/projects/*.json)   │
│  Persists: grades, clusters, face data       │
└─────────────────────────────────────────────┘
```

Three Rust modules:
- **Scanner** — loads ONNX models (RetinaFace/SCRFD + ArcFace), detects faces, generates 512-d embeddings, clusters with DBSCAN
- **Grader** — heuristic checks (sharpness, exposure, eyes closed, near-duplicates) + NIMA aesthetic model → assigns bad/ok/good
- **Organizer** — exports keepers (flat or by-person), moves rejects to system trash

## UI Layout

```
┌──────────┬─────────────────────────────────┐
│          │  ┌─────────────────────────────┐ │
│ Sidebar  │  │                             │ │
│          │  │     Photo (fullscreen)      │ │
│ Projects │  │                             │ │
│          │  │                             │ │
│ ──────── │  └─────────────────────────────┘ │
│          │  ┌─────────────────────────────┐ │
│ People   │  │ ◄ ■ ■ ■ ■ ■ ■ ■ ■ ■ ■ ► │ │
│ (faces)  │  │       Filmstrip             │ │
│          │  └─────────────────────────────┘ │
│ ──────── │  ┌─────────────────────────────┐ │
│ Grades   │  │ Grade: OK  │ 42/528 │ F2.8 │ │
│ Bad: 12  │  │       Status bar            │ │
│ OK: 340  │  └─────────────────────────────┘ │
│ Good: 50 │                                  │
└──────────┴──────────────────────────────────┘
```

### Two Views (shared grade state)

- **Timeline View** — all photos chronologically. Filmstrip + large preview. For first-pass culling.
- **People View** — sidebar shows detected people (face thumbnails + count). Click a person → filmstrip filters to their photos. For per-person selection.

### Keyboard Shortcuts

| Key | Action |
|-----|--------|
| `←` `→` | Previous / next photo |
| `1` | Grade as Bad |
| `2` | Grade as OK |
| `3` | Grade as Good |
| `0` | Clear grade (back to ungraded = kept) |
| `F` | Toggle fullscreen preview |
| `T` / `P` | Switch Timeline / People view |
| `Cmd+E` | Open export dialog |

### Grade Indicators

Filmstrip thumbnails get colored borders: red (bad), yellow (ok), green (good), no border (ungraded). Main preview shows a small badge. Status bar shows `Grade: OK (auto)` vs `Grade: Good (manual)`.

## Scan & Grade Workflow

### Import

1. User clicks "Import Folder" → selects photo directory
2. Photos load immediately into Timeline view (directory listing + thumbnail decoding)
3. **Face detection starts automatically** in background — progress bar: `Detecting faces... 142/528`
4. **Auto-Grade button** appears in toolbar — user clicks to opt-in to heuristic + NIMA scoring

### Face Detection (automatic on import)

- Runs once, results cached in project file
- As faces are detected, People sidebar populates live
- When clustering completes, People tab activates with a highlight
- No re-inference when browsing — People view reads from cached data

### Auto-Grading (manual trigger via button)

Heuristic signals flag obvious "bad":

| Signal | Method | Flags as Bad when... |
|--------|--------|---------------------|
| Sharpness | Laplacian variance on luminance | Variance below threshold |
| Exposure | Histogram clipped pixel analysis | >30% pixels clipped |
| Eyes closed | Eye aspect ratio from face landmarks | EAR below threshold |
| Near-duplicate | Cosine similarity between consecutive embeddings | >0.95 similarity, flag lower-scoring |

NIMA aesthetic model ranks the rest:
- Heuristic fail → **Bad** (auto)
- NIMA score < 5.0 → **OK** (auto)
- NIMA score >= 5.0 → **Good** (auto)

All auto-grades are suggestions — user overrides with a single keystroke. Thresholds tunable in settings.

## Export

Export dialog (`Cmd+E`):

- **Include filter:** All / OK + Good / Good only
- **Organization:** Flat folder / By person (subfolders named by cluster labels or user-given names)
- **Output path:** user-selected directory
- **Cleanup:** Optional checkbox to move "Bad" photos to system trash (recoverable)
- Ungraded photos count as "OK" for export filtering
- Progress bar during copy, then "Done — open folder?" prompt

## Project Persistence

Each imported folder gets a project file at `~/.culling/projects/<hash>.json` containing:
- Photo list with paths
- Face detection results (bboxes, embeddings, cluster assignments)
- Cluster labels (auto-numbered or user-renamed)
- Grade state per photo (bad/ok/good/ungraded, auto vs manual)
- Scan settings used

This avoids re-scanning when reopening a project.

## ONNX Models

| Model | Source | Size | Purpose |
|-------|--------|------|---------|
| `det_10g.onnx` | InsightFace buffalo_l (SCRFD) | ~16MB | Face detection |
| `w600k_r50.onnx` | InsightFace buffalo_l (ArcFace) | ~167MB | Face embeddings (512-d) |
| `nima.onnx` | NIMA (Neural Image Assessment) | ~25MB | Aesthetic quality scoring |

Models bundled with the app or auto-downloaded on first run.

## Rust Crate Dependencies

- `tauri` v2 — desktop shell + IPC
- `ort` — ONNX Runtime inference (with `coreml` feature for Apple Silicon)
- `linfa-clustering` — DBSCAN
- `image` — JPEG/PNG decoding, resizing
- `imageproc` — affine transforms (face alignment)
- `nalgebra` — linear algebra for similarity transforms
- `ndarray` — tensor manipulation for ONNX I/O
- `serde` / `serde_json` — project file serialization
- `trash` — cross-platform system trash
