import { writable, derived, get } from 'svelte/store';
import { invoke } from '@tauri-apps/api/core';

// Types mirroring Rust backend
export interface FaceDetection {
  bbox: [number, number, number, number];
  confidence: number;
  cluster_id: number | null;
}

export interface Photo {
  path: string;
  filename: string;
  grade: 'Ungraded' | 'Bad' | 'Ok' | 'Good';
  grade_source: 'Manual' | 'Auto';
  faces: FaceDetection[];
  aesthetic_score: number | null;
  sharpness_score: number | null;
  grade_reason: string | null;
  content_hash: string | null;
  graded_at: number | null;
  faces_detected_at: number | null;
}

export interface EnrichmentStatus {
  stage: 'thumbnails' | 'grading' | 'faces' | 'downloading' | null;
  current: number;
  total: number;
}

export interface Cluster {
  id: number;
  label: string;
  representative_photo: string;
  representative_bbox: [number, number, number, number];
  photo_count: number;
}

export interface Project {
  id: string;
  name: string;
  source_dir: string;
  photos: Photo[];
  clusters: Cluster[];
}

// Stores
export const currentProject = writable<Project | null>(null);
export const currentIndex = writable<number>(0);
export const viewMode = writable<'timeline' | 'people'>('timeline');
export const activePerson = writable<number | null>(null);
export const thumbnailProgress = writable<number | null>(null);
export const fullscreen = writable<boolean>(false);
export const enrichmentStatus = writable<EnrichmentStatus>({ stage: null, current: 0, total: 0 });

// Derived stores

// Photos filtered by active person (or all if no person selected)
export const filteredPhotos = derived(
  [currentProject, activePerson],
  ([$project, $person]) => {
    if (!$project) return [];
    if ($person === null) return $project.photos;
    return $project.photos.filter((p) =>
      p.faces.some((f) => f.cluster_id === $person)
    );
  }
);

export const filteredCount = derived(filteredPhotos, ($photos) => $photos.length);

export const currentPhoto = derived(
  [filteredPhotos, currentIndex],
  ([$photos, $index]) => $photos[$index] ?? null
);

export const gradeCounts = derived(currentProject, ($project) => {
  if (!$project) return { bad: 0, ok: 0, good: 0, ungraded: 0 };
  const counts = { bad: 0, ok: 0, good: 0, ungraded: 0 };
  for (const p of $project.photos) {
    if (p.grade === 'Bad') counts.bad++;
    else if (p.grade === 'Ok') counts.ok++;
    else if (p.grade === 'Good') counts.good++;
    else counts.ungraded++;
  }
  return counts;
});

export const totalPhotos = derived(currentProject, ($p) => $p?.photos.length ?? 0);

// Navigation helpers
export function navigateNext() {
  const total = get(filteredCount);
  if (total === 0) return;
  currentIndex.update((i) => Math.min(i + 1, total - 1));
}

export function navigatePrev() {
  currentIndex.update((i) => Math.max(i - 1, 0));
}

export function navigateTo(index: number) {
  const total = get(filteredCount);
  if (total === 0) return;
  currentIndex.set(Math.max(0, Math.min(index, total - 1)));
}

/** Poll the project from disk every 3 seconds to pick up enrichment progress.
 *  Call this after starting enrichment. Stops when all processing is done.
 *  Only updates photos that actually changed (grade/faces) to avoid
 *  re-triggering the photo viewer's {#key} block. */
let _pollInterval: ReturnType<typeof setInterval> | null = null;
export function startEnrichmentPolling(projectId: string) {
  if (_pollInterval) clearInterval(_pollInterval);
  _pollInterval = setInterval(async () => {
    try {
      const updated = await invoke<Project>('get_project', { id: projectId });

      // Merge changes into existing project without replacing the whole object.
      // This prevents the PhotoViewer from re-mounting the <img> on every poll.
      currentProject.update((prev) => {
        if (!prev) return updated;
        // Update photo count if new photos were added
        if (updated.photos.length !== prev.photos.length) {
          return updated; // structural change, must replace
        }
        // Patch individual photo grades/scores without replacing the array
        let changed = false;
        const photos = prev.photos.map((p, i) => {
          const u = updated.photos[i];
          if (!u || p.path !== u.path) return p; // different photo, skip
          if (p.grade !== u.grade || p.faces.length !== u.faces.length ||
              p.graded_at !== u.graded_at || p.faces_detected_at !== u.faces_detected_at) {
            changed = true;
            return u;
          }
          return p;
        });
        if (!changed && prev.clusters.length === updated.clusters.length) return prev;
        return { ...prev, photos, clusters: updated.clusters };
      });

      const needsGrading = updated.photos.filter(p => !p.graded_at && p.grade_source !== 'Manual').length;
      const needsFaces = updated.photos.filter(p => !p.faces_detected_at).length;
      const total = updated.photos.length;

      if (needsGrading > 0) {
        enrichmentStatus.set({ stage: 'grading', current: total - needsGrading, total });
      } else if (needsFaces > 0) {
        enrichmentStatus.set({ stage: 'faces', current: total - needsFaces, total });
      } else {
        enrichmentStatus.set({ stage: null, current: 0, total: 0 });
        if (_pollInterval) { clearInterval(_pollInterval); _pollInterval = null; }
      }
    } catch {
      // Project might be getting written — skip this poll
    }
  }, 3000);
}

export function updatePhotoGrade(photoPath: string, grade: Photo['grade']) {
  currentProject.update((project) => {
    if (!project) return project;
    const photos = project.photos.map((p) =>
      p.path === photoPath
        ? { ...p, grade, grade_source: 'Manual' as const }
        : p
    );
    return { ...project, photos };
  });
}
