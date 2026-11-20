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

  /* iOS-style capsules. Exactly one primary action per view: an ink capsule
   * with a soft lift. Secondary is a grey fill with no outline, ghost is text
   * that gains the fill on hover. `danger` is red text on the grey fill — a
   * filled red capsule (`danger-solid`) belongs only inside a ConfirmDialog,
   * where the user has already chosen the destructive path. */
  const variants: Record<ButtonVariant, string> = {
    primary: 'bg-accent text-on-accent shadow-control hover:bg-accent-hover',
    secondary: 'bg-fill text-fg hover:bg-fill-strong',
    ghost: 'text-fg-muted hover:bg-fill hover:text-fg',
    danger: 'bg-fill text-danger-fg hover:bg-danger-subtle',
    'danger-solid': 'bg-danger text-on-danger shadow-control hover:bg-danger-hover',
  };

  /* D5: 28 / 32 / 36 px. `lg` is for the chat composer and auth screens only. */
  const sizes: Record<ButtonSize, string> = {
    sm: 'h-control-sm gap-1.5 px-3 text-label',
    md: 'h-control gap-2 px-4 text-body',
    lg: 'h-control-lg gap-2 px-5 text-body',
  };

  // Anchors are never :disabled, so the disabled: variants below do nothing on
  // the href branch — an inert link would still navigate on click. That branch
  // gets the guard applied unconditionally instead.
  let inert = $derived(disabled || loading);

  let classes = $derived(
    [
      'inline-flex items-center justify-center rounded-full font-medium leading-none',
      // A label never wraps or gets squeezed: overflow is a layout bug in the
      // parent, not something a button should absorb by growing taller.
      'shrink-0 whitespace-nowrap transition-[background-color,color,transform] duration-150 ease-standard active:scale-[0.97]',
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
