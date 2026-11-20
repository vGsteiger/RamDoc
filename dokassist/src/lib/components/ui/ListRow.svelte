<script lang="ts">
  import type { Snippet } from 'svelte';

  /*
   * A dense (36px) list row: leading icon, title and secondary text, meta, and
   * trailing actions. Used for patients, sessions, documents and chats.
   *
   * With `href` or `onclick` the whole row is one link/button (a stretched
   * hit area), while `trailing` actions sit above it and stay separately
   * focusable — never nest a button inside the row's link. Keep trailing
   * actions visible (or in a Menu), not hover-only.
   */
  let {
    title = undefined,
    secondary = undefined,
    meta = undefined,
    href = undefined,
    onclick = undefined,
    selected = false,
    disabled = false,
    class: className = '',
    leading,
    trailing,
    children,
    metaSnippet,
    ...rest
  }: {
    title?: string;
    secondary?: string;
    /** Short right-aligned text such as a date. */
    meta?: string;
    href?: string;
    onclick?: (event: MouseEvent) => void;
    /** The current item (e.g. the open chat). */
    selected?: boolean;
    disabled?: boolean;
    class?: string;
    leading?: Snippet;
    trailing?: Snippet;
    /** Replaces title/secondary with custom content. */
    children?: Snippet;
    /** Replaces `meta` with custom content (e.g. a Badge). */
    metaSnippet?: Snippet;
    [key: string]: unknown;
  } = $props();

  let interactive = $derived(Boolean(href || onclick) && !disabled);
</script>

{#snippet body()}
  {#if leading}
    <span class="flex shrink-0 items-center text-fg-subtle">{@render leading()}</span>
  {/if}
  <span class="flex min-w-0 flex-1 items-baseline gap-2">
    {#if children}
      {@render children()}
    {:else}
      <span class="truncate text-body text-fg">{title}</span>
      {#if secondary}
        <span class="min-w-0 truncate text-label text-fg-muted">{secondary}</span>
      {/if}
    {/if}
  </span>
  {#if metaSnippet}
    <span class="flex shrink-0 items-center">{@render metaSnippet()}</span>
  {:else if meta}
    <span class="shrink-0 whitespace-nowrap text-caption text-fg-subtle" data-numeric>{meta}</span>
  {/if}
{/snippet}

<div
  class="group relative flex min-h-row min-w-0 items-center gap-3 rounded-control px-3 transition-colors duration-150 ease-standard {selected
    ? 'bg-surface-selected'
    : interactive
      ? 'hover:bg-surface-hover'
      : ''} {disabled ? 'opacity-50' : ''} {className}"
  {...rest}
>
  {#if href && !disabled}
    <a
      {href}
      class="flex min-w-0 flex-1 items-center gap-3 self-stretch focus-visible:outline-none after:absolute after:inset-0 after:rounded-control focus-visible:after:outline-2 focus-visible:after:outline-focus"
      aria-current={selected ? 'page' : undefined}
    >
      {@render body()}
    </a>
  {:else if onclick && !disabled}
    <button
      type="button"
      {onclick}
      class="flex min-w-0 flex-1 items-center gap-3 self-stretch text-left focus-visible:outline-none after:absolute after:inset-0 after:rounded-control focus-visible:after:outline-2 focus-visible:after:outline-focus"
      aria-current={selected ? 'true' : undefined}
    >
      {@render body()}
    </button>
  {:else}
    <div class="flex min-w-0 flex-1 items-center gap-3">{@render body()}</div>
  {/if}
  {#if trailing}
    <div class="relative z-10 flex shrink-0 items-center gap-1">{@render trailing()}</div>
  {/if}
</div>
