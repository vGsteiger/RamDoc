<script lang="ts">
  import type { AMDPCategory } from '$lib/amdp';
  import AMDPItem from './AMDPItem.svelte';

  interface Props {
    category: AMDPCategory;
    onScoreChange: (code: string, score: 0 | 1 | 2 | 3) => void;
  }

  let { category, onScoreChange }: Props = $props();
</script>

<div class="bg-gray-800 rounded-lg p-4 border border-gray-700">
  <h3 class="text-lg font-semibold text-gray-100 mb-3">{category.name}</h3>
  <div class="space-y-1">
    {#each category.items as item (item.code)}
      <AMDPItem {item} {onScoreChange} />
    {/each}
  </div>
</div>
