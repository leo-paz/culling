<script lang="ts">
  import { invoke, convertFileSrc } from '@tauri-apps/api/core';
  import { SvelteMap, SvelteSet } from 'svelte/reactivity';
  import { currentProject, currentIndex, navigateTo, filteredPhotos, type Photo } from '$lib/stores/project';

  let scrollContainer: HTMLDivElement | undefined = $state();
  let thumbnailPaths = $state<SvelteMap<string, string>>(new SvelteMap());

  // Grade pulse tracking — use a plain object ref to avoid reactive read/write loop
  let gradeCache: Record<string, Photo['grade']> = {};
  let pulsingPhotos = $state<SvelteSet<string>>(new SvelteSet());

  // Load thumbnail URLs — use convertFileSrc for asset protocol
  // Only re-run when project ID changes (not on every photo grade change)
  let lastProjectId = '';
  $effect(() => {
    const project = $currentProject;
    if (!project || project.id === lastProjectId) return;
    lastProjectId = project.id;

    const loadThumbnails = async () => {
      const paths = new SvelteMap<string, string>();
      for (const photo of project.photos) {
        // Use full image via asset protocol (thumbnails may not be ready)
        paths.set(photo.filename, convertFileSrc(photo.path));
      }
      thumbnailPaths = paths;

      // Then try to upgrade to actual thumbnails
      for (const photo of project.photos) {
        try {
          const thumbPath = await invoke<string>('get_thumbnail_path', {
            projectId: project.id,
            filename: photo.filename,
          });
          paths.set(photo.filename, convertFileSrc(thumbPath));
        } catch {
          // Keep full image fallback
        }
      }
    };

    loadThumbnails();
  });

  // Detect grade changes and trigger pulse — use untrack to avoid loop
  $effect(() => {
    const photos = $filteredPhotos;
    const newPulsing = new SvelteSet<string>();

    for (const photo of photos) {
      const prev = gradeCache[photo.path];
      if (prev !== undefined && prev !== photo.grade) {
        newPulsing.add(photo.path);
      }
      gradeCache[photo.path] = photo.grade;
    }

    if (newPulsing.size > 0) {
      pulsingPhotos = newPulsing;
      setTimeout(() => {
        pulsingPhotos = new SvelteSet();
      }, 400);
    }
  });

  // Auto-scroll to current thumbnail
  $effect(() => {
    const index = $currentIndex;
    if (scrollContainer) {
      const thumbnails = scrollContainer.querySelectorAll('[data-thumbnail]');
      const current = thumbnails[index];
      if (current) {
        current.scrollIntoView({
          behavior: 'smooth',
          block: 'nearest',
          inline: 'center',
        });
      }
    }
  });

  function gradeColor(grade: Photo['grade']): string {
    switch (grade) {
      case 'Bad': return 'bg-grade-bad';
      case 'Ok': return 'bg-grade-ok';
      case 'Good': return 'bg-grade-good';
      default: return 'bg-zinc-700';
    }
  }

  function handleClick(index: number) {
    navigateTo(index);
  }
</script>

<div class="bg-surface-raised border-t border-zinc-800 h-[130px] flex flex-col">
  <div
    bind:this={scrollContainer}
    class="flex-1 flex items-center gap-1.5 px-3 overflow-x-auto scrollbar-thin"
    style="scrollbar-width: thin; scrollbar-color: #3f3f46 transparent;"
  >
    {#if $currentProject}
      {#each $filteredPhotos as photo, index (photo.path)}
        <button
          data-thumbnail
          class="relative flex-shrink-0 w-[88px] h-[100px] rounded overflow-hidden cursor-pointer transition-all duration-150 group"
          class:ring-2={index === $currentIndex}
          class:ring-accent={index === $currentIndex}
          class:scale-105={index === $currentIndex}
          class:opacity-70={index !== $currentIndex}
          class:hover:opacity-100={index !== $currentIndex}
          class:hover:scale-102={index !== $currentIndex}
          style:border={index !== $currentIndex ? '1px solid #3f3f46' : 'none'}
          onclick={() => handleClick(index)}
        >
          {#if thumbnailPaths.get(photo.filename)}
            <img
              src={thumbnailPaths.get(photo.filename)}
              alt={photo.filename}
              class="w-full h-full object-cover select-none"
              draggable="false"
              loading="lazy"
            />
          {:else}
            <div class="w-full h-full bg-surface-overlay animate-pulse"></div>
          {/if}

          <!-- Grade indicator bar -->
          <div
            class="absolute bottom-0 left-0 right-0 h-[3px] {gradeColor(photo.grade)}"
            class:grade-pulse={pulsingPhotos.has(photo.path)}
          ></div>
        </button>
      {/each}
    {/if}
  </div>
</div>

<style>
  @keyframes grade-pulse {
    0% { opacity: 1; transform: scaleX(1); }
    50% { opacity: 0.5; transform: scaleX(1.05); }
    100% { opacity: 1; transform: scaleX(1); }
  }

  .grade-pulse {
    animation: grade-pulse 0.4s ease-in-out;
  }
</style>
