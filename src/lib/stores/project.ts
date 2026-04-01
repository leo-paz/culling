import { writable, derived, get } from 'svelte/store';

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
