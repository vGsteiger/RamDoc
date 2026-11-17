<script lang="ts">
  import type { Snippet } from 'svelte';

  type Width = 'prose' | 'form' | 'wide' | 'full';

  /*
   * The single page container. Every routed page renders inside one, so pages
   * share padding, max-width and left edge instead of inventing their own.
   *
   *   width="wide"  (default) lists, records, dashboards — max-w-page (1216px)
   *   width="form"  settings and forms — max-w-form (768px)
   *   width="prose" reading/writing surfaces — max-w-measure (72ch)
   *   width="full"  calendars and other canvases that use every pixel
   *
   * The column is centred, so pages with the same width share a left edge.
   * `fill` makes the shell take the full height of its scroll parent with the
   * header pinned and only the body scrolling (chat-like screens).
   */
  let {
    title = undefined,
    description = undefined,
    width = 'wide',
    fill = false,
    class: className = '',
    bodyClass = '',
    breadcrumb,
    actions,
    header,
    children,
  }: {
    /** Rendered as the page's only <h1>, in text-display. */
    title?: string;
    description?: string;
    width?: Width;
    fill?: boolean;
    class?: string;
    bodyClass?: string;
    /** Small trail above the title. */
    breadcrumb?: Snippet;
    /** Right-aligned header actions: at most one primary Button. */
    actions?: Snippet;
    /** Replaces the default title/description block entirely. */
    header?: Snippet;
    children?: Snippet;
  } = $props();

  const widths: Record<Width, string> = {
    prose: 'max-w-measure',
    form: 'max-w-form',
    wide: 'max-w-page',
    full: 'max-w-none',
  };
</script>

<div
  class="mx-auto w-full min-w-0 px-page-x {widths[width]} {fill
    ? 'flex h-full min-h-0 flex-col'
    : 'py-page-y'} {className}"
  data-page-shell={width}
>
  {#if header || title || actions || breadcrumb}
    <header class="{fill ? 'shrink-0 pt-page-y' : ''} mb-section min-w-0">
      {#if header}
        {@render header()}
      {:else}
        {#if breadcrumb}
          <div class="mb-1.5 flex min-w-0 items-center gap-1 text-caption text-fg-subtle">
            {@render breadcrumb()}
          </div>
        {/if}
        <div class="flex flex-wrap items-start justify-between gap-x-4 gap-y-3">
          <div class="min-w-0 flex-1 basis-64">
            {#if title}
              <h1 class="text-display text-fg [overflow-wrap:anywhere]">{title}</h1>
            {/if}
            {#if description}
              <p class="mt-1 text-body text-fg-muted">{description}</p>
            {/if}
          </div>
          {#if actions}
            <div class="flex shrink-0 flex-wrap items-center gap-2">{@render actions()}</div>
          {/if}
        </div>
      {/if}
    </header>
  {/if}

  <div class="min-w-0 {fill ? 'min-h-0 flex-1 overflow-y-auto pb-page-y' : ''} {bodyClass}">
    {@render children?.()}
  </div>
</div>
