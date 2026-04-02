<script lang="ts">
  import * as Tabs from '$lib/components/ui/tabs';
  import * as Tooltip from '$lib/components/ui/tooltip';
  import { Button } from '$lib/components/ui/button';
  import { viewMode, currentProject, activePerson, currentIndex, enrichmentStatus } from '$lib/stores/project';

  let { onexport = () => {} }: { onexport?: () => void } = $props();

  let hasClusters = $derived(
    ($currentProject?.clusters?.length ?? 0) > 0
  );

  // When switching back to timeline, reset person filter.
  // prevViewMode is a plain variable (not $state) to avoid read/write loop in the effect.
  let prevViewMode = $viewMode;
  $effect(() => {
    const mode = $viewMode;
    if (mode === 'timeline' && prevViewMode !== 'timeline') {
      activePerson.set(null);
      currentIndex.set(0);
    }
    prevViewMode = mode;
  });
</script>

<header class="relative flex items-center justify-between bg-surface-raised border-b border-zinc-800 px-3 h-12">
  <!-- Left: View Mode Tabs -->
  <div class="flex items-center">
    <Tabs.Root bind:value={$viewMode}>
      <Tabs.List class="bg-surface-overlay h-7">
        <Tabs.Trigger value="timeline" class="text-xs px-3 h-6">Timeline</Tabs.Trigger>
        <Tabs.Trigger value="people" class="text-xs px-3 h-6" disabled={!hasClusters}>People</Tabs.Trigger>
      </Tabs.List>
    </Tabs.Root>
  </div>

  <!-- Center: Enrichment Status -->
  <div class="flex items-center gap-2">
    {#if $enrichmentStatus.stage}
      <span class="text-xs text-zinc-500">
        {$enrichmentStatus.stage === 'thumbnails' ? 'Generating thumbnails' :
         $enrichmentStatus.stage === 'grading' ? 'Grading' :
         $enrichmentStatus.stage === 'faces' ? 'Detecting faces' : ''}...
        {$enrichmentStatus.current}/{$enrichmentStatus.total}
      </span>
    {/if}
  </div>

  <!-- Right: Export -->
  <div class="flex items-center">
    <Tooltip.Root>
      <Tooltip.Trigger>
        <Button
          variant="outline"
          size="sm"
          class="text-xs border-zinc-700 text-zinc-300 hover:text-zinc-100"
          disabled={!$currentProject}
          onclick={onexport}
        >
          Export
        </Button>
      </Tooltip.Trigger>
      <Tooltip.Content>
        <p>Export curated photos (Cmd+E)</p>
      </Tooltip.Content>
    </Tooltip.Root>
  </div>
</header>
