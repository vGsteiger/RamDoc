<script lang="ts">
  import type { AMDPItem } from '$lib/amdp';

  interface Props {
    item: AMDPItem;
    onScoreChange: (code: string, score: 0 | 1 | 2 | 3) => void;
  }

  let { item, onScoreChange }: Props = $props();

  const scores = [
    { value: 0, label: '0', title: 'Nicht vorhanden' },
    { value: 1, label: '1', title: 'Leicht' },
    { value: 2, label: '2', title: 'Mittel' },
    { value: 3, label: '3', title: 'Schwer' },
  ] as const;

  function handleScoreClick(score: 0 | 1 | 2 | 3) {
    onScoreChange(item.code, score);
  }
</script>

<div class="flex items-center justify-between py-2 px-3 hover:bg-gray-700/30 rounded">
  <div class="flex-1">
    <span class="text-sm text-gray-300">
      <span class="text-gray-500 font-mono text-xs mr-2">{item.code}</span>
      {item.label}
    </span>
  </div>
  <div class="flex gap-1 ml-4">
    {#each scores as { value, label, title }}
      <button
        type="button"
        class="w-10 h-10 rounded transition-colors font-medium text-sm"
        class:bg-gray-600={item.score !== value}
        class:text-gray-300={item.score !== value}
        class:bg-blue-600={item.score === value}
        class:text-white={item.score === value}
        class:ring-2={item.score === value}
        class:ring-blue-400={item.score === value}
        onclick={() => handleScoreClick(value)}
        {title}
      >
        {label}
      </button>
    {/each}
  </div>
</div>
