<script lang="ts">
  import Sidebar from './Sidebar.svelte';
  import Toolbar from './Toolbar.svelte';
  import PhotoViewer from './PhotoViewer.svelte';
  import Filmstrip from './Filmstrip.svelte';
  import StatusBar from './StatusBar.svelte';
  import ThumbnailProgress from './ThumbnailProgress.svelte';
  import ExportDialog from './ExportDialog.svelte';
  import { fullscreen, currentPhoto, filteredCount, currentIndex } from '$lib/stores/project';

  let exportDialog: ReturnType<typeof ExportDialog> | undefined = $state();

  export function openExport() {
    exportDialog?.show();
  }

  // Fullscreen grade flash — shows briefly when grade changes
  let gradeFlash = $state<string | null>(null);
  let gradeFlashTimeout: ReturnType<typeof setTimeout> | null = null;
  let prevGrade = '';

  $effect(() => {
    const grade = $currentPhoto?.grade ?? '';
    if (prevGrade && grade !== prevGrade && $fullscreen) {
      gradeFlash = grade;
      if (gradeFlashTimeout) clearTimeout(gradeFlashTimeout);
      gradeFlashTimeout = setTimeout(() => { gradeFlash = null; }, 1200);
    }
    prevGrade = grade;
  });

  function gradeColor(grade: string): string {
    switch (grade) {
      case 'Bad': return 'bg-grade-bad';
      case 'Ok': return 'bg-grade-ok';
      case 'Good': return 'bg-grade-good';
      default: return 'bg-zinc-500';
    }
  }
</script>

{#if $fullscreen}
  <!-- Fullscreen: photo only, black background -->
  <div class="h-screen w-screen bg-black relative">
    <PhotoViewer />

    <!-- Subtle grade indicator in fullscreen -->
    {#if gradeFlash}
      <div class="absolute top-6 left-1/2 -translate-x-1/2 flex items-center gap-2 px-4 py-2 rounded-full bg-black/60 backdrop-blur-sm animate-fade-out">
        <span class="w-2.5 h-2.5 rounded-full {gradeColor(gradeFlash)}"></span>
        <span class="text-sm text-zinc-200 font-medium">{gradeFlash}</span>
      </div>
    {/if}

    <!-- Position counter in fullscreen -->
    <div class="absolute bottom-4 left-1/2 -translate-x-1/2 text-xs text-zinc-500 font-mono bg-black/40 px-3 py-1 rounded-full">
      {$currentIndex + 1} / {$filteredCount}
    </div>
  </div>
{:else}
  <div class="h-screen w-screen grid overflow-hidden" style="grid-template-columns: 260px 1fr; grid-template-rows: 48px 1fr 32px;">
    <!-- Sidebar spans all rows -->
    <div class="row-span-3 h-full overflow-hidden">
      <Sidebar />
    </div>

    <!-- Toolbar -->
    <div>
      <Toolbar onexport={() => exportDialog?.show()} />
    </div>

    <!-- Main content: Photo viewer + Filmstrip -->
    <div class="flex flex-col overflow-hidden relative">
      <div class="flex-1 min-h-0">
        <PhotoViewer />
      </div>
      <Filmstrip />
      <ThumbnailProgress />
    </div>

    <!-- Status bar -->
    <div>
      <StatusBar />
    </div>
  </div>
{/if}

<ExportDialog bind:this={exportDialog} />
