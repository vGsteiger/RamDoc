<script module lang="ts">
  export interface TabItem {
    id: string;
    label: string;
    /** Route tabs: renders links (navigation landmark) instead of ARIA tabs. */
    href?: string;
    /** Id of the tab panel this tab controls (ARIA tabs only). */
    controls?: string;
    /** Small trailing count, e.g. number of documents. */
    count?: number;
    disabled?: boolean;
  }
</script>

<script lang="ts">
  import { tick } from 'svelte';

  /*
   * Underline tabs. The selected tab carries a 2px ink underline and ink
   * text; the rest are muted. The strip scrolls horizontally inside its own
   * box when it does not fit, so it never widens the page (700px windows).
   *
   * Two modes:
   *  - In-page tabs (no href): role=tablist with roving tabindex; Arrow keys,
   *    Home and End move focus and select (automatic activation). Bind
   *    `value` and render the panel yourself with role="tabpanel",
   *    id = item.controls and aria-labelledby = tabId(item.id).
   *  - Route tabs (every item has href): a <nav> of links, the active one
   *    marked aria-current="page". Pass `value` = the active id.
   */
  let {
    items,
    value = $bindable(),
    label,
    size = 'md',
    idPrefix = undefined,
    onchange = undefined,
    class: className = '',
  }: {
    items: TabItem[];
    value?: string;
    /** Accessible name for the tab list / nav. */
    label: string;
    size?: 'sm' | 'md';
    /** Prefix for generated tab ids; defaults to a unique id. */
    idPrefix?: string;
    onchange?: (id: string) => void;
    class?: string;
  } = $props();

  const uid = $props.id();
  let prefix = $derived(idPrefix ?? uid);
  let isNav = $derived(items.length > 0 && items.every((item) => item.href));
  let strip = $state<HTMLElement | null>(null);

  export function tabId(id: string) {
    return `${prefix}-tab-${id}`;
  }

  async function select(id: string, focus = false) {
    if (value !== id) {
      value = id;
      onchange?.(id);
    }
    await tick();
    const el = strip?.querySelector<HTMLElement>(`[data-tab-id="${CSS.escape(id)}"]`);
    if (focus) el?.focus();
    // jsdom has no scrollIntoView; real browsers keep the active tab visible.
    el?.scrollIntoView?.({ block: 'nearest', inline: 'nearest' });
  }

  function onKeydown(event: KeyboardEvent) {
    const enabled = items.filter((item) => !item.disabled);
    if (enabled.length === 0) return;
    const current = enabled.findIndex((item) => item.id === value);
    let next: number | null = null;
    if (event.key === 'ArrowRight') next = (current + 1) % enabled.length;
    else if (event.key === 'ArrowLeft') next = (current - 1 + enabled.length) % enabled.length;
    else if (event.key === 'Home') next = 0;
    else if (event.key === 'End') next = enabled.length - 1;
    if (next === null) return;
    event.preventDefault();
    select(enabled[next].id, true);
  }

  const sizes = { sm: 'h-control-sm text-label', md: 'h-control text-body' };

  function tabClass(active: boolean) {
    return [
      'relative inline-flex shrink-0 items-center gap-1.5 whitespace-nowrap border-b-2 px-1 font-medium',
      'transition-colors duration-150 ease-standard focus-visible:outline-offset-[-2px]',
      'disabled:pointer-events-none disabled:opacity-50',
      sizes[size],
      active ? 'border-accent text-fg' : 'border-transparent text-fg-muted hover:text-fg',
    ].join(' ');
  }
</script>

{#snippet content(item: TabItem)}
  {item.label}
  {#if item.count !== undefined}
    <span class="text-caption text-fg-subtle" data-numeric>{item.count}</span>
  {/if}
{/snippet}

<div
  class="min-w-0 max-w-full overflow-x-auto [contain:inline-size] [scrollbar-width:none] {className}"
>
  {#if isNav}
    <nav aria-label={label}>
      <ul bind:this={strip} class="flex min-w-full w-max items-stretch gap-5 border-b border-line">
        {#each items as item (item.id)}
          <li class="flex">
            <a
              href={item.disabled ? undefined : item.href}
              data-tab-id={item.id}
              class={tabClass(item.id === value)}
              aria-current={item.id === value ? 'page' : undefined}
              aria-disabled={item.disabled || undefined}
            >
              {@render content(item)}
            </a>
          </li>
        {/each}
      </ul>
    </nav>
  {:else}
    <div
      bind:this={strip}
      role="tablist"
      aria-label={label}
      tabindex="-1"
      class="flex min-w-full w-max items-stretch gap-5 border-b border-line"
      onkeydown={onKeydown}
    >
      {#each items as item (item.id)}
        {@const active = item.id === value}
        <button
          type="button"
          role="tab"
          id={tabId(item.id)}
          data-tab-id={item.id}
          aria-selected={active}
          aria-controls={item.controls}
          tabindex={active || (!value && item === items[0]) ? 0 : -1}
          disabled={item.disabled}
          class={tabClass(active)}
          onclick={() => select(item.id)}
        >
          {@render content(item)}
        </button>
      {/each}
    </div>
  {/if}
</div>
