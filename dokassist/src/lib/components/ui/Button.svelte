<script lang="ts">
  import type { Snippet } from 'svelte';
  import { LoaderCircle } from 'lucide-svelte';

  type Variant = 'primary' | 'secondary' | 'ghost' | 'subtle' | 'danger';
  type Size = 'sm' | 'md';

  let {
    variant = 'secondary',
    size = 'md',
    type = 'button',
    href = undefined,
    disabled = false,
    loading = false,
    full = false,
    class: className = '',
    children,
    ...rest
  }: {
    variant?: Variant;
    size?: Size;
    type?: 'button' | 'submit' | 'reset';
    href?: string;
    disabled?: boolean;
    loading?: boolean;
    full?: boolean;
    class?: string;
    children?: Snippet;
    [key: string]: unknown;
  } = $props();

  /* Exactly one primary action per view. Everything else is secondary or
   * ghost — the accent stops meaning anything if it fills three buttons. */
  const variants: Record<Variant, string> = {
    primary: 'border-transparent bg-accent text-on-accent hover:bg-accent-hover',
    secondary: 'border-line bg-surface-raised text-fg hover:bg-surface-hover',
    ghost: 'border-transparent text-fg-muted hover:bg-surface-hover hover:text-fg',
    subtle: 'border-transparent bg-surface-hover text-fg hover:bg-surface-selected',
    danger: 'border-transparent bg-danger text-on-danger hover:bg-danger-hover',
  };

  const sizes: Record<Size, string> = {
    sm: 'min-h-9 gap-1.5 px-3 text-label',
    md: 'min-h-10 gap-2 px-3.5 text-body',
  };

  // Anchors are never :disabled, so the disabled: variants below do nothing on
  // the href branch — an inert link would still navigate on click. That branch
  // gets the guard applied unconditionally instead.
  let inert = $derived(disabled || loading);

  let classes = $derived(
    [
      'inline-flex items-center justify-center rounded-control border font-medium leading-none',
      'whitespace-nowrap transition-colors duration-150 ease-standard',
      'disabled:pointer-events-none disabled:opacity-50',
      sizes[size],
      variants[variant],
      full ? 'w-full' : '',
      className,
    ]
      .filter(Boolean)
      .join(' ')
  );
</script>

{#if href}
  <a
    {href}
    class="{classes}{inert ? ' pointer-events-none opacity-50' : ''}"
    aria-disabled={inert || undefined}
    tabindex={inert ? -1 : undefined}
    {...rest}
  >
    {#if loading}
      <LoaderCircle size={14} class="animate-spin" aria-hidden="true" />
    {/if}
    {@render children?.()}
  </a>
{:else}
  <button {type} class={classes} disabled={inert} {...rest}>
    {#if loading}
      <LoaderCircle size={14} class="animate-spin" aria-hidden="true" />
    {/if}
    {@render children?.()}
  </button>
{/if}
