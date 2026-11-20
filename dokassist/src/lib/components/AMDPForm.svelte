<script lang="ts">
  import { ChevronLeft, ChevronRight } from 'lucide-svelte';
  import { t } from '$lib/translations';
  import type { AMDPCategory } from '$lib/amdp';
  import { Button, Tabs, type TabItem } from '$lib/components/ui';
  import AMDPCategoryComponent from './AMDPCategory.svelte';

  /* AMDP findings by category: underline tabs (scroll inside their strip) plus Back/Next. */
  interface Props {
    categories: AMDPCategory[];
    onScoreChange: (code: string, score: 0 | 1 | 2 | 3) => void;
  }

  let { categories, onScoreChange }: Props = $props();

  let activeCategoryIndex = $state(0);
  const uid = $props.id();
  const panelId = `${uid}-amdp-panel`;

  let items = $derived<TabItem[]>(
    categories.map((category, index) => ({
      id: String(index),
      label: category.name,
      controls: panelId,
    }))
  );
  let tabValue = $derived(String(activeCategoryIndex));
</script>

<div class="space-y-3">
  <Tabs
    {items}
    value={tabValue}
    size="sm"
    idPrefix={uid}
    label={$t('sessions.amdpTitle')}
    onchange={(id) => (activeCategoryIndex = Number(id))}
  />

  {#if categories[activeCategoryIndex]}
    <div id={panelId} role="tabpanel" aria-labelledby="{uid}-tab-{activeCategoryIndex}">
      <AMDPCategoryComponent category={categories[activeCategoryIndex]} {onScoreChange} />
    </div>
  {/if}

  <div class="flex justify-between gap-2">
    <Button
      size="sm"
      disabled={activeCategoryIndex === 0}
      onclick={() => (activeCategoryIndex = Math.max(0, activeCategoryIndex - 1))}
    >
      <ChevronLeft size={14} aria-hidden="true" />
      {$t('common.back')}
    </Button>
    <Button
      size="sm"
      disabled={activeCategoryIndex === categories.length - 1}
      onclick={() =>
        (activeCategoryIndex = Math.min(categories.length - 1, activeCategoryIndex + 1))}
    >
      {$t('common.next')}
      <ChevronRight size={14} aria-hidden="true" />
    </Button>
  </div>
</div>
