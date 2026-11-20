<script module lang="ts">
  import type { ComponentType } from 'svelte';

  export interface MenuItem {
    label: string;
    onSelect: () => void;
    icon?: ComponentType;
    /** Destructive action: danger text, and visually separated from the rest. */
    danger?: boolean;
    disabled?: boolean;
    /** Draw a divider above this item. Danger items get one automatically. */
    separator?: boolean;
  }
</script>

<script lang="ts">
  import { tick, type Snippet } from 'svelte';
  import { MoreHorizontal } from 'lucide-svelte';
  import { t } from '$lib/translations';

  /*
   * Accessible dropdown menu for "⋯" overflow actions and Export.
   * Button with aria-haspopup/aria-expanded; role="menu" list of
   * role="menuitem"; Arrow keys, Home/End, Enter/Space select; Esc and Tab
   * close and Esc returns focus to the trigger; click outside closes.
   *
   * The popup is position: fixed, computed from the trigger, so it is never
   * clipped by an overflow:auto ancestor (list rows, sidebars).
   */
  let {
    items,
    label = undefined,
    align = 'end',
    size = 'md',
    triggerVariant = 'ghost',
    class: className = '',
    trigger,
  }: {
    items: MenuItem[];
    /** Accessible name of the trigger. Defaults to "More actions". */
    label?: string;
    align?: 'start' | 'end';
    size?: 'sm' | 'md';
    /** `ghost` icon button (default) or a `secondary` labelled button. */
    triggerVariant?: 'ghost' | 'secondary';
    class?: string;
    /** Custom trigger content (icon and/or text). Defaults to a ⋯ icon. */
    trigger?: Snippet;
  } = $props();

  const uid = $props.id();
  let open = $state(false);
  let button = $state<HTMLButtonElement | null>(null);
  let menu = $state<HTMLElement | null>(null);
  let position = $state({ top: 0, left: 0, right: 0 });

  function enabledItems() {
    return menu
      ? Array.from(menu.querySelectorAll<HTMLElement>('[role="menuitem"]:not([disabled])'))
      : [];
  }

  function place() {
    if (!button) return;
    const rect = button.getBoundingClientRect();
    position = {
      top: rect.bottom + 4,
      left: rect.left,
      right: window.innerWidth - rect.right,
    };
  }

  async function show(focus: 'first' | 'last' = 'first') {
    place();
    open = true;
    await tick();
    const targets = enabledItems();
    (focus === 'first' ? targets[0] : targets[targets.length - 1])?.focus();
  }

  function hide(returnFocus = true) {
    open = false;
    if (returnFocus) button?.focus();
  }

  function onTriggerKeydown(event: KeyboardEvent) {
    if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
      event.preventDefault();
      show(event.key === 'ArrowDown' ? 'first' : 'last');
    }
  }

  function onMenuKeydown(event: KeyboardEvent) {
    const targets = enabledItems();
    const index = targets.indexOf(document.activeElement as HTMLElement);
    const move = (to: number) => {
      event.preventDefault();
      targets[(to + targets.length) % targets.length]?.focus();
    };
    if (event.key === 'ArrowDown') move(index + 1);
    else if (event.key === 'ArrowUp') move(index - 1);
    else if (event.key === 'Home') move(0);
    else if (event.key === 'End') move(targets.length - 1);
    else if (event.key === 'Escape') {
      event.preventDefault();
      event.stopPropagation();
      hide();
    } else if (event.key === 'Tab') {
      hide(false);
    }
  }

  function choose(item: MenuItem) {
    if (item.disabled) return;
    hide();
    item.onSelect();
  }

  function onWindowPointerDown(event: PointerEvent) {
    const target = event.target as Node;
    if (menu?.contains(target) || button?.contains(target)) return;
    hide(false);
  }

  const triggerSizes = {
    sm: { ghost: 'size-control-sm', secondary: 'h-control-sm px-2.5 text-label' },
    md: { ghost: 'size-control', secondary: 'h-control px-3 text-body' },
  };
</script>

<svelte:window
  onpointerdown={open ? onWindowPointerDown : undefined}
  onresize={open ? () => hide(false) : undefined}
  onscrollcapture={open ? () => hide(false) : undefined}
/>

<div class="relative inline-flex {className}">
  <button
    bind:this={button}
    type="button"
    id="{uid}-trigger"
    aria-haspopup="menu"
    aria-expanded={open}
    aria-controls={open ? `${uid}-menu` : undefined}
    aria-label={trigger ? label : (label ?? $t('common.moreActions'))}
    title={trigger ? undefined : (label ?? $t('common.moreActions'))}
    class="inline-flex shrink-0 items-center justify-center gap-1.5 whitespace-nowrap rounded-control font-medium transition-colors duration-150 ease-standard {triggerSizes[
      size
    ][triggerVariant]} {triggerVariant === 'secondary'
      ? 'border border-line bg-surface-raised text-fg hover:bg-surface-hover'
      : 'text-fg-subtle hover:bg-surface-hover hover:text-fg'} {open ? 'bg-surface-hover' : ''}"
    onclick={() => (open ? hide(false) : show())}
    onkeydown={onTriggerKeydown}
  >
    {#if trigger}
      {@render trigger()}
    {:else}
      <MoreHorizontal size={16} aria-hidden="true" />
    {/if}
  </button>

  {#if open}
    <div
      bind:this={menu}
      id="{uid}-menu"
      role="menu"
      tabindex="-1"
      aria-labelledby="{uid}-trigger"
      class="fixed z-50 min-w-44 max-w-72 rounded-card border border-line bg-surface-overlay p-1 shadow-popover"
      style:top="{position.top}px"
      style:left={align === 'start' ? `${position.left}px` : undefined}
      style:right={align === 'end' ? `${position.right}px` : undefined}
      onkeydown={onMenuKeydown}
    >
      {#each items as item, i (i)}
        {@const Icon = item.icon}
        {#if i > 0 && (item.separator || (item.danger && !items[i - 1].danger))}
          <div role="separator" class="my-1 h-px bg-line-subtle"></div>
        {/if}
        <button
          type="button"
          role="menuitem"
          tabindex="-1"
          disabled={item.disabled}
          class="flex h-control w-full items-center gap-2 whitespace-nowrap rounded-control px-2 text-left text-body transition-colors duration-150 ease-standard focus:outline-none disabled:pointer-events-none disabled:opacity-50 {item.danger
            ? 'text-danger-fg hover:bg-danger-subtle focus:bg-danger-subtle'
            : 'text-fg hover:bg-surface-hover focus:bg-surface-hover'}"
          onclick={() => choose(item)}
        >
          {#if Icon}
            <Icon size={14} aria-hidden="true" class="shrink-0" />
          {/if}
          <span class="truncate">{item.label}</span>
        </button>
      {/each}
    </div>
  {/if}
</div>
