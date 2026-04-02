<script lang="ts">
  import { invoke, Channel } from '@tauri-apps/api/core';
  import { open } from '@tauri-apps/plugin-dialog';
  import { Button } from '$lib/components/ui/button';
  import * as Dialog from '$lib/components/ui/dialog';
  import * as Progress from '$lib/components/ui/progress';
  import { Separator } from '$lib/components/ui/separator';
  import { currentProject, type Project } from '$lib/stores/project';

  let open_dialog = $state(false);
  let gradeFilter = $state<'All' | 'OkAndGood' | 'GoodOnly'>('OkAndGood');
  let organization = $state<'Flat' | 'ByPerson'>('Flat');
  let trashBad = $state(false);
  let exporting = $state(false);
  let exportProgress = $state<{ current: number; total: number } | null>(null);
  let exportResult = $state<{ success: boolean; count: number; error?: string } | null>(null);

  export function show() {
    exportResult = null;
    exportProgress = null;
    exporting = false;
    open_dialog = true;
  }

  let hasClusters = $derived(($currentProject?.clusters?.length ?? 0) > 0);

  // Compute counts based on selected grade filter
  let exportCount = $derived.by(() => {
    const project = $currentProject;
    if (!project) return 0;
    switch (gradeFilter) {
      case 'All':
        return project.photos.length;
      case 'OkAndGood':
        return project.photos.filter(
          (p) => p.grade !== 'Bad'
        ).length;
      case 'GoodOnly':
        return project.photos.filter(
          (p) => p.grade === 'Good'
        ).length;
    }
  });

  let badCount = $derived(
    $currentProject?.photos.filter((p) => p.grade === 'Bad').length ?? 0
  );

  async function handleExport() {
    const project = $currentProject;
    if (!project) return;

    // Open native folder picker
    const outputDir = await open({
      directory: true,
      title: 'Choose export folder',
    });
    if (!outputDir) return;

    exporting = true;
    exportResult = null;

    const onProgress = new Channel<{ current: number; total: number; message: string }>();
    onProgress.onmessage = (p) => {
      exportProgress = { current: p.current, total: p.total };
    };

    try {
      const count = await invoke<number>('export_photos', {
        projectId: project.id,
        outputDir,
        gradeFilter: gradeFilter,
        organization,
        trashBad,
        onProgress,
      });
      exportResult = { success: true, count };
    } catch (e) {
      console.error('Export failed:', e);
      exportResult = { success: false, count: 0, error: String(e) };
    } finally {
      exporting = false;
      exportProgress = null;
    }
  }
</script>

<Dialog.Root bind:open={open_dialog}>
  <Dialog.Content class="sm:max-w-md bg-surface-raised border border-zinc-800 text-zinc-100">
    <Dialog.Header>
      <Dialog.Title class="text-lg font-semibold text-zinc-100">Export Photos</Dialog.Title>
      <Dialog.Description class="text-zinc-400">
        Choose which photos to export and how to organize them.
      </Dialog.Description>
    </Dialog.Header>

    {#if exportResult}
      <!-- Result state -->
      <div class="py-4 text-center">
        {#if exportResult.success}
          <div class="text-grade-good text-lg font-medium mb-2">Export Complete</div>
          <p class="text-zinc-400 text-sm">
            {exportResult.count} photo{exportResult.count === 1 ? '' : 's'} exported successfully.
          </p>
        {:else}
          <div class="text-grade-bad text-lg font-medium mb-2">Export Failed</div>
          <p class="text-zinc-400 text-sm">{exportResult.error}</p>
        {/if}
      </div>
      <Dialog.Footer>
        <Button
          variant="outline"
          class="border-zinc-700 text-zinc-300"
          onclick={() => { open_dialog = false; }}
        >
          Close
        </Button>
      </Dialog.Footer>
    {:else if exporting}
      <!-- Progress state -->
      <div class="py-6 space-y-4">
        <p class="text-sm text-zinc-400 text-center">
          Exporting photos...
          {#if exportProgress}
            {exportProgress.current} / {exportProgress.total}
          {/if}
        </p>
        <Progress.Root
          value={exportProgress?.current ?? 0}
          max={exportProgress?.total ?? 100}
          class="h-2 bg-zinc-800"
        />
      </div>
    {:else}
      <!-- Configuration state -->
      <div class="space-y-5">
        <!-- Grade filter -->
        <fieldset class="space-y-2">
          <legend class="text-sm font-medium text-zinc-300">Photos to export</legend>
          <div class="space-y-1.5">
            <label class="flex items-center gap-2.5 cursor-pointer group">
              <input
                type="radio"
                name="gradeFilter"
                value="OkAndGood"
                bind:group={gradeFilter}
                class="w-4 h-4 accent-accent bg-zinc-800 border-zinc-600"
              />
              <span class="text-sm text-zinc-300 group-hover:text-zinc-100">
                Ok + Good (exclude Bad)
              </span>
            </label>
            <label class="flex items-center gap-2.5 cursor-pointer group">
              <input
                type="radio"
                name="gradeFilter"
                value="GoodOnly"
                bind:group={gradeFilter}
                class="w-4 h-4 accent-accent bg-zinc-800 border-zinc-600"
              />
              <span class="text-sm text-zinc-300 group-hover:text-zinc-100">
                Good only
              </span>
            </label>
            <label class="flex items-center gap-2.5 cursor-pointer group">
              <input
                type="radio"
                name="gradeFilter"
                value="All"
                bind:group={gradeFilter}
                class="w-4 h-4 accent-accent bg-zinc-800 border-zinc-600"
              />
              <span class="text-sm text-zinc-300 group-hover:text-zinc-100">
                All photos
              </span>
            </label>
          </div>
        </fieldset>

        <Separator class="bg-zinc-800" />

        <!-- Organization -->
        <fieldset class="space-y-2">
          <legend class="text-sm font-medium text-zinc-300">Folder structure</legend>
          <div class="space-y-1.5">
            <label class="flex items-center gap-2.5 cursor-pointer group">
              <input
                type="radio"
                name="organization"
                value="Flat"
                bind:group={organization}
                class="w-4 h-4 accent-accent bg-zinc-800 border-zinc-600"
              />
              <span class="text-sm text-zinc-300 group-hover:text-zinc-100">
                Flat (all in one folder)
              </span>
            </label>
            <label class="flex items-center gap-2.5 cursor-pointer group">
              <input
                type="radio"
                name="organization"
                value="ByPerson"
                bind:group={organization}
                disabled={!hasClusters}
                class="w-4 h-4 accent-accent bg-zinc-800 border-zinc-600 disabled:opacity-40"
              />
              <span class="text-sm text-zinc-300 group-hover:text-zinc-100" class:opacity-40={!hasClusters}>
                By person (subfolder per person)
                {#if !hasClusters}
                  <span class="text-xs text-zinc-500 ml-1">- run face detection first</span>
                {/if}
              </span>
            </label>
          </div>
        </fieldset>

        <Separator class="bg-zinc-800" />

        <!-- Trash bad option -->
        <label class="flex items-center gap-2.5 cursor-pointer group">
          <input
            type="checkbox"
            bind:checked={trashBad}
            class="w-4 h-4 accent-grade-bad bg-zinc-800 border-zinc-600 rounded"
          />
          <span class="text-sm text-zinc-300 group-hover:text-zinc-100">
            Move Bad photos to Trash
            {#if badCount > 0}
              <span class="text-zinc-500">({badCount})</span>
            {/if}
          </span>
        </label>
      </div>

      <Dialog.Footer>
        <Button
          variant="outline"
          class="border-zinc-700 text-zinc-300"
          onclick={() => { open_dialog = false; }}
        >
          Cancel
        </Button>
        <Button
          class="bg-accent hover:bg-accent-hover text-zinc-900 font-medium"
          disabled={exportCount === 0}
          onclick={handleExport}
        >
          Export {exportCount} photo{exportCount === 1 ? '' : 's'}
        </Button>
      </Dialog.Footer>
    {/if}
  </Dialog.Content>
</Dialog.Root>
