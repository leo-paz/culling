<script lang="ts">
  import { thumbnailProgress } from '$lib/stores/project';
</script>

{#if $thumbnailProgress !== null}
  <!-- Subtle progress bar at the top of the main content area — non-blocking -->
  <div class="absolute top-0 left-0 right-0 z-10">
    <div
      class="h-0.5 bg-accent transition-all duration-300 ease-out"
      style="width: {$thumbnailProgress}%"
    ></div>
  </div>
{/if}
