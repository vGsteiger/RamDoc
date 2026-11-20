<script lang="ts">
  import type { Snippet } from 'svelte';
  import { AlertTriangle } from 'lucide-svelte';
  import { t } from '$lib/translations';
  import Button from './Button.svelte';
  import Dialog from './Dialog.svelte';

  /*
   * Replaces window.confirm() and hand-rolled confirmation modals.
   * Built on Dialog: focus moves in and is trapped, Esc and the scrim cancel,
   * and focus returns to the trigger on close.
   *
   * For `tone="danger"` the confirm button is the filled danger button (the
   * only place one is allowed), it carries a warning icon, and initial focus
   * lands on Cancel so Enter never destroys anything by accident. Name the
   * record in `message` ("Delete the diagnosis F33.1?").
   *
   * `onConfirm` may return a promise; the dialog shows a busy state and stays
   * open until it settles, then closes. Throw to keep it open (show the error
   * yourself).
   */
  let {
    open = $bindable(false),
    title,
    message = undefined,
    confirmLabel = undefined,
    cancelLabel = undefined,
    tone = 'default',
    onConfirm,
    onCancel = undefined,
    children,
  }: {
    open?: boolean;
    title: string;
    message?: string;
    /** Defaults to "Delete" for danger, "Confirm" otherwise. Use a verb. */
    confirmLabel?: string;
    cancelLabel?: string;
    tone?: 'default' | 'danger';
    onConfirm: () => void | Promise<void>;
    onCancel?: () => void;
    /** Extra body content (e.g. a typed confirmation field). */
    children?: Snippet;
  } = $props();

  let busy = $state(false);

  async function confirm() {
    busy = true;
    try {
      await onConfirm();
      open = false;
    } catch {
      // The caller reports the error; keep the dialog open for a retry.
    } finally {
      busy = false;
    }
  }

  function cancel() {
    if (busy) return;
    open = false;
    onCancel?.();
  }
</script>

<Dialog bind:open {title} size="sm" onClose={() => onCancel?.()}>
  {#if message}
    <p class="text-body text-fg">{message}</p>
  {/if}
  {@render children?.()}
  {#if tone === 'danger'}
    <p class="mt-2 text-label text-fg-muted">{$t('common.cannotUndo')}</p>
  {/if}

  {#snippet footer()}
    <Button variant="secondary" onclick={cancel} disabled={busy}>
      {cancelLabel ?? $t('common.cancel')}
    </Button>
    <Button
      variant={tone === 'danger' ? 'danger-solid' : 'primary'}
      onclick={confirm}
      loading={busy}
      data-confirm
    >
      {#if tone === 'danger' && !busy}
        <AlertTriangle size={14} aria-hidden="true" />
      {/if}
      {confirmLabel ?? (tone === 'danger' ? $t('common.delete') : $t('common.confirm'))}
    </Button>
  {/snippet}
</Dialog>
