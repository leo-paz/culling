<script lang="ts">
  import { invoke, convertFileSrc } from '@tauri-apps/api/core';
  import { currentProject, currentIndex, navigateTo, filteredPhotos, type Photo } from '$lib/stores/project';

  let scrollContainer: HTMLDivElement | undefined = $state();
  let thumbnailPaths = $state<Map<string, string>>(new Map());

  // Load thumbnail paths for all photos
  $effect(() => {
    const project = $currentProject;
    if (!project) return;

    const loadThumbnails = async () => {
      const paths = new Map<string, string>();
      for (const photo of project.photos) {
        try {
          const thumbPath = await invoke<string>('get_thumbnail_path', {
            projectId: project.id,
            filename: photo.filename,
          });
          paths.set(photo.filename, convertFileSrc(thumbPath));
        } catch {
          // Fallback to full image if thumbnail not available
          paths.set(photo.filename, convertFileSrc(photo.path));
        }
      }
      thumbnailPaths = paths;
    };

    loadThumbnails();
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
      {#each $filteredPhotos as photo, index}
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
          <div class="absolute bottom-0 left-0 right-0 h-[3px] {gradeColor(photo.grade)}"></div>
        </button>
      {/each}
    {/if}
  </div>
</div>
