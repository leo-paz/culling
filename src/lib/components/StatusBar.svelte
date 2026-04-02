<script lang="ts">
  import { currentPhoto, currentIndex, filteredCount } from '$lib/stores/project';
  import type { Photo } from '$lib/stores/project';

  function gradeColor(grade: Photo['grade']): string {
    switch (grade) {
      case 'Bad': return 'bg-grade-bad';
      case 'Ok': return 'bg-grade-ok';
      case 'Good': return 'bg-grade-good';
      default: return 'bg-zinc-600';
    }
  }

  function gradeLabel(grade: Photo['grade']): string {
    switch (grade) {
      case 'Bad': return 'Bad';
      case 'Ok': return 'OK';
      case 'Good': return 'Good';
      default: return 'Ungraded';
    }
  }

  function gradeLabelColor(grade: Photo['grade']): string {
    switch (grade) {
      case 'Bad': return 'text-grade-bad';
      case 'Ok': return 'text-grade-ok';
      case 'Good': return 'text-grade-good';
      default: return 'text-zinc-500';
    }
  }
</script>

<footer class="flex items-center justify-between bg-surface border-t border-zinc-800 h-8 px-4 text-xs">
  <!-- Left: Grade badge -->
  <div class="flex items-center gap-2 min-w-0">
    {#if $currentPhoto}
      <span class="w-2 h-2 rounded-full flex-shrink-0 {gradeColor($currentPhoto.grade)}"></span>
      <span class="font-medium {gradeLabelColor($currentPhoto.grade)}">
        {gradeLabel($currentPhoto.grade)}
      </span>
      <span class="text-zinc-600">
        ({$currentPhoto.grade_source === 'Auto' ? 'auto' : 'manual'})
      </span>
      {#if $currentPhoto.grade_reason}
        <span class="text-zinc-600 ml-1 truncate max-w-[250px]" title={$currentPhoto.grade_reason}>
          — {$currentPhoto.grade_reason}
        </span>
      {/if}
    {/if}
  </div>

  <!-- Center: Position indicator -->
  <div class="text-zinc-500 font-mono">
    {#if $filteredCount > 0}
      {$currentIndex + 1} / {$filteredCount}
    {/if}
  </div>

  <!-- Right: Filename -->
  <div class="text-zinc-500 font-mono truncate max-w-[200px] text-right">
    {$currentPhoto?.filename ?? ''}
  </div>
</footer>
