<script lang="ts">
  import type { Snippet } from 'svelte';

  type Tone = 'neutral' | 'success' | 'warning' | 'danger' | 'info';

  let {
    tone = 'neutral',
    class: className = '',
    children,
    ...rest
  }: {
    tone?: Tone;
    class?: string;
    children?: Snippet;
    [key: string]: unknown;
  } = $props();

  /* Badges are outlined with a subtle tint, never a saturated fill. A badge is
   * a label: it carries text, so colour is never its only signal. */
  const tones: Record<Tone, string> = {
    neutral: 'bg-surface-sunken text-fg-muted border-line',
    info: 'bg-info-subtle text-info-fg border-info-line',
    success: 'bg-success-subtle text-success-fg border-success-line',
    warning: 'bg-warning-subtle text-warning-fg border-warning-line',
    danger: 'bg-danger-subtle text-danger-fg border-danger-line',
  };

  let classes = $derived(
    [
      'inline-flex shrink-0 items-center gap-1 whitespace-nowrap rounded-control border px-1.5 py-0.5',
      'text-caption font-medium leading-none',
      tones[tone] ?? tones.neutral,
      className,
    ].join(' ')
  );
</script>

<span class={classes} {...rest}>{@render children?.()}</span>
