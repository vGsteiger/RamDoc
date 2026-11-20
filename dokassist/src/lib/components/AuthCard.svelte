<script lang="ts">
  import type { Snippet } from 'svelte';
  import { BrandMark } from '$lib/components/ui';

  /*
   * The frame shared by the layoutless auth screens (setup, unlock, recover,
   * reset): a centred card on the page surface with the brand mark on top.
   * Purely visual — every screen keeps its own logic and confirmations.
   *
   *   width  sm 28rem (unlock, reset) · md 40rem · lg 56rem (24-word grids)
   *   tone   danger draws the card's hairline in the danger tone (reset)
   */
  let {
    title = undefined,
    description = undefined,
    width = 'sm',
    tone = 'default',
    headingLevel = 1,
    children,
    footer,
  }: {
    title?: string;
    description?: string;
    width?: 'sm' | 'md' | 'lg';
    tone?: 'default' | 'danger';
    /** Use 2 when the page already rendered its h1 (e.g. a second step). */
    headingLevel?: 1 | 2;
    children?: Snippet;
    /** Secondary links under a hairline (recovery, factory reset). */
    footer?: Snippet;
  } = $props();

  const widths = { sm: 'max-w-md', md: 'max-w-2xl', lg: 'max-w-4xl' };
</script>

<div class="flex min-h-screen items-center justify-center bg-surface px-4 py-10 text-fg sm:px-8">
  <main
    class="w-full {widths[width]} rounded-card border bg-surface-raised px-6 py-8 sm:px-10 {tone ===
    'danger'
      ? 'border-danger-line'
      : 'border-line'}"
  >
    <div class="flex flex-col items-center text-center">
      <BrandMark size={32} />
      {#if title}
        <svelte:element this={`h${headingLevel}`} class="mt-5 text-display text-fg">
          {title}
        </svelte:element>
      {/if}
      {#if description}
        <p class="mt-2 max-w-prose text-body text-fg-muted">{description}</p>
      {/if}
    </div>

    <div class="mt-8">
      {@render children?.()}
    </div>

    {#if footer}
      <div class="mt-8 border-t border-line-subtle pt-5 text-center">
        {@render footer()}
      </div>
    {/if}
  </main>
</div>
