<script lang="ts">
  import { Progress } from '$lib/components/ui/progress';
  import { thumbnailProgress } from '$lib/stores/project';
</script>

{#if $thumbnailProgress !== null}
  <div class="absolute inset-0 z-50 flex items-center justify-center bg-surface/80 backdrop-blur-sm">
    <div class="flex flex-col items-center gap-4 p-6 rounded-lg bg-surface-raised border border-zinc-800">
      <div class="text-sm text-zinc-300 font-medium">Generating thumbnails...</div>
      <div class="w-64">
        <Progress value={$thumbnailProgress} max={100} class="h-1.5 bg-zinc-800" />
      </div>
      <div class="text-xs text-zinc-500 font-mono">{Math.round($thumbnailProgress)}%</div>
    </div>
  </div>
{/if}
