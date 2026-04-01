<script lang="ts">
  import { Separator } from '$lib/components/ui/separator';
  import { currentProject, gradeCounts } from '$lib/stores/project';
</script>

<aside class="flex flex-col h-full bg-surface-raised border-r border-zinc-800 overflow-hidden">
  <!-- Project Name -->
  <div class="px-4 py-3">
    <h2 class="text-sm font-semibold text-zinc-100 truncate font-display">
      {$currentProject?.name ?? 'Untitled'}
    </h2>
    <p class="text-xs text-zinc-500 truncate mt-0.5">
      {$currentProject?.source_dir ?? ''}
    </p>
  </div>

  <Separator class="bg-zinc-800" />

  <!-- People Section -->
  <div class="px-4 py-3">
    <h3 class="text-[10px] font-medium text-zinc-400 uppercase tracking-wider mb-2">
      People
    </h3>
    <p class="text-xs text-zinc-500 italic">
      Detect faces to see people
    </p>
  </div>

  <Separator class="bg-zinc-800" />

  <!-- Grade Counts -->
  <div class="px-4 py-3">
    <h3 class="text-[10px] font-medium text-zinc-400 uppercase tracking-wider mb-3">
      Grades
    </h3>
    <div class="space-y-2">
      <div class="flex items-center justify-between text-xs">
        <div class="flex items-center gap-2">
          <span class="w-2 h-2 rounded-full bg-grade-bad"></span>
          <span class="text-zinc-300">Bad</span>
        </div>
        <span class="text-zinc-500 font-mono">{$gradeCounts.bad}</span>
      </div>
      <div class="flex items-center justify-between text-xs">
        <div class="flex items-center gap-2">
          <span class="w-2 h-2 rounded-full bg-grade-ok"></span>
          <span class="text-zinc-300">OK</span>
        </div>
        <span class="text-zinc-500 font-mono">{$gradeCounts.ok}</span>
      </div>
      <div class="flex items-center justify-between text-xs">
        <div class="flex items-center gap-2">
          <span class="w-2 h-2 rounded-full bg-grade-good"></span>
          <span class="text-zinc-300">Good</span>
        </div>
        <span class="text-zinc-500 font-mono">{$gradeCounts.good}</span>
      </div>
      <div class="flex items-center justify-between text-xs pt-1 border-t border-zinc-800">
        <div class="flex items-center gap-2">
          <span class="w-2 h-2 rounded-full bg-zinc-600"></span>
          <span class="text-zinc-400">Ungraded</span>
        </div>
        <span class="text-zinc-500 font-mono">{$gradeCounts.ungraded}</span>
      </div>
    </div>
  </div>

  <!-- Spacer to push content up -->
  <div class="flex-1"></div>
</aside>
