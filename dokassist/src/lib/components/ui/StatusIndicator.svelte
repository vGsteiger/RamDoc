<script lang="ts">
  import { CheckCircle2, AlertTriangle, XCircle, Circle, LoaderCircle } from 'lucide-svelte';

  type Tone = 'ok' | 'warn' | 'error' | 'idle' | 'busy';

  /*
   * A status is an icon plus a text label — never colour alone (model
   * loaded, embedding ready, sync failed). Each tone has its own icon shape,
   * so the state survives greyscale and colour-vision differences.
   *
   * With `onclick` or `href` it becomes a control with at least a 24px
   * target (WCAG 2.5.8), e.g. the sidebar "Model ready" chip.
   */
  let {
    tone = 'idle',
    label,
    detail = undefined,
    href = undefined,
    onclick = undefined,
    live = false,
    class: className = '',
    ...rest
  }: {
    tone?: Tone;
    /** Visible text, e.g. "Model ready" / "Modell bereit". Required. */
    label: string;
    /** Secondary text after the label, e.g. the model name. */
    detail?: string;
    href?: string;
    onclick?: (event: MouseEvent) => void;
    /** Announce changes politely (role="status"). */
    live?: boolean;
    class?: string;
    [key: string]: unknown;
  } = $props();

  const icons = {
    ok: CheckCircle2,
    warn: AlertTriangle,
    error: XCircle,
    idle: Circle,
    busy: LoaderCircle,
  };

  const tones: Record<Tone, string> = {
    ok: 'text-success-fg',
    warn: 'text-warning-fg',
    error: 'text-danger-fg',
    idle: 'text-fg-subtle',
    busy: 'text-fg-muted',
  };

  let Icon = $derived(icons[tone]);
  let interactive = $derived(Boolean(href || onclick));
  let classes = $derived(
    [
      'inline-flex min-w-0 max-w-full items-center gap-1.5 text-label',
      interactive
        ? 'min-h-6 rounded-control px-1.5 -mx-1.5 transition-colors duration-150 ease-standard hover:bg-surface-hover'
        : '',
      className,
    ].join(' ')
  );
</script>

{#snippet content()}
  <Icon
    size={14}
    class="shrink-0 {tones[tone]} {tone === 'busy' ? 'animate-spin' : ''}"
    aria-hidden="true"
  />
  <span class="truncate text-fg">{label}</span>
  {#if detail}
    <span class="truncate text-fg-muted">· {detail}</span>
  {/if}
{/snippet}

{#if href}
  <a {href} class={classes} data-tone={tone} {...rest}>{@render content()}</a>
{:else if onclick}
  <button type="button" {onclick} class={classes} data-tone={tone} {...rest}>
    {@render content()}
  </button>
{:else}
  <span
    class={classes}
    data-tone={tone}
    role={live ? 'status' : undefined}
    aria-live={live ? 'polite' : undefined}
    {...rest}>{@render content()}</span
  >
{/if}
