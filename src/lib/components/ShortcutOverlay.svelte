<script lang="ts">
  import * as Dialog from '$lib/components/ui/dialog';

  let { open = $bindable(false) }: { open?: boolean } = $props();

  const sections = [
    {
      title: 'Navigation',
      shortcuts: [
        { keys: ['\u2190', '\u2192'], description: 'Previous / Next photo' },
      ],
    },
    {
      title: 'Grading',
      shortcuts: [
        { keys: ['1'], description: 'Grade as Bad' },
        { keys: ['2'], description: 'Grade as OK' },
        { keys: ['3'], description: 'Grade as Good' },
        { keys: ['0'], description: 'Clear grade' },
      ],
    },
    {
      title: 'Views',
      shortcuts: [
        { keys: ['T'], description: 'Timeline view' },
        { keys: ['P'], description: 'People view' },
        { keys: ['F'], description: 'Toggle fullscreen' },
      ],
    },
    {
      title: 'Actions',
      shortcuts: [
        { keys: ['Cmd+E'], description: 'Export photos' },
        { keys: ['?'], description: 'Show shortcuts' },
        { keys: ['Esc'], description: 'Close dialog / Exit fullscreen' },
      ],
    },
  ];
</script>

<Dialog.Root bind:open>
  <Dialog.Content class="sm:max-w-md bg-surface-raised border border-zinc-800 text-zinc-100">
    <Dialog.Header>
      <Dialog.Title class="text-lg font-semibold text-zinc-100">Keyboard Shortcuts</Dialog.Title>
      <Dialog.Description class="text-zinc-400 text-sm">
        Quick reference for all available shortcuts.
      </Dialog.Description>
    </Dialog.Header>

    <div class="space-y-5 py-2">
      {#each sections as section}
        <div>
          <h3 class="text-[10px] font-medium text-zinc-400 uppercase tracking-wider mb-2">
            {section.title}
          </h3>
          <div class="space-y-1.5">
            {#each section.shortcuts as shortcut}
              <div class="flex items-center justify-between">
                <div class="flex items-center gap-1.5">
                  {#each shortcut.keys as key}
                    <kbd class="inline-flex items-center justify-center min-w-[28px] h-6 px-2 rounded bg-zinc-700 text-zinc-300 text-xs font-mono">
                      {key}
                    </kbd>
                  {/each}
                </div>
                <span class="text-sm text-zinc-400">{shortcut.description}</span>
              </div>
            {/each}
          </div>
        </div>
      {/each}
    </div>
  </Dialog.Content>
</Dialog.Root>
