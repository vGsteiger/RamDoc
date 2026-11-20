<script lang="ts">
  import { t } from '$lib/translations';

  /*
   * Determinate or indeterminate progress (downloads, imports, installs).
   * `value` is 0–max; null/undefined means indeterminate. Always give a
   * `label` naming what is progressing; `showValue` prints the percentage.
   * Indeterminate is an opacity pulse, not a sliding bar (motion rule:
   * colour and opacity only).
   */
  let {
    value = undefined,
    max = 100,
    label = undefined,
    showValue = false,
    size = 'md',
    tone = 'default',
    class: className = '',
  }: {
    value?: number | null;
    max?: number;
    label?: string;
    showValue?: boolean;
    size?: 'sm' | 'md';
    tone?: 'default' | 'danger';
    class?: string;
  } = $props();

  let determinate = $derived(typeof value === 'number' && Number.isFinite(value) && max > 0);
  let percent = $derived(
    determinate ? Math.round(Math.min(100, Math.max(0, ((value as number) / max) * 100))) : 0
  );
  let name = $derived(label ?? $t('common.progress'));
</script>

<div class="min-w-0 {className}">
  {#if label || showValue}
    <div class="mb-1 flex items-baseline justify-between gap-3 text-caption text-fg-muted">
      {#if label}<span class="truncate">{label}</span>{/if}
      {#if showValue && determinate}<span class="shrink-0" data-numeric>{percent}%</span>{/if}
    </div>
  {/if}
  <div
    role="progressbar"
    aria-label={name}
    aria-valuemin={determinate ? 0 : undefined}
    aria-valuemax={determinate ? 100 : undefined}
    aria-valuenow={determinate ? percent : undefined}
    class="relative w-full overflow-hidden rounded-full bg-surface-selected {size === 'sm'
      ? 'h-1'
      : 'h-1.5'}"
  >
    <div
      class="h-full rounded-full {tone === 'danger' ? 'bg-danger' : 'bg-accent'} {determinate
        ? ''
        : 'progress-indeterminate w-full'}"
      style:width={determinate ? `${percent}%` : undefined}
    ></div>
  </div>
</div>
