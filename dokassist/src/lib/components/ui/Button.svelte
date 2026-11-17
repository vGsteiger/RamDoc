<script lang="ts">
  import type { Snippet } from 'svelte';
  import { LoaderCircle } from 'lucide-svelte';

  type ButtonVariant = 'primary' | 'secondary' | 'ghost' | 'danger' | 'danger-solid';
  type ButtonSize = 'sm' | 'md' | 'lg';

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
    variant?: ButtonVariant;
    size?: ButtonSize;
    type?: 'button' | 'submit' | 'reset';
    href?: string;
    disabled?: boolean;
    loading?: boolean;
    full?: boolean;
    class?: string;
    children?: Snippet;
    [key: string]: unknown;
  } = $props();

  /* Exactly one primary action per view: an ink fill. Everything else is
   * secondary or ghost. `danger` is an outline at rest — a filled red button
   * (`danger-solid`) belongs only inside a ConfirmDialog, where the user has
   * already chosen the destructive path. */
  const variants: Record<ButtonVariant, string> = {
    primary: 'border-transparent bg-accent text-on-accent hover:bg-accent-hover',
    secondary: 'border-line bg-surface-raised text-fg hover:bg-surface-hover',
    ghost: 'border-transparent text-fg-muted hover:bg-surface-hover hover:text-fg',
    danger: 'border-danger-line bg-surface-raised text-danger-fg hover:bg-danger-subtle',
    'danger-solid': 'border-transparent bg-danger text-on-danger hover:bg-danger-hover',
  };

  /* D5: 28 / 32 / 36 px. `lg` is for the chat composer and auth screens only. */
  const sizes: Record<ButtonSize, string> = {
    sm: 'h-control-sm gap-1.5 px-2.5 text-label',
    md: 'h-control gap-2 px-3 text-body',
    lg: 'h-control-lg gap-2 px-4 text-body',
  };

  // Anchors are never :disabled, so the disabled: variants below do nothing on
  // the href branch — an inert link would still navigate on click. That branch
  // gets the guard applied unconditionally instead.
  let inert = $derived(disabled || loading);

  let classes = $derived(
    [
      'inline-flex items-center justify-center rounded-control border font-medium leading-none',
      // A label never wraps or gets squeezed: overflow is a layout bug in the
      // parent, not something a button should absorb by growing taller.
      'shrink-0 whitespace-nowrap transition-colors duration-150 ease-standard',
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
  <button {type} class={classes} disabled={inert} aria-busy={loading || undefined} {...rest}>
    {#if loading}
      <LoaderCircle size={14} class="animate-spin" aria-hidden="true" />
    {/if}
    {@render children?.()}
  </button>
{/if}
