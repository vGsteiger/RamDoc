<script lang="ts">
  import type { Snippet } from 'svelte';

  /*
   * CSS-only tooltip for rail icons and icon buttons: shown on hover and on
   * keyboard focus within, hidden from pointer events. The tooltip is a
   * supplementary visual hint; the trigger must still have its own accessible
   * name (IconButton's `label`). Wire `describedby` if the text adds
   * information beyond the name:
   *
   *   <Tooltip text={$t('nav.chat')} side="right">
   *     {#snippet children(describedby)}
   *       <IconButton label={$t('nav.chat')} aria-describedby={describedby}>…</IconButton>
   *     {/snippet}
   *   </Tooltip>
   */
  let {
    text,
    side = 'top',
    class: className = '',
    children,
  }: {
    text: string;
    side?: 'top' | 'bottom' | 'left' | 'right';
    class?: string;
    children: Snippet<[string]>;
  } = $props();

  const id = $props.id();

  const sides = {
    top: 'bottom-full left-1/2 mb-1.5 -translate-x-1/2',
    bottom: 'top-full left-1/2 mt-1.5 -translate-x-1/2',
    left: 'right-full top-1/2 mr-1.5 -translate-y-1/2',
    right: 'left-full top-1/2 ml-1.5 -translate-y-1/2',
  };
</script>

<span class="group/tooltip relative inline-flex {className}">
  {@render children(id)}
  <span
    {id}
    role="tooltip"
    class="pointer-events-none absolute z-50 whitespace-nowrap rounded-control bg-accent px-2 py-1 text-caption text-on-accent opacity-0 shadow-popover transition-opacity duration-150 ease-standard group-hover/tooltip:opacity-100 group-focus-within/tooltip:opacity-100 {sides[
      side
    ]}"
  >
    {text}
  </span>
</span>
