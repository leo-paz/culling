<script lang="ts">
  import { Button } from '$lib/components/ui/button';
  import * as Tabs from '$lib/components/ui/tabs';
  import * as Tooltip from '$lib/components/ui/tooltip';
  import { viewMode } from '$lib/stores/project';
</script>

<header class="flex items-center justify-between bg-surface-raised border-b border-zinc-800 px-3 h-12">
  <!-- Left: View Mode Tabs -->
  <div class="flex items-center">
    <Tabs.Root bind:value={$viewMode}>
      <Tabs.List class="bg-surface-overlay h-7">
        <Tabs.Trigger value="timeline" class="text-xs px-3 h-6">Timeline</Tabs.Trigger>
        <Tabs.Trigger value="people" class="text-xs px-3 h-6" disabled>People</Tabs.Trigger>
      </Tabs.List>
    </Tabs.Root>
  </div>

  <!-- Center: Action Buttons -->
  <div class="flex items-center gap-2">
    <Tooltip.Root>
      <Tooltip.Trigger>
        <Button variant="outline" size="sm" disabled class="text-xs border-zinc-700 text-zinc-500">
          Detect Faces
        </Button>
      </Tooltip.Trigger>
      <Tooltip.Content>
        <p>Coming soon</p>
      </Tooltip.Content>
    </Tooltip.Root>

    <Tooltip.Root>
      <Tooltip.Trigger>
        <Button variant="outline" size="sm" disabled class="text-xs border-zinc-700 text-zinc-500">
          Auto-Grade
        </Button>
      </Tooltip.Trigger>
      <Tooltip.Content>
        <p>Coming soon</p>
      </Tooltip.Content>
    </Tooltip.Root>
  </div>

  <!-- Right: Export -->
  <div class="flex items-center">
    <Tooltip.Root>
      <Tooltip.Trigger>
        <Button variant="outline" size="sm" disabled class="text-xs border-zinc-700 text-zinc-500">
          Export
        </Button>
      </Tooltip.Trigger>
      <Tooltip.Content>
        <p>Coming soon</p>
      </Tooltip.Content>
    </Tooltip.Root>
  </div>
</header>
