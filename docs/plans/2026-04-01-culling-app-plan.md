# Culling App — Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Build a Tauri v2 desktop app for photo culling with face-based organization, auto-grading, and keyboard-driven workflow.

**Architecture:** Rust backend handles all ML inference (face detection, embeddings, aesthetic scoring via ONNX Runtime), clustering (DBSCAN), and file operations. Svelte 5 frontend is a thin view layer communicating over Tauri IPC. Project state persisted as JSON.

**Tech Stack:** Tauri v2, Rust, ort (ONNX Runtime), linfa, image/imageproc/nalgebra, Svelte 5, Tailwind v4, shadcn-svelte

**Design Direction:** Dark, cinematic, utilitarian — like a professional darkroom. Photos are the star. Dark backgrounds (zinc-950) so images pop. Warm amber accent (#D4A574) for selections and active states. Typography: Satoshi for UI headings, DM Sans for body, JetBrains Mono for metadata/EXIF. Smooth crossfade transitions between photos. Filmstrip with tactile hover scaling. Minimal chrome, maximum photo real estate.

---

## Phase 1: Project Scaffolding

### Task 1: Scaffold Tauri v2 + Svelte 5 project

**Files:**
- Create: entire project structure via CLI

**Step 1: Initialize the Tauri project**

The project root is the current repo. Remove the Python prototype files first (sort_faces.py, generate_thumbnails.py, requirements.txt, __pycache__), keeping only docs/.

Run from the repo root:
```bash
npm create tauri-app@latest culling-app -- --template svelte-ts
```

Then move the generated contents into the repo root (not a subdirectory). The project structure should be:

```
/
├── src-tauri/
│   ├── Cargo.toml
│   ├── tauri.conf.json
│   ├── capabilities/
│   │   └── default.json
│   └── src/
│       ├── main.rs
│       └── lib.rs
├── src/
│   ├── App.svelte
│   ├── main.ts
│   └── app.css
├── docs/plans/          (keep existing)
├── package.json
├── vite.config.ts
├── svelte.config.js
├── tsconfig.json
└── index.html
```

**Step 2: Verify it runs**

Run: `npm run tauri dev`
Expected: A window opens with the default Tauri + Svelte template.

**Step 3: Commit**

```bash
git add -A
git commit -m "feat: scaffold Tauri v2 + Svelte 5 project"
```

---

### Task 2: Add Tailwind v4 + shadcn-svelte

**Files:**
- Modify: `vite.config.ts`
- Modify: `src/app.css`
- Create: `components.json` (via CLI)
- Create: `src/lib/utils.ts` (via CLI)

**Step 1: Install Tailwind v4**

```bash
npm install tailwindcss @tailwindcss/vite
```

Update `vite.config.ts` to add the Tailwind plugin (before the svelte plugin).

Update `src/app.css`:
```css
@import "tailwindcss";
```

**Step 2: Initialize shadcn-svelte**

```bash
npx shadcn-svelte@latest init
```

Select: default style, zinc base color, `src/app.css` for CSS, `$lib/components` for components path.

**Step 3: Add core shadcn components**

```bash
npx shadcn-svelte@latest add button dialog card tabs tooltip slider separator scroll-area badge progress
```

**Step 4: Verify**

Run: `npm run tauri dev`
Add a shadcn Button to App.svelte, confirm it renders with styling.

**Step 5: Commit**

```bash
git add -A
git commit -m "feat: add Tailwind v4 + shadcn-svelte"
```

---

### Task 3: Configure Tauri capabilities + app theme

**Files:**
- Modify: `src-tauri/tauri.conf.json`
- Modify: `src-tauri/capabilities/default.json`
- Modify: `src/app.css`

**Step 1: Configure Tauri for photo access**

Update `src-tauri/tauri.conf.json`:
- Set `productName` to `"Culling"`
- Set `identifier` to `"com.culling.app"`
- Set window defaults: width 1400, height 900, title "Culling"
- Enable decorations, resizable, fullscreenable

Update `src-tauri/capabilities/default.json` permissions:
```json
{
  "identifier": "default",
  "windows": ["main"],
  "permissions": [
    "core:default",
    "core:window:default",
    "dialog:allow-open",
    "dialog:allow-save",
    {
      "identifier": "fs:default",
      "allow": [{ "path": "$HOME/**" }, { "path": "/tmp/**" }]
    },
    "fs:allow-read",
    "fs:allow-write",
    "fs:allow-exists",
    "fs:allow-mkdir",
    "fs:allow-readdir",
    "fs:allow-copy",
    "fs:allow-remove"
  ]
}
```

**Step 2: Set up the dark theme in `src/app.css`**

@ frontend-design skill — Apply the cinematic darkroom aesthetic:

```css
@import "tailwindcss";

@theme {
  --color-surface: #09090b;
  --color-surface-raised: #18181b;
  --color-surface-overlay: #27272a;
  --color-accent: #d4a574;
  --color-accent-muted: #d4a57440;
  --color-grade-bad: #ef4444;
  --color-grade-ok: #eab308;
  --color-grade-good: #22c55e;
  --font-display: 'Satoshi', sans-serif;
  --font-body: 'DM Sans', sans-serif;
  --font-mono: 'JetBrains Mono', monospace;
}
```

Add Google Fonts links for Satoshi, DM Sans, JetBrains Mono in `index.html` (or use @fontsource packages).

**Step 3: Install Tauri plugins**

```bash
npm install @tauri-apps/plugin-fs @tauri-apps/plugin-dialog
cd src-tauri && cargo add tauri-plugin-fs tauri-plugin-dialog
```

Register plugins in `src-tauri/src/lib.rs`:
```rust
tauri::Builder::default()
    .plugin(tauri_plugin_fs::init())
    .plugin(tauri_plugin_dialog::init())
```

**Step 4: Verify**

Run: `npm run tauri dev`
Expected: Dark-themed window with Culling title, correct dimensions.

**Step 5: Commit**

```bash
git add -A
git commit -m "feat: configure Tauri capabilities and dark theme"
```

---

### Task 4: Set up Rust project structure

**Files:**
- Create: `src-tauri/src/scanner/mod.rs`
- Create: `src-tauri/src/scanner/detector.rs`
- Create: `src-tauri/src/scanner/embedder.rs`
- Create: `src-tauri/src/scanner/cluster.rs`
- Create: `src-tauri/src/grader/mod.rs`
- Create: `src-tauri/src/grader/heuristics.rs`
- Create: `src-tauri/src/grader/aesthetic.rs`
- Create: `src-tauri/src/organizer/mod.rs`
- Create: `src-tauri/src/organizer/export.rs`
- Create: `src-tauri/src/project.rs`
- Create: `src-tauri/src/commands.rs`
- Modify: `src-tauri/src/lib.rs`
- Modify: `src-tauri/Cargo.toml`

**Step 1: Add Rust dependencies to Cargo.toml**

```toml
[dependencies]
tauri = { version = "2", features = [] }
tauri-build = { version = "2", features = [] }
tauri-plugin-fs = "2"
tauri-plugin-dialog = "2"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
image = "0.25"
imageproc = "0.25"
nalgebra = "0.33"
ndarray = "0.16"
ort = { version = "2.0.0-rc.12", features = ["ndarray", "coreml"] }
linfa-clustering = "0.7"
linfa = "0.7"
linfa-nn = "0.7"
trash = "5"
walkdir = "2"
rayon = "1"
tokio = { version = "1", features = ["full"] }
```

**Step 2: Create module structure**

Create stub modules with `pub mod` declarations. Each module file has a comment describing its purpose and empty pub structs/fns that will be implemented later.

`src-tauri/src/lib.rs`:
```rust
mod commands;
mod grader;
mod organizer;
mod project;
mod scanner;

pub use commands::*;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            commands::import_folder,
            commands::get_project,
            commands::get_photo,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

`src-tauri/src/project.rs` — define core data types:
```rust
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub id: String,
    pub name: String,
    pub source_dir: PathBuf,
    pub photos: Vec<Photo>,
    pub clusters: Vec<Cluster>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Photo {
    pub path: PathBuf,
    pub filename: String,
    pub grade: Grade,
    pub grade_source: GradeSource,
    pub faces: Vec<FaceDetection>,
    pub aesthetic_score: Option<f32>,
    pub sharpness_score: Option<f32>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum Grade {
    Ungraded,
    Bad,
    Ok,
    Good,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum GradeSource {
    Manual,
    Auto,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FaceDetection {
    pub bbox: [f32; 4],
    pub confidence: f32,
    pub embedding: Vec<f32>,
    pub cluster_id: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Cluster {
    pub id: usize,
    pub label: String,
    pub representative_photo: PathBuf,
    pub representative_bbox: [f32; 4],
    pub photo_count: usize,
}
```

`src-tauri/src/commands.rs` — stub Tauri commands:
```rust
use crate::project::Project;

#[tauri::command]
pub async fn import_folder(path: String) -> Result<Project, String> {
    todo!()
}

#[tauri::command]
pub async fn get_project(id: String) -> Result<Project, String> {
    todo!()
}

#[tauri::command]
pub async fn get_photo(path: String) -> Result<Vec<u8>, String> {
    todo!()
}
```

**Step 3: Verify it compiles**

Run: `cd src-tauri && cargo check`
Expected: compiles with no errors (todo!() is fine for now).

**Step 4: Commit**

```bash
git add -A
git commit -m "feat: set up Rust module structure and data types"
```

---

## Phase 2: Photo Browser (Timeline View)

### Task 5: Implement folder import + photo listing (Rust)

**Files:**
- Modify: `src-tauri/src/commands.rs`
- Modify: `src-tauri/src/project.rs`

Implement `import_folder` command:
1. Read directory listing with `walkdir` (non-recursive, filter to image extensions: jpg, jpeg, png, heic, tif, tiff)
2. Create a `Project` struct with all photos listed (grade defaults to `Ungraded`)
3. Save project JSON to `~/.culling/projects/<hash>.json` (create dirs if needed)
4. Return the `Project` to the frontend

Add a `list_projects` command that reads all project JSON files from `~/.culling/projects/`.

Add a `save_project` helper that serializes and writes the project JSON.

**Test:** `cargo test` — write a unit test that creates a temp dir with dummy files, imports it, and verifies the photo list.

**Commit:** `git commit -m "feat: implement folder import and project persistence"`

---

### Task 6: Implement thumbnail generation (Rust)

**Files:**
- Create: `src-tauri/src/thumbnailer.rs`
- Modify: `src-tauri/src/commands.rs`
- Modify: `src-tauri/src/lib.rs`

Generate thumbnails on import for the filmstrip. For each photo:
1. Decode JPEG with `image` crate
2. Respect EXIF orientation (use `image::io::Reader::with_guessed_format().decode()` which handles orientation)
3. Resize to 300px on the longest edge using Lanczos3
4. Save as JPEG (quality 85) to `~/.culling/thumbnails/<project_id>/<filename>.jpg`
5. Return thumbnail paths as part of the Project data

Add a `get_thumbnail` command that returns the thumbnail path for a given photo.

Use `rayon` for parallel thumbnail generation (multiple cores).

Add a `generate_thumbnails` command with a `Channel<ProgressPayload>` parameter to stream progress to the frontend:
```rust
#[derive(Clone, Serialize)]
pub struct ProgressPayload {
    pub current: usize,
    pub total: usize,
    pub message: String,
}
```

**Test:** Unit test that generates a thumbnail from a test JPEG and verifies dimensions.

**Commit:** `git commit -m "feat: add parallel thumbnail generation with progress"`

---

### Task 7: Build the app shell layout (Svelte)

**Files:**
- Create: `src/lib/components/AppShell.svelte`
- Create: `src/lib/components/Sidebar.svelte`
- Create: `src/lib/components/Toolbar.svelte`
- Create: `src/lib/components/StatusBar.svelte`
- Create: `src/lib/stores/project.ts`
- Modify: `src/App.svelte`

@ frontend-design skill — Implement the cinematic darkroom layout:

**App Shell:** Full viewport, no scroll. CSS Grid layout:
```
grid-template-areas:
  "sidebar toolbar"
  "sidebar main"
  "sidebar statusbar"
grid-template-columns: 260px 1fr
grid-template-rows: 48px 1fr 32px
```

**Sidebar (`260px`):**
- Top section: Project name + folder path (truncated)
- Middle section: "People" area (placeholder, populated after face detection)
- Bottom section: Grade summary counts (Bad: 0, OK: 0, Good: 0)
- Background: `var(--color-surface)` (#09090b)
- Subtle 1px right border in zinc-800

**Toolbar (`48px` height):**
- Left: View switcher tabs (Timeline / People) using shadcn Tabs
- Center: "Detect Faces" and "Auto-Grade" action buttons
- Right: Export button, settings gear
- Background: `var(--color-surface-raised)` (#18181b)

**Main area:** Will hold the photo viewer + filmstrip (next task)

**Status bar (`32px` height):**
- Left: Current grade badge + "(auto)" or "(manual)" label
- Center: Position indicator "42 / 528"
- Right: EXIF data — aperture, shutter speed, ISO (placeholder)
- Background: `var(--color-surface)`, text in zinc-500, monospace font

**Svelte store** (`src/lib/stores/project.ts`):
- Writable store for current project, current photo index, view mode (timeline/people)
- Derived stores for filtered photos, grade counts

**Commit:** `git commit -m "feat: build app shell with sidebar, toolbar, and status bar"`

---

### Task 8: Build the photo viewer + filmstrip (Svelte)

**Files:**
- Create: `src/lib/components/PhotoViewer.svelte`
- Create: `src/lib/components/Filmstrip.svelte`
- Create: `src/lib/components/FilmstripItem.svelte`
- Modify: `src/lib/components/AppShell.svelte`

@ frontend-design skill:

**Photo Viewer (main area, top ~70%):**
- Displays the current photo at maximum size within the available space (object-fit: contain)
- Dark background (#09090b) — photo floats on darkness
- Smooth crossfade transition (200ms opacity) when navigating between photos
- Use `convertFileSrc()` from Tauri to serve local photos via asset protocol
- Loading state: subtle skeleton pulse in zinc-900

**Filmstrip (bottom ~30%, horizontal scroll):**
- Horizontal strip of thumbnail images, fixed height (120px)
- Current photo has a 2px amber (#D4A574) border, slight scale(1.05)
- Other thumbnails: 1px zinc-700 border, opacity 0.7
- Hover: opacity 1, slight scale(1.02), 150ms transition
- Scroll behavior: current photo always centered in the filmstrip (smooth scroll)
- Grade indicators: thin colored bar at the bottom of each thumbnail (red/yellow/green/none)
- Use shadcn ScrollArea for smooth horizontal scrolling

**Keyboard navigation:**
- `←` / `→` to navigate (update current index in store, smooth scroll filmstrip)
- Prevent default browser behavior for arrow keys
- Debounce rapid key presses (allow holding arrow key for fast scrubbing)

**Commit:** `git commit -m "feat: build photo viewer with filmstrip and keyboard navigation"`

---

### Task 9: Import flow — connect frontend to backend

**Files:**
- Create: `src/lib/components/ImportDialog.svelte`
- Create: `src/lib/components/WelcomeScreen.svelte`
- Modify: `src/App.svelte`
- Modify: `src/lib/stores/project.ts`

**Welcome Screen:** Shown when no project is loaded.
- Center of the screen: app name "Culling" in Satoshi bold, large
- Subtitle: "Drop a folder or click to import" in DM Sans, zinc-500
- A dashed-border drop zone (zinc-800 border, zinc-900/50 background)
- "Import Folder" button (shadcn Button, amber accent)
- Recent projects list below (from `list_projects` command)

**Import flow:**
1. User clicks "Import Folder" → opens native folder dialog via `@tauri-apps/plugin-dialog`
2. Calls `import_folder` Tauri command
3. Shows progress dialog (thumbnail generation) using the Channel-based progress
4. On completion, loads the project into the store → app shell renders

**Commit:** `git commit -m "feat: add import flow with welcome screen and progress"`

---

## Phase 3: Grading System

### Task 10: Implement grading (keyboard + persistence)

**Files:**
- Modify: `src/lib/stores/project.ts`
- Create: `src/lib/stores/keyboard.ts`
- Modify: `src/lib/components/PhotoViewer.svelte`
- Modify: `src/lib/components/FilmstripItem.svelte`
- Modify: `src/lib/components/StatusBar.svelte`
- Modify: `src/lib/components/Sidebar.svelte`
- Modify: `src-tauri/src/commands.rs`

**Keyboard handler** (`src/lib/stores/keyboard.ts`):
- Global keydown listener
- `1` → grade Bad, `2` → grade OK, `3` → grade Good, `0` → clear (Ungraded)
- On grade change: update the store, call `update_grade` Tauri command to persist

**Rust command** `update_grade`:
- Takes photo path + new grade + source (Manual)
- Updates the project JSON and saves

**Visual updates:**
- FilmstripItem: colored bottom bar (4px) — red/yellow/green based on grade
- StatusBar: shows current photo's grade as a colored badge with "(auto)" or "(manual)"
- Sidebar grade counts: update reactively from the store

**Commit:** `git commit -m "feat: add keyboard grading with visual indicators"`

---

## Phase 4: Face Detection & Clustering

### Task 11: Implement SCRFD face detection in Rust

**Files:**
- Modify: `src-tauri/src/scanner/detector.rs`
- Create: `src-tauri/src/scanner/preprocess.rs`
- Create: `src-tauri/src/scanner/postprocess.rs`

This is the most complex Rust task. Implement the SCRFD face detector:

**Preprocessing (`preprocess.rs`):**
1. Load image with `image` crate
2. Resize to 640x640 (letterbox with padding, preserve aspect ratio)
3. Convert to NCHW float32 tensor: `(pixel - 127.5) / 128.0`
4. Return the tensor as `ndarray::Array4<f32>` + the scale/padding offsets for bbox mapping

**Inference (`detector.rs`):**
1. Load `det_10g.onnx` model using `ort::Session`
2. Run inference: input tensor → 9 output tensors (3 strides × {scores, bboxes, keypoints})

**Postprocessing (`postprocess.rs`):**
1. Decode bounding boxes from anchor centers + distance predictions at strides [8, 16, 32]
2. Decode 5-point facial keypoints similarly
3. Apply confidence threshold (0.5)
4. Apply NMS with IoU threshold 0.4
5. Map coordinates back to original image space (undo letterbox scaling)

Reference: the existing `prabhat0206/scrfd` Rust crate implements this exact pipeline. Port its approach.

**Test:** Unit test with a test image, verify detected face count and bbox sanity.

**Commit:** `git commit -m "feat: implement SCRFD face detection in Rust"`

---

### Task 12: Implement ArcFace embedding + face alignment

**Files:**
- Modify: `src-tauri/src/scanner/embedder.rs`
- Create: `src-tauri/src/scanner/alignment.rs`

**Face alignment (`alignment.rs`):**
1. Given 5 facial keypoints from SCRFD, compute a similarity transform that maps them to the ArcFace template landmarks:
   ```
   [[38.2946, 51.6963], [73.5318, 51.5014], [56.0252, 71.7366], [41.5493, 92.3655], [70.7299, 92.2041]]
   ```
2. Use `nalgebra` to solve the least-squares similarity transform (rotation + scale + translation)
3. Apply the affine warp using `imageproc::geometric_transformations::warp` to produce a 112×112 aligned face crop

**Embedding (`embedder.rs`):**
1. Load `w600k_r50.onnx` model using `ort::Session`
2. Preprocess aligned face: NCHW float32, `(pixel - 127.5) / 127.5`
3. Run inference → 512-d embedding vector
4. L2-normalize the embedding

**Test:** Unit test — detect face in test image, align, embed, verify embedding is 512-d and unit-norm.

**Commit:** `git commit -m "feat: implement ArcFace face embedding with alignment"`

---

### Task 13: Implement DBSCAN clustering

**Files:**
- Modify: `src-tauri/src/scanner/cluster.rs`

1. Collect all L2-normalized 512-d embeddings across all photos
2. Build an `ndarray::Array2<f64>` (n_faces × 512) matrix
3. Run `linfa_clustering::Dbscan` with Euclidean distance, eps=0.75 (adjusted for Euclidean on unit vectors — `sqrt(2 * (1 - cos_threshold))` where cos_threshold ≈ 0.72 → eps ≈ 0.75), min_samples=2
4. Map cluster labels back to face detections
5. For each cluster, find the representative face (highest confidence)
6. Generate face thumbnails for cluster representatives (crop + resize to 80×80)

**Test:** Unit test with synthetic embeddings — verify clustering groups similar vectors together.

**Commit:** `git commit -m "feat: implement DBSCAN face clustering"`

---

### Task 14: Wire face detection pipeline + Tauri commands

**Files:**
- Modify: `src-tauri/src/scanner/mod.rs`
- Modify: `src-tauri/src/commands.rs`
- Modify: `src-tauri/src/lib.rs`

Create the full scanning pipeline:
```rust
pub async fn scan_faces(
    project: &mut Project,
    on_progress: Channel<ProgressPayload>,
) -> Result<(), String> {
    let detector = Detector::new("models/det_10g.onnx")?;
    let embedder = Embedder::new("models/w600k_r50.onnx")?;

    // Phase 1: Detect + embed all faces (with progress)
    for (i, photo) in project.photos.iter_mut().enumerate() {
        let faces = detector.detect(&photo.path)?;
        let filtered = filter_faces(&faces, 0.5, 80);
        for face in &filtered {
            let aligned = align_face(&photo.path, &face.keypoints)?;
            let embedding = embedder.embed(&aligned)?;
            photo.faces.push(FaceDetection {
                bbox: face.bbox,
                confidence: face.confidence,
                embedding: embedding.to_vec(),
                cluster_id: None,
            });
        }
        on_progress.send(ProgressPayload { current: i + 1, total: project.photos.len(), message: "Detecting faces...".into() });
    }

    // Phase 2: Cluster all faces
    let all_embeddings = collect_embeddings(project);
    let labels = cluster_faces(&all_embeddings, 0.75, 2);
    assign_clusters(project, &labels);

    // Phase 3: Generate representative thumbnails
    generate_cluster_thumbnails(project)?;

    save_project(project)?;
    Ok(())
}
```

Add `start_face_detection` Tauri command that:
1. Loads the project
2. Runs `scan_faces` with a Channel for progress
3. Returns updated project

**Commit:** `git commit -m "feat: wire face detection pipeline with progress reporting"`

---

### Task 15: ONNX model management

**Files:**
- Create: `src-tauri/src/models.rs`
- Modify: `src-tauri/src/lib.rs`

Handle model downloading/bundling:
1. On first run, check if models exist at `~/.culling/models/`
2. If not, download from InsightFace GitHub releases:
   - `det_10g.onnx` (~16MB) from buffalo_l zip
   - `w600k_r50.onnx` (~167MB) from buffalo_l zip
3. Show download progress in the UI
4. Cache models permanently

For NIMA model (Task 19), same approach but from a different source.

Alternative: bundle small models with the app binary, download large ones on first run.

**Commit:** `git commit -m "feat: add ONNX model management with auto-download"`

---

## Phase 5: People View

### Task 16: Build People sidebar (Svelte)

**Files:**
- Create: `src/lib/components/PeopleList.svelte`
- Create: `src/lib/components/PersonCard.svelte`
- Modify: `src/lib/components/Sidebar.svelte`
- Modify: `src/lib/stores/project.ts`

@ frontend-design skill:

**People section in sidebar:**
- Appears after face detection completes (before that, shows a muted "Detect faces to see people" message)
- Each person card: 48×48 circular face thumbnail + label + photo count badge
- Active person: amber left border, slightly brighter background
- Click a person → filters the filmstrip to photos containing that person
- "All Photos" option at top to clear the filter
- Editable labels: double-click the name to rename (inline text input)
- Scrollable list with shadcn ScrollArea

**Store additions:**
- `activePerson` writable store (null = show all)
- `filteredPhotos` derived store that filters by active person's cluster_id

**Commit:** `git commit -m "feat: build People sidebar with face thumbnails"`

---

### Task 17: People View — filmstrip filtering + face overlay

**Files:**
- Modify: `src/lib/components/Filmstrip.svelte`
- Modify: `src/lib/components/PhotoViewer.svelte`
- Modify: `src/lib/stores/project.ts`
- Modify: `src-tauri/src/commands.rs`

**Filmstrip filtering:**
- When a person is selected, filmstrip shows only photos containing that person
- Navigation (`←`/`→`) cycles through filtered set
- Position indicator updates: "12 / 40" (within the filtered set)

**Face overlay on photo viewer (optional but nice):**
- When in People view, show a subtle rounded rectangle outline on detected faces in the current photo
- The active person's face: amber border
- Other faces: zinc-600 border, more transparent
- Toggle-able with a keyboard shortcut (`D` for detect overlay)

**Rust command:** `rename_cluster` — update a cluster's label in the project JSON.

**Commit:** `git commit -m "feat: add People view filtering and face overlay"`

---

## Phase 6: Auto-Grading

### Task 18: Implement heuristic grading (Rust)

**Files:**
- Modify: `src-tauri/src/grader/heuristics.rs`
- Modify: `src-tauri/src/grader/mod.rs`

**Sharpness detection:**
1. Convert image to grayscale
2. Apply Laplacian filter (3×3 kernel)
3. Compute variance of the Laplacian — low variance = blurry
4. Threshold: variance < 100 → flag as likely blurry

**Exposure analysis:**
1. Compute luminance histogram (256 bins)
2. Calculate % of pixels in bottom 5% (underexposed) and top 95% (overexposed)
3. If >30% clipped either way → flag as bad exposure

**Eyes closed detection:**
1. For photos with face detections, analyze the eye landmarks
2. Eye Aspect Ratio (EAR): vertical distance between upper/lower eyelid landmarks / horizontal eye width
3. EAR < 0.2 → eyes likely closed

**Near-duplicate detection:**
1. Compare consecutive photos by cosine similarity of their face embeddings (or a simple perceptual hash if no faces)
2. Similarity > 0.95 → flag the lower-scoring one as duplicate

Each heuristic returns a score. Combine: if any heuristic flags "bad" → grade as Bad (auto).

**Test:** Unit tests for each heuristic with known-good and known-bad test images.

**Commit:** `git commit -m "feat: implement heuristic image quality grading"`

---

### Task 19: Implement NIMA aesthetic scoring (Rust)

**Files:**
- Modify: `src-tauri/src/grader/aesthetic.rs`
- Modify: `src-tauri/src/grader/mod.rs`
- Modify: `src-tauri/src/models.rs`

1. Source a NIMA model in ONNX format (~25MB). The MobileNetV2-based NIMA is widely available as ONNX.
2. Load with `ort::Session`
3. Preprocess: resize to 224×224, normalize to ImageNet mean/std
4. Run inference → 10-element distribution (scores 1-10)
5. Compute weighted mean as the aesthetic score

**Grading thresholds:**
- Photos that passed heuristics get NIMA scored
- NIMA < 5.0 → OK (auto)
- NIMA >= 5.0 → Good (auto)

**Test:** Unit test with a test image, verify score is in [1, 10] range.

**Commit:** `git commit -m "feat: implement NIMA aesthetic scoring"`

---

### Task 20: Wire auto-grading pipeline + UI

**Files:**
- Modify: `src-tauri/src/grader/mod.rs`
- Modify: `src-tauri/src/commands.rs`
- Modify: `src/lib/components/Toolbar.svelte`
- Modify: `src/lib/stores/project.ts`

**Rust pipeline:**
```rust
pub async fn auto_grade(
    project: &mut Project,
    on_progress: Channel<ProgressPayload>,
) -> Result<(), String> {
    for (i, photo) in project.photos.iter_mut().enumerate() {
        let heuristic_result = run_heuristics(&photo.path, &photo.faces)?;
        if heuristic_result.is_bad {
            photo.grade = Grade::Bad;
            photo.grade_source = GradeSource::Auto;
        } else {
            let nima_score = score_aesthetic(&photo.path)?;
            photo.aesthetic_score = Some(nima_score);
            photo.grade = if nima_score >= 5.0 { Grade::Good } else { Grade::Ok };
            photo.grade_source = GradeSource::Auto;
        }
        photo.sharpness_score = Some(heuristic_result.sharpness);
        on_progress.send(ProgressPayload { current: i + 1, total: project.photos.len(), message: "Grading photos...".into() });
    }
    save_project(project)?;
    Ok(())
}
```

Add `start_auto_grade` Tauri command.

**UI:**
- "Auto-Grade" button in toolbar triggers the command
- Progress bar in toolbar: `Grading photos... 89/528`
- On completion, toast notification: "Auto-grading complete — 12 bad, 340 ok, 50 good"
- Grade counts in sidebar update reactively
- Filmstrip thumbnails gain their colored grade bars

**Commit:** `git commit -m "feat: wire auto-grading pipeline with UI progress"`

---

## Phase 7: Export & Cleanup

### Task 21: Build export dialog (Svelte)

**Files:**
- Create: `src/lib/components/ExportDialog.svelte`
- Modify: `src/lib/stores/keyboard.ts`

@ frontend-design skill:

**Export dialog** (triggered by `Cmd+E`):
- shadcn Dialog, dark overlay
- Radio group: "All (490)" / "OK + Good (478)" / "Good only (50)" — counts computed from store
- Radio group: "Flat folder" / "By person"
- Output path selector (opens native folder dialog)
- Checkbox: "Move Bad photos to Trash"
- Cancel + Export button (amber accent, shows count: "Export 478 photos")

**Commit:** `git commit -m "feat: build export dialog"`

---

### Task 22: Implement export + trash (Rust)

**Files:**
- Modify: `src-tauri/src/organizer/export.rs`
- Modify: `src-tauri/src/organizer/mod.rs`
- Modify: `src-tauri/src/commands.rs`

**Export command:**
1. Takes: output_dir, grade_filter (All/OkGood/GoodOnly), organize_by (Flat/ByPerson), trash_bad (bool)
2. Filter photos by grade according to filter (Ungraded counts as OK)
3. If flat: copy all to output_dir
4. If by-person: create subdirectories named by cluster labels, copy photos. Photos with no face go to "ungrouped/"
5. If trash_bad: use `trash` crate to move Bad-graded photos to system trash
6. Stream progress via Channel

**Test:** Integration test that creates a mock project, runs export, verifies folder structure.

**Commit:** `git commit -m "feat: implement export with by-person organization and trash"`

---

## Phase 8: Polish

### Task 23: Keyboard shortcut overlay + fullscreen

**Files:**
- Create: `src/lib/components/ShortcutOverlay.svelte`
- Modify: `src/lib/stores/keyboard.ts`
- Modify: `src/lib/components/PhotoViewer.svelte`

**Shortcut overlay:**
- Press `?` to toggle a shadcn Dialog showing all keyboard shortcuts
- Organized by section: Navigation, Grading, Views, Actions

**Fullscreen:**
- Press `F` to toggle fullscreen photo view (hides sidebar, toolbar, filmstrip — just the photo on black)
- Press `Escape` or `F` again to exit
- Grade shortcuts still work in fullscreen

**Commit:** `git commit -m "feat: add keyboard shortcut overlay and fullscreen mode"`

---

### Task 24: Settings panel

**Files:**
- Create: `src/lib/components/SettingsDialog.svelte`
- Create: `src-tauri/src/settings.rs`
- Modify: `src-tauri/src/commands.rs`

**Settings (persisted to `~/.culling/settings.json`):**
- Face detection confidence threshold (slider, default 0.5)
- Minimum face size (slider, default 80px)
- DBSCAN epsilon (slider, default 0.75)
- Sharpness threshold (slider, default 100)
- NIMA good/ok cutoff (slider, default 5.0)
- Exposure clipping threshold (slider, default 30%)

shadcn Dialog with shadcn Sliders, organized in sections.

**Commit:** `git commit -m "feat: add settings panel with tunable thresholds"`

---

### Task 25: Animations + micro-interactions

**Files:**
- Modify: various Svelte components

@ frontend-design skill:

- **Photo transition:** Smooth crossfade (200ms) when navigating between photos. Use Svelte transitions.
- **Filmstrip scroll:** Smooth scroll to center current photo. Slight parallax effect on the active thumbnail.
- **Grade change:** Brief color flash on the filmstrip thumbnail when graded (pulse animation on the bottom bar).
- **People sidebar:** Face thumbnails fade in as face detection discovers them.
- **Progress bars:** Subtle gradient shimmer animation while in progress.
- **Toast notifications:** Slide in from bottom-right, auto-dismiss after 3s.
- **Import drop zone:** Gentle breathing animation on the dashed border.

Keep all animations performant — CSS transitions and Svelte `transition:` directives, no heavy JS animation libraries.

**Commit:** `git commit -m "feat: add animations and micro-interactions"`

---

### Task 26: Final review, README, and packaging

**Step 1:** Read through all code end-to-end. Ensure:
- No `todo!()` stubs remaining
- All Tauri commands are registered
- All keyboard shortcuts work
- Export flow works end-to-end
- Project persistence works (close and reopen)

**Step 2:** Create a `README.md` with:
- Project description and screenshot
- Installation instructions (building from source)
- Usage guide (import, cull, export workflow)
- Keyboard shortcuts table
- Development setup instructions
- License (choose open source license)

**Step 3:** Configure `tauri.conf.json` for production build:
- App icons
- macOS bundle settings
- Minimum window size

**Step 4:** Test production build:
```bash
npm run tauri build
```

**Step 5:** Final commit:
```bash
git add -A
git commit -m "feat: Culling v0.1.0 — photo culling app with face detection"
```
