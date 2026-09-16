<script lang="ts">
  import { t } from '$lib/translations';
  import Card from './Card.svelte';
  import Skeleton from './Skeleton.svelte';

  let {
    count = 4,
    variant = 'row',
    class: className = '',
  }: {
    count?: number;
    variant?: 'row' | 'file' | 'tile' | 'dashboard';
    class?: string;
  } = $props();

  const items = $derived(Array.from({ length: count }, (_, i) => i));
</script>

<div class={className} role="status" aria-label={$t('common.loading')}>
  {#if variant === 'dashboard'}
    <div class="grid grid-cols-1 gap-4 lg:grid-cols-3">
      {#each items.slice(0, 3) as i (i)}
        <Card>
          <Skeleton class="mb-4 h-4 w-40" />
          <div class="space-y-2">
            {#each [0, 1, 2] as row (row)}
              <div class="rounded-control p-2">
                <Skeleton class="h-3.5 w-3/4" />
                <Skeleton class="mt-2 h-3 w-1/2" />
              </div>
            {/each}
          </div>
        </Card>
      {/each}
    </div>
  {:else if variant === 'tile'}
    <div class="grid grid-cols-1 gap-4 lg:grid-cols-2 xl:grid-cols-3">
      {#each items as i (i)}
        <div class="rounded-card border border-line bg-surface-raised p-4">
          <div class="flex items-start gap-2">
            <Skeleton class="h-6 w-6 shrink-0" />
            <div class="min-w-0 flex-1 space-y-2">
              <Skeleton class="h-4 w-3/4" />
              <Skeleton class="h-3 w-1/2" />
            </div>
          </div>
        </div>
      {/each}
    </div>
  {:else if variant === 'file'}
    <div class="space-y-2">
      {#each items as i (i)}
        <div class="rounded-card border border-line bg-surface-raised p-3">
          <div class="flex items-start gap-3">
            <Skeleton class="h-8 w-8 shrink-0" />
            <div class="min-w-0 flex-1 space-y-2">
              <Skeleton class="h-4 w-2/3" />
              <Skeleton class="h-3 w-1/2" />
            </div>
          </div>
          <div class="mt-3 flex gap-2">
            <Skeleton class="h-7 w-16" />
            <Skeleton class="h-7 w-20" />
          </div>
        </div>
      {/each}
    </div>
  {:else}
    <div class="grid gap-2">
      {#each items as i (i)}
        <Card padding="sm">
          <div class="flex items-start justify-between gap-4">
            <div class="min-w-0 flex-1 space-y-2">
              <Skeleton class="h-4 w-48 max-w-full" />
              <Skeleton class="h-3 w-32 max-w-full" />
            </div>
            <Skeleton class="h-4 w-20 shrink-0" />
          </div>
        </Card>
      {/each}
    </div>
  {/if}
</div>
