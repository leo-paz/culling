<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import {
    currentProject,
    currentPhoto,
    navigateNext,
    navigatePrev,
    updatePhotoGrade,
    type Photo,
  } from '$lib/stores/project';
  import WelcomeScreen from '$lib/components/WelcomeScreen.svelte';
  import AppShell from '$lib/components/AppShell.svelte';

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
    }
  }
</script>

<svelte:window onkeydown={handleKeydown} />

{#if $currentProject}
  <AppShell />
{:else}
  <WelcomeScreen />
{/if}
