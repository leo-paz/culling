<script lang="ts">
  import { convertFileSrc } from '@tauri-apps/api/core';
  import { fade } from 'svelte/transition';
  import { currentPhoto, filteredCount } from '$lib/stores/project';

  // Derive src directly from current photo — pure derivation, no side effects
  const currentSrc = $derived(
    $currentPhoto ? convertFileSrc($currentPhoto.path) : ''
  );

  // Track which src has finished loading via the onload event — no effect needed.
  // When currentSrc changes (new photo), isLoaded becomes false until onload fires.
  let loadedSrc = $state('');
  const isLoaded = $derived(loadedSrc === currentSrc && currentSrc !== '');
</script>

<div class="relative flex items-center justify-center bg-surface overflow-hidden w-full h-full">
  {#if $filteredCount === 0}
    <div class="text-zinc-500 text-sm">No photos to display</div>
  {:else if $currentPhoto}
    {#key $currentPhoto.path}
      <div
        class="absolute inset-0 flex items-center justify-center"
        in:fade={{ duration: 150 }}
      >
        <!-- Loading skeleton -->
        {#if !isLoaded}
          <div class="absolute inset-0 flex items-center justify-center">
            <div class="w-16 h-16 rounded-lg bg-surface-raised animate-pulse"></div>
          </div>
        {/if}

        <!-- Photo -->
        <img
          src={currentSrc}
          alt={$currentPhoto.filename}
          class="max-w-full max-h-full object-contain transition-opacity duration-200 select-none"
          style:opacity={isLoaded ? 1 : 0}
          onload={() => { loadedSrc = currentSrc; }}
          draggable="false"
        />
      </div>
    {/key}
  {:else}
    <div class="text-zinc-600 text-sm">No photo selected</div>
  {/if}
</div>
