<script lang="ts">
  import { t } from '$lib/translations';
  import {
    downloadFile,
    getFileText,
    processFile,
    isAudioMime,
    DICTATION_EVENTS,
    type FileRecord,
  } from '$lib/api';
  import { onDestroy, tick } from 'svelte';
  import { listen } from '@tauri-apps/api/event';
  import { Download, FileText, X } from 'lucide-svelte';
  import { errorText } from '$lib/translations/labels';
  import { Alert, Button, EmptyState, IconButton, ProgressBar } from '$lib/components/ui';
  import { fileExtension, formatFileSize } from '$lib/components/documents/file-meta';

  /*
   * Full-size preview of a patient file in a modal overlay: strong scrim,
   * focus moves in and is trapped, Esc or the scrim closes, and focus returns
   * to whatever opened it. PDFs and images render inline; dictation
   * recordings get an audio player plus their transcript, with an on-demand
   * "Transcribe" for recordings that have none yet.
   */
  interface Props {
    file: FileRecord | null;
    onClose?: () => void;
  }

  let { file, onClose }: Props = $props();

  const titleId = $props.id();
  let panel = $state<HTMLElement | null>(null);
  let returnFocus: HTMLElement | null = null;

  let blobUrl = $state<string | null>(null);
  let isLoading = $state(false);
  let errorMessage = $state('');

  async function loadFile() {
    if (!file) return;

    try {
      isLoading = true;
      errorMessage = '';

      const data = await downloadFile(file.id);
      const blob = new Blob([new Uint8Array(data)], { type: file.mime_type });

      // Revoke previous blob URL after the await so blobUrl is not read
      // synchronously inside $effect (which would make it a tracked dependency
      // and cause an infinite reload loop in Svelte 5).
      if (blobUrl) {
        URL.revokeObjectURL(blobUrl);
      }
      blobUrl = URL.createObjectURL(blob);
    } catch (error) {
      console.error('Failed to load file:', error);
      errorMessage = $t('files.loadFailed').replace('{error}', String(error));
    } finally {
      isLoading = false;
    }
  }

  // ── Dictation recordings: transcript + on-demand transcription ──────────
  let transcript = $state<string | null>(null);
  let transcribing = $state(false);
  let transcribeProgress = $state<{ done: number; total: number } | null>(null);
  let transcribeError = $state('');

  async function loadTranscript() {
    if (!file || !isAudioMime(file.mime_type)) return;
    transcript = await getFileText(file.id).catch(() => null);
  }

  async function transcribe() {
    if (!file) return;
    const fileId = file.id;
    transcribing = true;
    transcribeError = '';
    transcribeProgress = null;
    const unlisten = await listen<{ file_id: string; done: number; total: number }>(
      DICTATION_EVENTS.fileProgress,
      (e) => {
        if (e.payload.file_id === fileId) transcribeProgress = e.payload;
      }
    );
    try {
      await processFile(fileId);
      await loadTranscript();
    } catch (error) {
      transcribeError = $t('dictation.transcriptionFailed').replace('{error}', $errorText(error));
    } finally {
      unlisten();
      transcribing = false;
      transcribeProgress = null;
    }
  }

  function handleDownload() {
    if (!blobUrl || !file) return;

    const a = document.createElement('a');
    a.href = blobUrl;
    a.download = file.filename;
    document.body.appendChild(a);
    a.click();
    document.body.removeChild(a);
  }

  function handleClose() {
    if (blobUrl) {
      URL.revokeObjectURL(blobUrl);
      blobUrl = null;
    }
    onClose?.();
  }

  function focusable(): HTMLElement[] {
    if (!panel) return [];
    return Array.from(
      panel.querySelectorAll<HTMLElement>(
        'a[href], button:not([disabled]), input:not([disabled]), textarea:not([disabled]), audio[controls], iframe, [tabindex]:not([tabindex="-1"])'
      )
    );
  }

  function onKeydown(event: KeyboardEvent) {
    if (event.key === 'Escape') {
      event.preventDefault();
      event.stopPropagation();
      handleClose();
      return;
    }
    if (event.key !== 'Tab' || !panel) return;
    const targets = focusable();
    if (targets.length === 0) {
      event.preventDefault();
      panel.focus();
      return;
    }
    const first = targets[0];
    const last = targets[targets.length - 1];
    const active = document.activeElement;
    if (event.shiftKey && (active === first || active === panel)) {
      event.preventDefault();
      last.focus();
    } else if (!event.shiftKey && active === last) {
      event.preventDefault();
      first.focus();
    }
  }

  onDestroy(() => {
    if (blobUrl) {
      URL.revokeObjectURL(blobUrl);
    }
  });

  $effect(() => {
    if (file) {
      loadFile();
      transcript = null;
      transcribeError = '';
      loadTranscript();
    }
  });

  // Move focus into the overlay when it opens and back to the opener on close.
  $effect(() => {
    if (file && panel) {
      returnFocus ??= document.activeElement instanceof HTMLElement ? document.activeElement : null;
      void tick().then(() => panel?.querySelector<HTMLElement>('[data-viewer-close]')?.focus());
    } else if (!file && returnFocus) {
      returnFocus.focus();
      returnFocus = null;
    }
  });

  let transcribeLabel = $derived(
    transcribeProgress
      ? $t('dictation.transcribingFile')
          .replace('{done}', String(transcribeProgress.done))
          .replace('{total}', String(transcribeProgress.total))
      : $t('dictation.transcribing')
  );
</script>

<svelte:window onkeydown={file ? onKeydown : undefined} />

{#if file}
  <div class="fixed inset-0 z-50 flex items-center justify-center p-2 sm:p-6">
    <button
      type="button"
      class="fixed inset-0 bg-scrim-strong"
      aria-label={$t('common.close')}
      tabindex="-1"
      onclick={handleClose}
    ></button>

    <div
      bind:this={panel}
      class="relative z-10 flex h-full max-h-full w-full max-w-6xl flex-col overflow-hidden rounded-card border border-line bg-surface-overlay shadow-modal focus:outline-none"
      role="dialog"
      aria-modal="true"
      aria-labelledby={titleId}
      tabindex="-1"
    >
      <header class="flex shrink-0 items-center gap-3 border-b border-line-subtle px-4 py-2.5">
        <div class="min-w-0 flex-1">
          <h2 id={titleId} class="truncate text-heading text-fg" title={file.filename}>
            {file.filename}
          </h2>
          <p class="text-caption text-fg-subtle" data-numeric>
            {$t('files.viewerDescription')
              .replace('{type}', fileExtension(file.filename) || file.mime_type)
              .replace('{size}', formatFileSize(file.size_bytes))}
          </p>
        </div>
        <Button size="sm" onclick={handleDownload} disabled={!blobUrl}>
          <Download size={14} aria-hidden="true" />
          {$t('files.download')}
        </Button>
        <IconButton label={$t('common.close')} size="sm" onclick={handleClose} data-viewer-close>
          <X size={16} aria-hidden="true" />
        </IconButton>
      </header>

      <div class="min-h-0 flex-1 overflow-hidden bg-surface-sunken">
        {#if isLoading}
          <div class="flex h-full items-center justify-center p-6">
            <div class="w-full max-w-xs">
              <ProgressBar label={$t('files.loadingFile')} />
            </div>
          </div>
        {:else if errorMessage}
          <div class="flex h-full items-center justify-center p-6">
            <Alert tone="danger" class="max-w-md">{errorMessage}</Alert>
          </div>
        {:else if blobUrl}
          {#if file.mime_type === 'application/pdf'}
            <iframe src={blobUrl} title={file.filename} class="h-full w-full"></iframe>
          {:else if file.mime_type.startsWith('image/')}
            <div class="flex h-full items-center justify-center overflow-auto p-4">
              <img src={blobUrl} alt={file.filename} class="max-h-full max-w-full object-contain" />
            </div>
          {:else if isAudioMime(file.mime_type)}
            <div class="h-full overflow-auto bg-surface-overlay">
              <div class="mx-auto flex max-w-measure flex-col gap-4 px-4 py-6 sm:px-6">
                <audio controls src={blobUrl} class="w-full shrink-0"></audio>
                <div class="flex flex-wrap items-center justify-between gap-3">
                  <h3 class="text-heading text-fg">{$t('dictation.transcript')}</h3>
                  {#if !transcript}
                    <Button
                      variant="primary"
                      size="sm"
                      onclick={transcribe}
                      disabled={transcribing}
                    >
                      {$t('dictation.transcribe')}
                    </Button>
                  {/if}
                </div>
                {#if transcribing}
                  <div role="status" aria-live="polite">
                    <ProgressBar
                      value={transcribeProgress?.done ?? null}
                      max={transcribeProgress?.total ?? 100}
                      label={transcribeLabel}
                      size="sm"
                    />
                  </div>
                {/if}
                {#if transcribeError}
                  <Alert tone="danger">{transcribeError}</Alert>
                {/if}
                {#if transcript}
                  <pre
                    class="whitespace-pre-wrap rounded-card border border-line bg-surface-raised p-4 font-sans text-body text-fg">{transcript}</pre>
                {/if}
              </div>
            </div>
          {:else}
            <div class="flex h-full items-center justify-center p-6">
              <EmptyState
                icon={FileText}
                title={$t('files.previewNotAvailable')}
                class="w-full max-w-md bg-surface-overlay"
              >
                {#snippet action()}
                  <Button variant="primary" onclick={handleDownload}>
                    {$t('files.downloadToView')}
                  </Button>
                {/snippet}
              </EmptyState>
            </div>
          {/if}
        {/if}
      </div>
    </div>
  </div>
{/if}
