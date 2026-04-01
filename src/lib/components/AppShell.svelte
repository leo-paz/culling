<script lang="ts">
  import Sidebar from './Sidebar.svelte';
  import Toolbar from './Toolbar.svelte';
  import PhotoViewer from './PhotoViewer.svelte';
  import Filmstrip from './Filmstrip.svelte';
  import StatusBar from './StatusBar.svelte';
  import ThumbnailProgress from './ThumbnailProgress.svelte';
</script>

<div class="h-screen w-screen grid overflow-hidden" style="grid-template-columns: 260px 1fr; grid-template-rows: 48px 1fr 32px;">
  <!-- Sidebar spans all rows -->
  <div class="row-span-3 h-full overflow-hidden">
    <Sidebar />
  </div>

  <!-- Toolbar -->
  <div>
    <Toolbar />
  </div>

  <!-- Main content: Photo viewer + Filmstrip -->
  <div class="flex flex-col overflow-hidden relative">
    <div class="flex-1 min-h-0">
      <PhotoViewer />
    </div>
    <Filmstrip />
    <ThumbnailProgress />
  </div>

  <!-- Status bar -->
  <div>
    <StatusBar />
  </div>
</div>
