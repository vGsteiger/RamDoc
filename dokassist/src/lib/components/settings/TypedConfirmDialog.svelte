<script lang="ts">
  import type { Snippet } from 'svelte';
  import { AlertTriangle } from 'lucide-svelte';
  import { Button, Dialog, Input } from '$lib/components/ui';
  import { t } from '$lib/translations';

  /*
   * A confirmation that only proceeds once the user has typed a word
   * (RESET, RESTORE, EXPORT). The friction is the point: the confirm button
   * stays disabled until the input matches `word` exactly, and Enter submits
   * only on a match — the same rule the inline forms used before Settings
   * was split into panes.
   *
   * The parent owns the work: `onConfirm` runs the action, `busy` and
   * `error` reflect it, and the parent closes the dialog on success.
   */
  let {
    open = $bindable(false),
    title,
    word,
    confirmLabel,
    busyLabel = undefined,
    tone = 'danger',
    busy = false,
    error = '',
    onConfirm,
    onCancel = undefined,
    children,
  }: {
    open?: boolean;
    title: string;
    /** The exact text to type. Compared case-sensitively. */
    word: string;
    confirmLabel: string;
    busyLabel?: string;
    /** `danger` uses the filled red button; `default` the ink primary. */
    tone?: 'default' | 'danger';
    busy?: boolean;
    error?: string;
    onConfirm: () => void;
    onCancel?: () => void;
    /** The explanation shown above the input. */
    children?: Snippet;
  } = $props();

  let input = $state('');
  let matches = $derived(input === word);

  // Every opening starts from an empty field.
  $effect(() => {
    if (open) input = '';
  });

  function confirm() {
    if (!matches || busy) return;
    onConfirm();
  }

  function cancel() {
    if (busy) return;
    open = false;
    onCancel?.();
  }
</script>

<Dialog bind:open {title} size="sm" onClose={() => onCancel?.()}>
  <div class="text-body text-fg">{@render children?.()}</div>
  <Input
    bind:value={input}
    class="mt-3"
    placeholder={word}
    aria-label={$t('settings.typedConfirmLabel')}
    autocomplete="off"
    autocapitalize="off"
    spellcheck="false"
    disabled={busy}
    onkeydown={(e: KeyboardEvent) => {
      if (e.key === 'Enter') confirm();
    }}
  />
  {#if error}
    <p class="mt-2 text-caption text-danger-fg" role="alert">{error}</p>
  {/if}

  {#snippet footer()}
    <Button variant="secondary" onclick={cancel} disabled={busy}>{$t('common.cancel')}</Button>
    <Button
      variant={tone === 'danger' ? 'danger-solid' : 'primary'}
      disabled={!matches}
      loading={busy}
      onclick={confirm}
      data-confirm
    >
      {#if tone === 'danger' && !busy}
        <AlertTriangle size={14} aria-hidden="true" />
      {/if}
      {busy && busyLabel ? busyLabel : confirmLabel}
    </Button>
  {/snippet}
</Dialog>
