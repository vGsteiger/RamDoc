<script lang="ts">
  import type { AMDPCategory } from '$lib/amdp';
  import AMDPCategoryComponent from './AMDPCategory.svelte';

  interface Props {
    categories: AMDPCategory[];
    onScoreChange: (code: string, score: 0 | 1 | 2 | 3) => void;
  }

  let { categories, onScoreChange }: Props = $props();

  let activeCategoryIndex = $state(0);
</script>

<div class="space-y-4">
  <!-- Category tabs -->
  <div class="flex gap-2 flex-wrap">
    {#each categories as category, index}
      <button
        type="button"
        class="px-4 py-2 rounded-lg text-sm font-medium transition-colors"
        class:bg-blue-600={activeCategoryIndex === index}
        class:text-white={activeCategoryIndex === index}
        class:bg-gray-700={activeCategoryIndex !== index}
        class:text-gray-300={activeCategoryIndex !== index}
        class:hover:bg-gray-600={activeCategoryIndex !== index}
        onclick={() => (activeCategoryIndex = index)}
      >
        {category.name}
      </button>
    {/each}
  </div>

  <!-- Active category content -->
  {#if categories[activeCategoryIndex]}
    <AMDPCategoryComponent category={categories[activeCategoryIndex]} {onScoreChange} />
  {/if}

  <!-- Navigation buttons -->
  <div class="flex justify-between pt-4">
    <button
      type="button"
      class="px-4 py-2 bg-gray-700 text-gray-300 rounded-lg hover:bg-gray-600 disabled:opacity-50 disabled:cursor-not-allowed"
      disabled={activeCategoryIndex === 0}
      onclick={() => (activeCategoryIndex = Math.max(0, activeCategoryIndex - 1))}
    >
      ← Zurück
    </button>
    <button
      type="button"
      class="px-4 py-2 bg-gray-700 text-gray-300 rounded-lg hover:bg-gray-600 disabled:opacity-50 disabled:cursor-not-allowed"
      disabled={activeCategoryIndex === categories.length - 1}
      onclick={() =>
        (activeCategoryIndex = Math.min(categories.length - 1, activeCategoryIndex + 1))}
    >
      Weiter →
    </button>
  </div>
</div>
