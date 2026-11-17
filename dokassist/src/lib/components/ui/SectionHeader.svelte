<script lang="ts">
  import type { Snippet } from 'svelte';

  /*
   * Heading for a section inside a page or tab: text-heading title, optional
   * description, right-aligned actions. Use `level` to keep the outline
   * correct (the page title is the h1, so sections default to h2).
   */
  let {
    title,
    description = undefined,
    level = 2,
    id = undefined,
    class: className = '',
    actions,
  }: {
    title: string;
    description?: string;
    level?: 2 | 3 | 4;
    /** Id for the heading, e.g. to label a <section aria-labelledby>. */
    id?: string;
    class?: string;
    actions?: Snippet;
  } = $props();
</script>

<div class="mb-3 flex flex-wrap items-end justify-between gap-x-4 gap-y-2 {className}">
  <div class="min-w-0 flex-1 basis-48">
    <svelte:element this={`h${level}`} {id} class="text-heading text-fg">{title}</svelte:element>
    {#if description}
      <p class="mt-0.5 text-label text-fg-muted">{description}</p>
    {/if}
  </div>
  {#if actions}
    <div class="flex shrink-0 flex-wrap items-center gap-2">{@render actions()}</div>
  {/if}
</div>
