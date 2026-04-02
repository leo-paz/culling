<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import {
    currentProject,
    currentPhoto,
    navigateNext,
    navigatePrev,
    updatePhotoGrade,
    fullscreen,
    viewMode,
    activePerson,
    currentIndex,
    type Photo,
  } from '$lib/stores/project';
  import WelcomeScreen from '$lib/components/WelcomeScreen.svelte';
  import AppShell from '$lib/components/AppShell.svelte';
  import ShortcutOverlay from '$lib/components/ShortcutOverlay.svelte';

  let appShell: ReturnType<typeof AppShell> | undefined = $state();
  let shortcutsOpen = $state(false);

  async function setGrade(grade: Photo['grade']) {
    const project = $currentProject;
    const photo = $currentPhoto;
    if (!project || !photo) return;

    updatePhotoGrade(photo.path, grade);

    try {
      await invoke('update_grade', {
        projectId: project.id,
        photoPath: photo.path,
        grade,
      });
    } catch (e) {
      console.error('Failed to update grade:', e);
    }
  }

  function handleKeydown(e: KeyboardEvent) {
    // Ignore if user is typing in an input
    if (
      e.target instanceof HTMLInputElement ||
      e.target instanceof HTMLTextAreaElement
    ) {
      return;
    }

    if (!$currentProject) return;

    // Cmd+R / Ctrl+R → refresh current project from disk
    if (e.key === 'r' && (e.metaKey || e.ctrlKey)) {
      e.preventDefault();
      const project = $currentProject;
      if (project) {
        invoke<import('$lib/stores/project').Project>('get_project', { id: project.id })
          .then((refreshed) => {
            currentProject.set(refreshed);
            fullscreen.set(false);
          })
          .catch((e) => console.error('Failed to refresh project:', e));
      }
      return;
    }

    // Cmd+E / Ctrl+E → open export dialog
    if (e.key === 'e' && (e.metaKey || e.ctrlKey)) {
      e.preventDefault();
      appShell?.openExport();
      return;
    }

    switch (e.key) {
      case 'ArrowLeft':
        e.preventDefault();
        navigatePrev();
        break;
      case 'ArrowRight':
        e.preventDefault();
        navigateNext();
        break;
      case '1':
        setGrade('Bad');
        break;
      case '2':
        setGrade('Ok');
        break;
      case '3':
        setGrade('Good');
        break;
      case '0':
        setGrade('Ungraded');
        break;
      case '?':
        shortcutsOpen = !shortcutsOpen;
        break;
      case 'f':
      case 'F':
        if (!e.metaKey && !e.ctrlKey) {
          e.preventDefault();
          fullscreen.update((v) => !v);
        }
        break;
      case 't':
      case 'T':
        if (!e.metaKey && !e.ctrlKey) {
          viewMode.set('timeline');
          activePerson.set(null);
          currentIndex.set(0);
        }
        break;
      case 'p':
      case 'P':
        if (!e.metaKey && !e.ctrlKey) {
          const hasClusters = ($currentProject?.clusters?.length ?? 0) > 0;
          if (hasClusters) {
            viewMode.set('people');
          }
        }
        break;
      case 'Escape':
        if ($fullscreen) {
          fullscreen.set(false);
        }
        break;
    }
  }
</script>

<svelte:window onkeydown={handleKeydown} />

{#if $currentProject}
  <AppShell bind:this={appShell} />
{:else}
  <WelcomeScreen />
{/if}

<ShortcutOverlay bind:open={shortcutsOpen} />
