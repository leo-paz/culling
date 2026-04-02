<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { open } from '@tauri-apps/plugin-dialog';
  import { Button } from '$lib/components/ui/button';
  import { Separator } from '$lib/components/ui/separator';
  import { currentProject, currentIndex, enrichmentStatus, startEnrichmentPolling, type Project } from '$lib/stores/project';

  let recentProjects = $state<Project[]>([]);
  let isImporting = $state(false);
  let error = $state<string | null>(null);

  function triggerEnrichment(projectId: string) {
    invoke('start_enrichment', { projectId }).catch((e) =>
      console.error('Enrichment start failed:', e)
    );
    enrichmentStatus.set({ stage: 'grading', current: 0, total: 0 });
    startEnrichmentPolling(projectId);
  }

  async function loadRecentProjects() {
    try {
      recentProjects = await invoke<Project[]>('list_projects');
    } catch {
      // No projects yet, that's fine
      recentProjects = [];
    }
  }

  async function openProject(project: Project) {
    try {
      currentIndex.set(0);
      const loaded = await invoke<Project>('open_project', { id: project.id });
      currentProject.set(loaded);
      // Trigger background enrichment and poll for updates
      triggerEnrichment(loaded.id);
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    }
  }

  async function handleImport() {
    try {
      error = null;
      const selected = await open({ directory: true });
      if (!selected) return;

      isImporting = true;
      const project = await invoke<Project>('import_folder', { path: selected });
      currentIndex.set(0);
      currentProject.set(project);
      isImporting = false;

      // Trigger background enrichment and poll for updates
      triggerEnrichment(project.id);
    } catch (e) {
      isImporting = false;
      error = e instanceof Error ? e.message : String(e);
    }
  }

  // Load recent projects on mount
  loadRecentProjects();
</script>

<div class="flex h-screen items-center justify-center bg-surface">
  <div class="text-center space-y-6 max-w-md px-6">
    <div class="space-y-2">
      <h1 class="text-5xl font-bold font-display text-accent tracking-tight">
        Culling
      </h1>
      <p class="text-zinc-400 font-body text-base">
        Fast photo culling with face detection
      </p>
    </div>

    <Button
      class="bg-accent hover:bg-accent-hover text-surface font-semibold px-6 h-10 text-sm transition-colors duration-150"
      disabled={isImporting}
      onclick={handleImport}
    >
      {#if isImporting}
        <svg class="animate-spin -ml-1 mr-2 h-4 w-4" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24">
          <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
          <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
        </svg>
        Importing...
      {:else}
        Import Folder
      {/if}
    </Button>

    {#if error}
      <p class="text-red-400 text-sm">{error}</p>
    {/if}

    {#if recentProjects.length > 0}
      <div class="pt-2">
        <Separator class="bg-zinc-800" />
        <p class="text-zinc-500 text-xs uppercase tracking-wider mt-4 mb-3 font-medium">
          Recent Projects
        </p>
        <div class="space-y-1">
          {#each recentProjects as project (project.id)}
            <button
              class="w-full text-left px-3 py-2 rounded-md text-sm text-zinc-300 hover:bg-surface-raised hover:text-zinc-100 transition-colors duration-150 cursor-pointer"
              onclick={() => openProject(project)}
            >
              <span class="font-medium">{project.name}</span>
              <span class="text-zinc-500 text-xs block truncate">{project.source_dir}</span>
            </button>
          {/each}
        </div>
      </div>
    {/if}
  </div>
</div>
