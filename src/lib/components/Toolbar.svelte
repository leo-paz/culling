<script lang="ts">
  import { get } from 'svelte/store';
  import { invoke, Channel } from '@tauri-apps/api/core';
  import { Button } from '$lib/components/ui/button';
  import * as Tabs from '$lib/components/ui/tabs';
  import * as Tooltip from '$lib/components/ui/tooltip';
  import { viewMode, currentProject, activePerson, currentIndex, type Project } from '$lib/stores/project';

  let { onexport = () => {} }: { onexport?: () => void } = $props();

  let detecting = $state(false);
  let detectProgress = $state<{ current: number; total: number; message: string } | null>(null);

  let grading = $state(false);
  let gradeProgress = $state<{ current: number; total: number; message: string } | null>(null);

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

  async function startFaceDetection() {
    const project = get(currentProject);
    if (!project) return;

    detecting = true;
    detectProgress = null;
    const onProgress = new Channel<{ current: number; total: number; message: string }>();
    onProgress.onmessage = (p) => {
      detectProgress = p;
    };

    try {
      const updated = await invoke<Project>('start_face_detection', {
        projectId: project.id,
        onProgress,
      });
      currentProject.set(updated);
    } catch (e) {
      console.error('Face detection failed:', e);
    } finally {
      detecting = false;
      detectProgress = null;
    }
  }

  async function startAutoGrade() {
    const project = get(currentProject);
    if (!project) return;

    grading = true;
    gradeProgress = null;
    const onProgress = new Channel<{ current: number; total: number; message: string }>();
    onProgress.onmessage = (p) => {
      gradeProgress = p;
    };

    try {
      const updated = await invoke<Project>('start_auto_grade', {
        projectId: project.id,
        onProgress,
      });
      currentProject.set(updated);
    } catch (e) {
      console.error('Auto-grade failed:', e);
    } finally {
      grading = false;
      gradeProgress = null;
    }
  }
</script>

<header class="flex items-center justify-between bg-surface-raised border-b border-zinc-800 px-3 h-12">
  <!-- Left: View Mode Tabs -->
  <div class="flex items-center">
    <Tabs.Root bind:value={$viewMode}>
      <Tabs.List class="bg-surface-overlay h-7">
        <Tabs.Trigger value="timeline" class="text-xs px-3 h-6">Timeline</Tabs.Trigger>
        <Tabs.Trigger value="people" class="text-xs px-3 h-6" disabled={!hasClusters}>People</Tabs.Trigger>
      </Tabs.List>
    </Tabs.Root>
  </div>

  <!-- Center: Action Buttons -->
  <div class="flex items-center gap-2">
    <Tooltip.Root>
      <Tooltip.Trigger>
        {#if detecting}
          <Button variant="outline" size="sm" disabled class="text-xs border-zinc-700 text-zinc-400">
            {#if detectProgress}
              Detecting... {detectProgress.current}/{detectProgress.total}
            {:else}
              Detecting...
            {/if}
          </Button>
        {:else}
          <Button
            variant="outline"
            size="sm"
            class="text-xs border-zinc-700 text-zinc-300 hover:text-zinc-100"
            disabled={!$currentProject || hasClusters}
            onclick={startFaceDetection}
          >
            {hasClusters ? 'Faces Detected' : 'Detect Faces'}
          </Button>
        {/if}
      </Tooltip.Trigger>
      <Tooltip.Content>
        <p>{hasClusters ? 'Face detection already completed' : 'Detect and cluster faces in all photos'}</p>
      </Tooltip.Content>
    </Tooltip.Root>

    <Tooltip.Root>
      <Tooltip.Trigger>
        {#if grading}
          <Button variant="outline" size="sm" disabled class="text-xs border-zinc-700 text-zinc-400">
            {#if gradeProgress}
              Grading... {gradeProgress.current}/{gradeProgress.total}
            {:else}
              Grading...
            {/if}
          </Button>
        {:else}
          <Button
            variant="outline"
            size="sm"
            class="text-xs border-zinc-700 text-zinc-300 hover:text-zinc-100"
            disabled={!$currentProject}
            onclick={startAutoGrade}
          >
            Auto-Grade
          </Button>
        {/if}
      </Tooltip.Trigger>
      <Tooltip.Content>
        <p>Automatically grade photos based on quality heuristics</p>
      </Tooltip.Content>
    </Tooltip.Root>
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
