<script module lang="ts">
  import type { ComponentType } from 'svelte';

  export interface SegmentOption {
    value: string;
    label: string;
    /** Optional lucide icon shown before the label. */
    icon?: ComponentType;
    disabled?: boolean;
  }
</script>

<script lang="ts">
  import { tick } from 'svelte';

  /*
   * Mutually exclusive options (calendar view, theme, document type). A
   * radiogroup: one tab stop, Arrow keys move and select. iOS style: a grey
   * capsule track with the selected segment as a raised white knob.
   */
  let {
    options,
    value = $bindable(),
    label = undefined,
    size = 'md',
    full = false,
    onchange = undefined,
    class: className = '',
    ...rest
  }: {
    options: SegmentOption[];
    value?: string;
    /** Accessible name; or pass aria-labelledby. */
    label?: string;
    size?: 'sm' | 'md';
    /** Stretch segments to fill the width. */
    full?: boolean;
    onchange?: (value: string) => void;
    class?: string;
    [key: string]: unknown;
  } = $props();

  let group = $state<HTMLElement | null>(null);

  async function choose(next: string, focus = false) {
    if (value !== next) {
      value = next;
      onchange?.(next);
    }
    if (focus) {
      await tick();
      group?.querySelector<HTMLElement>(`[data-value="${CSS.escape(next)}"]`)?.focus();
    }
  }

  function onKeydown(event: KeyboardEvent) {
    const enabled = options.filter((option) => !option.disabled);
    const current = enabled.findIndex((option) => option.value === value);
    let next: number | null = null;
    if (event.key === 'ArrowRight' || event.key === 'ArrowDown') next = current + 1;
    else if (event.key === 'ArrowLeft' || event.key === 'ArrowUp') next = current - 1;
    else if (event.key === 'Home') next = 0;
    else if (event.key === 'End') next = enabled.length - 1;
    if (next === null || enabled.length === 0) return;
    event.preventDefault();
    choose(enabled[(next + enabled.length) % enabled.length].value, true);
  }

  const sizes = { sm: 'h-control-sm text-label', md: 'h-control text-body' };
</script>

<div
  bind:this={group}
  role="radiogroup"
  aria-label={label}
  tabindex="-1"
  class="{full
    ? 'flex w-full'
    : 'inline-flex'} max-w-full items-stretch gap-0.5 rounded-full bg-fill p-0.5 {sizes[
    size
  ]} {className}"
  onkeydown={onKeydown}
  {...rest}
>
  {#each options as option (option.value)}
    {@const checked = option.value === value}
    {@const Icon = option.icon}
    <button
      type="button"
      role="radio"
      aria-checked={checked}
      data-value={option.value}
      tabindex={checked || (value === undefined && option === options[0]) ? 0 : -1}
      disabled={option.disabled}
      class="inline-flex min-w-0 items-center justify-center gap-1.5 whitespace-nowrap rounded-full px-3 font-medium transition-[background-color,color,box-shadow] duration-200 ease-standard disabled:pointer-events-none disabled:opacity-50 {full
        ? 'flex-1'
        : 'shrink-0'} {checked
        ? 'bg-surface-raised text-fg shadow-control'
        : 'text-fg-muted hover:text-fg'}"
      onclick={() => choose(option.value)}
    >
      {#if Icon}
        <Icon size={14} aria-hidden="true" />
      {/if}
      <span class="truncate">{option.label}</span>
    </button>
  {/each}
</div>
