<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { Separator } from '$lib/components/ui/separator';
  import { fade } from 'svelte/transition';
  import { currentProject, gradeCounts, activePerson, currentIndex, enrichmentStatus } from '$lib/stores/project';

  let modelsAvailable = $state(false);
  $effect(() => {
    if ($currentProject) {
      invoke<boolean>('check_models').then((v) => { modelsAvailable = v; }).catch(() => {});
    }
  });
</script>

<aside class="flex flex-col h-full bg-surface-raised border-r border-zinc-800 overflow-hidden">
  <!-- Project Name -->
  <div class="px-4 py-3">
    <h2 class="text-sm font-semibold text-zinc-100 truncate font-display">
      {$currentProject?.name ?? 'Untitled'}
    </h2>
    <p class="text-xs text-zinc-500 truncate mt-0.5">
      {$currentProject?.source_dir ?? ''}
    </p>
  </div>

  <Separator class="bg-zinc-800" />

  <!-- People Section -->
  <div class="px-4 py-3 flex-1 overflow-hidden flex flex-col">
    <h3 class="text-[10px] font-medium text-zinc-400 uppercase tracking-wider mb-2">
      People
    </h3>

    {#if $currentProject?.clusters && $currentProject.clusters.length > 0}
      <!-- "All Photos" option -->
      <button
        class="w-full flex items-center gap-3 px-2 py-1.5 rounded text-xs text-left transition-colors"
        class:bg-accent-muted={$activePerson === null}
        class:text-accent={$activePerson === null}
        class:text-zinc-300={$activePerson !== null}
        class:hover:bg-zinc-800={$activePerson !== null}
        onclick={() => { activePerson.set(null); currentIndex.set(0); }}
      >
        <span class="font-medium">All Photos</span>
        <span class="ml-auto text-zinc-500 font-mono">{$currentProject.photos.length}</span>
      </button>

      <!-- Person list (scrollable) -->
      <div class="mt-1 overflow-y-auto flex-1 space-y-0.5">
        {#each $currentProject.clusters as cluster (cluster.id)}
          <div transition:fade={{ duration: 200 }}>
            <button
              class="w-full flex items-center gap-3 px-2 py-1.5 rounded text-xs text-left transition-colors"
              class:bg-accent-muted={$activePerson === cluster.id}
              class:text-accent={$activePerson === cluster.id}
              class:border-l-2={$activePerson === cluster.id}
              class:border-accent={$activePerson === cluster.id}
              class:text-zinc-300={$activePerson !== cluster.id}
              class:hover:bg-zinc-800={$activePerson !== cluster.id}
              onclick={() => { activePerson.set(cluster.id); currentIndex.set(0); }}
            >
              <!-- Face thumbnail (small circle) -->
              <div class="w-8 h-8 rounded-full bg-surface-overlay overflow-hidden flex-shrink-0">
                <div class="w-full h-full flex items-center justify-center text-zinc-500 text-[10px] font-medium">
                  {cluster.label.charAt(0).toUpperCase()}
                </div>
              </div>
              <span class="font-medium truncate">{cluster.label}</span>
              <span class="ml-auto text-zinc-500 font-mono flex-shrink-0">{cluster.photo_count}</span>
            </button>
          </div>
        {/each}
      </div>
    {:else}
      <div class="space-y-2">
        {#if $enrichmentStatus.stage === 'downloading'}
          <p class="text-xs text-zinc-400">
            Downloading face detection models...
          </p>
          <div class="h-1 bg-zinc-800 rounded overflow-hidden">
            <div class="h-full bg-accent animate-pulse" style="width: 100%"></div>
          </div>
        {:else if $enrichmentStatus.stage === 'faces'}
          <p class="text-xs text-zinc-400">
            Detecting faces... {$enrichmentStatus.current}/{$enrichmentStatus.total}
          </p>
          <div class="h-1 bg-zinc-800 rounded overflow-hidden">
            <div
              class="h-full bg-accent transition-all duration-300"
              style="width: {$enrichmentStatus.total > 0 ? ($enrichmentStatus.current / $enrichmentStatus.total * 100) : 0}%"
            ></div>
          </div>
        {:else if !modelsAvailable}
          <p class="text-xs text-zinc-500">
            Face detection will be available when connected to the internet.
          </p>
        {:else}
          <p class="text-xs text-zinc-500 italic">
            No faces detected in these photos
          </p>
        {/if}
      </div>
    {/if}
  </div>

  <Separator class="bg-zinc-800" />

  <!-- Grade Counts -->
  <div class="px-4 py-3">
    <h3 class="text-[10px] font-medium text-zinc-400 uppercase tracking-wider mb-3">
      Grades
    </h3>
    <div class="space-y-2">
      <div class="flex items-center justify-between text-xs">
        <div class="flex items-center gap-2">
          <span class="w-2 h-2 rounded-full bg-grade-bad"></span>
          <span class="text-zinc-300">Bad</span>
        </div>
        <span class="text-zinc-500 font-mono">{$gradeCounts.bad}</span>
      </div>
      <div class="flex items-center justify-between text-xs">
        <div class="flex items-center gap-2">
          <span class="w-2 h-2 rounded-full bg-grade-ok"></span>
          <span class="text-zinc-300">OK</span>
        </div>
        <span class="text-zinc-500 font-mono">{$gradeCounts.ok}</span>
      </div>
      <div class="flex items-center justify-between text-xs">
        <div class="flex items-center gap-2">
          <span class="w-2 h-2 rounded-full bg-grade-good"></span>
          <span class="text-zinc-300">Good</span>
        </div>
        <span class="text-zinc-500 font-mono">{$gradeCounts.good}</span>
      </div>
      <div class="flex items-center justify-between text-xs pt-1 border-t border-zinc-800">
        <div class="flex items-center gap-2">
          <span class="w-2 h-2 rounded-full bg-zinc-600"></span>
          <span class="text-zinc-400">Ungraded</span>
        </div>
        <span class="text-zinc-500 font-mono">{$gradeCounts.ungraded}</span>
      </div>
    </div>
  </div>

</aside>
