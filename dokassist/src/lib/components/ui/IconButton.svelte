<script lang="ts">
  import type { Snippet } from 'svelte';

  type Tone = 'default' | 'danger';
  type Size = 'sm' | 'md' | 'lg';

  let {
    label,
    tone = 'default',
    size = 'md',
    href = undefined,
    disabled = false,
    class: className = '',
    children,
    ...rest
  }: {
    label: string;
    tone?: Tone;
    size?: Size;
    href?: string;
    disabled?: boolean;
    class?: string;
    children?: Snippet;
    [key: string]: unknown;
  } = $props();

  const tones: Record<Tone, string> = {
    default: 'text-fg-subtle hover:bg-surface-hover hover:text-fg',
    danger: 'text-fg-subtle hover:bg-danger-subtle hover:text-danger-fg',
  };

  /* Square, matching Button heights (D5): 28 / 32 / 36 px. */
  const sizes: Record<Size, string> = {
    sm: 'size-control-sm',
    md: 'size-control',
    lg: 'size-control-lg',
  };

  let classes = $derived(
    [
      'inline-flex shrink-0 items-center justify-center rounded-control',
      sizes[size],
      'transition-colors duration-150 ease-standard',
      'disabled:pointer-events-none disabled:opacity-50',
      tones[tone],
      className,
    ].join(' ')
  );
</script>

{#if href}
  <!-- Anchors are never :disabled, so the disabled: variants in `classes` do
       nothing here; the guard is applied directly instead. -->
  <a
    {href}
    class="{classes}{disabled ? ' pointer-events-none opacity-50' : ''}"
    aria-label={label}
    title={label}
    aria-disabled={disabled || undefined}
    tabindex={disabled ? -1 : undefined}
    {...rest}>{@render children?.()}</a
  >
{:else}
  <button type="button" class={classes} aria-label={label} title={label} {disabled} {...rest}>
    {@render children?.()}
  </button>
{/if}
