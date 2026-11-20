<script lang="ts">
  import type { AMDPCategory } from '$lib/amdp';
  import AMDPItem from './AMDPItem.svelte';

  interface Props {
    category: AMDPCategory;
    onScoreChange: (code: string, score: 0 | 1 | 2 | 3) => void;
  }

  let { category, onScoreChange }: Props = $props();
</script>

<div>
  <h3 class="mb-2 text-heading text-fg">{category.name}</h3>
  <div class="space-y-0.5">
    {#each category.items as item (item.code)}
      <AMDPItem {item} {onScoreChange} />
    {/each}
  </div>
</div>
