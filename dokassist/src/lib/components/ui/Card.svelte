<script lang="ts">
  import type { Snippet } from 'svelte';

  let {
    padding = 'md',
    interactive = false,
    href = undefined,
    onclick = undefined,
    class: className = '',
    children,
    ...rest
  }: {
    padding?: 'none' | 'sm' | 'md';
    interactive?: boolean;
    href?: string;
    onclick?: (event: MouseEvent) => void;
    class?: string;
    children?: Snippet;
    [key: string]: unknown;
  } = $props();

  const paddings = { none: '', sm: 'p-3', md: 'p-4' };

  // A card that navigates or acts is interactive whether or not the caller
  // says so, so the hover affordance is never accidentally omitted.
  let isInteractive = $derived(interactive || Boolean(href) || Boolean(onclick));

  /* iOS-style sheet: a soft lift and a light rim instead of an outline. */
  let classes = $derived(
    [
      'card-surface rounded-panel',
      paddings[padding],
      isInteractive
        ? 'block transition-[background-color,transform] duration-150 ease-standard hover:bg-surface-raised active:scale-[0.995]'
        : '',
      className,
    ]
      .filter(Boolean)
      .join(' ')
  );
</script>

{#if href}
  <a {href} class={classes} {...rest}>{@render children?.()}</a>
{:else if onclick}
  <button type="button" {onclick} class="{classes} w-full text-left" {...rest}>
    {@render children?.()}
  </button>
{:else}
  <div class={classes} {...rest}>{@render children?.()}</div>
{/if}
