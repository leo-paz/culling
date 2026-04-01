<script lang="ts">
  import { convertFileSrc } from '@tauri-apps/api/core';
  import { currentPhoto } from '$lib/stores/project';

  let imageLoaded = $state(false);
  let currentSrc = $state('');

  // Track photo changes and manage loading state
  $effect(() => {
    const photo = $currentPhoto;
    if (photo) {
      const newSrc = convertFileSrc(photo.path);
      if (newSrc !== currentSrc) {
        imageLoaded = false;
        currentSrc = newSrc;
      }
    } else {
      currentSrc = '';
      imageLoaded = false;
    }
  });

  function handleImageLoad() {
    imageLoaded = true;
  }
</script>

<div class="relative flex items-center justify-center bg-surface overflow-hidden w-full h-full">
  {#if $currentPhoto}
    <!-- Loading skeleton -->
    {#if !imageLoaded}
      <div class="absolute inset-0 flex items-center justify-center">
        <div class="w-16 h-16 rounded-lg bg-surface-raised animate-pulse"></div>
      </div>
    {/if}

    <!-- Photo -->
    <img
      src={currentSrc}
      alt={$currentPhoto.filename}
      class="max-w-full max-h-full object-contain transition-opacity duration-200 select-none"
      style:opacity={imageLoaded ? 1 : 0}
      onload={handleImageLoad}
      draggable="false"
    />
  {:else}
    <div class="text-zinc-600 text-sm">No photo selected</div>
  {/if}
</div>
