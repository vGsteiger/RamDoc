<script lang="ts">
  import type { Snippet } from 'svelte';

  /*
   * The action bar under a document editor. It sticks to the bottom of the
   * scrolling page, so Save / Send stay in reach of a long draft. `start`
   * holds secondary context on the left (a ⋯ menu, a hint, a Back link); the
   * default slot holds the actions on the right, primary last. At most one
   * primary per bar.
   */
  let {
    start,
    children,
    class: className = '',
  }: {
    start?: Snippet;
    children?: Snippet;
    class?: string;
  } = $props();
</script>

<div
  class="sticky bottom-0 z-10 mt-6 flex flex-wrap items-center gap-2 border-t border-line-subtle bg-surface py-3 {className}"
  data-action-bar
>
  {#if start}
    <div class="mr-auto flex min-w-0 items-center gap-2">{@render start()}</div>
  {/if}
  <div class="ml-auto flex flex-wrap items-center justify-end gap-2">
    {@render children?.()}
  </div>
</div>
