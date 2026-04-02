# Background Enrichment Pipeline — Design

## Problem

Grading and face detection are currently manual button clicks. They should be automatic background processes that run whenever new photos enter the system, with incremental processing and crash recovery.

## Solution

An automatic enrichment pipeline that runs on project open/import. No user-facing buttons — just subtle status indicators. Only processes photos that need it (new, changed, or unprocessed).

## Data Model Changes

Add to `Photo`:
- `content_hash: Option<String>` — SHA-256 of first 64KB of file. Detects file modifications.
- `graded_at: Option<u64>` — Unix timestamp when grading completed. None = needs grading.
- `faces_detected_at: Option<u64>` — Unix timestamp when face detection completed. None = needs detection.

### Processing Criteria

A photo needs grading if:
- `graded_at.is_none()` (never graded), OR
- `content_hash` changed since last grading (file was modified)
- AND `grade_source != Manual` (never overwrite manual grades)

A photo needs face detection if:
- `faces_detected_at.is_none()` (never processed)
- AND ONNX models are available on disk

## Pipeline Architecture

```
on_project_open / on_import:
  1. scan_for_new_photos(folder)     — diff file list, add new, flag removed, update hashes
  2. spawn background enrichment:
     a. generate_thumbnails()        — photos missing thumbnails
     b. auto_grade()                 — photos where graded_at is None
     c. detect_faces() + cluster()   — photos where faces_detected_at is None
                                       (silently skip if models unavailable)
```

### Incremental Processing

- Each step only processes photos that need it
- Project saves after every batch (~10 photos) for crash recovery
- On crash/reopen, pipeline picks up where it left off
- Each step runs sequentially (not parallel) to avoid resource contention

### Progress Reporting

Uses Tauri events (not channels) since the pipeline runs detached from any single command:
- `enrichment:progress` event with `{ stage: "grading" | "faces" | "thumbnails", current, total }`
- `enrichment:complete` event when all stages finish
- `enrichment:error` event for non-fatal errors (individual photo failures)

## Frontend Changes

- Remove "Detect Faces" and "Auto-Grade" buttons from toolbar
- Add subtle status text in toolbar center: `Grading... 42/229` → `Detecting faces... 100/229` → empty when done
- Grade badges on filmstrip populate incrementally as grading completes
- People sidebar populates when face detection + clustering finishes
- No user interaction required — fully automatic

## Reopen/Rescan Flow

When opening an existing project:
1. Re-scan source folder
2. Diff against project photos:
   - New files → add as Photo with all fields empty
   - Missing files → mark photo as `missing: true` (dimmed in UI, not reprocessed)
   - Existing files → compare content_hash. If changed, reset `graded_at` and `faces_detected_at`
3. Run enrichment pipeline on anything that needs processing

## Edge Cases

| Scenario | Behavior |
|----------|----------|
| Models not installed | Face detection silently skips. Runs when models appear on next open. |
| Crash during enrichment | Project saved every 10 photos. Pipeline resumes on reopen. |
| User manually grades during auto-grade | Manual grade wins. Pipeline checks `grade_source != Manual`. |
| File deleted from folder | Photo marked as missing, not reprocessed, dimmed in UI. |
| File modified on disk | content_hash changes, triggers re-grade and re-detect. |
| Empty folder | No photos, no enrichment. Welcome screen or empty state. |

## Rust Module Changes

- `pipeline.rs` → add `run_enrichment_pipeline()` that orchestrates all stages
- `pipeline.rs` → add `scan_for_changes()` that diffs folder vs project
- `project.rs` → add `content_hash`, `graded_at`, `faces_detected_at` to Photo
- `commands.rs` → remove `start_face_detection` and `start_auto_grade` commands
- `commands.rs` → modify `import_folder` and add `open_project` to trigger enrichment
- `lib.rs` → use `app.emit()` for progress events instead of Channel IPC

## Frontend Changes

- `Toolbar.svelte` → remove Detect Faces / Auto-Grade buttons, add status indicator
- `WelcomeScreen.svelte` → `openProject` triggers enrichment automatically  
- `+page.svelte` → listen for `enrichment:progress` events, update store
- `stores/project.ts` → add enrichment status store
