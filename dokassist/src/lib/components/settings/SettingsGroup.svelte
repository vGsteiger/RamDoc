<script lang="ts">
  import type { Snippet } from 'svelte';
  import { SectionHeader } from '$lib/components/ui';

  /*
   * A titled group of SettingsRows on one raised surface, separated by
   * hairlines (the rows draw their own). Every settings pane is a stack of
   * these, so panes share rhythm and edges.
   */
  let {
    title = undefined,
    description = undefined,
    class: className = '',
    actions,
    children,
  }: {
    title?: string;
    description?: string;
    class?: string;
    /** Right-aligned header actions, e.g. "Import verified model". */
    actions?: Snippet;
    children?: Snippet;
  } = $props();
</script>

<section class="mb-section min-w-0 {className}">
  {#if title}
    <SectionHeader {title} {description} {actions} />
  {/if}
  <div class="min-w-0 card-surface rounded-panel px-4">
    {@render children?.()}
  </div>
</section>
