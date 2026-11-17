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
  import { onDestroy } from 'svelte';
  import { listen } from '@tauri-apps/api/event';
  import { Hourglass, FileText } from 'lucide-svelte';
  import { errorText } from '$lib/translations/labels';

  interface Props {
    file: FileRecord | null;
    onClose?: () => void;
  }

  let { file, onClose }: Props = $props();

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
      errorMessage = `Failed to load file: ${error}`;
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
</script>

{#if file}
  <div
    class="fixed inset-0 z-50 flex items-center justify-center bg-scrim-strong"
    role="presentation"
    onclick={handleClose}
    onkeydown={(e) => e.key === 'Escape' && handleClose()}
  >
    <div
      class="relative w-full h-full max-w-6xl max-h-[90vh] m-4"
      role="dialog"
      aria-modal="true"
      aria-label={file.filename}
      tabindex="-1"
      onclick={(e) => e.stopPropagation()}
      onkeydown={(e) => e.stopPropagation()}
    >
      <div
        class="absolute top-0 left-0 right-0 bg-surface-sunken border-b border-line-subtle p-4 flex items-center justify-between rounded-t-card"
      >
        <div class="flex-1 min-w-0">
          <h2 class="text-fg font-medium truncate" title={file.filename}>
            {file.filename}
          </h2>
        </div>

        <div class="flex items-center gap-2 ml-4">
          <button
            onclick={handleDownload}
            class="h-8 px-3 bg-accent hover:bg-accent-hover text-on-accent rounded-control transition-colors"
            disabled={!blobUrl}
          >
            {$t('files.download')}
          </button>

          <button
            onclick={handleClose}
            class="h-8 px-3 bg-surface-selected hover:bg-surface-selected text-fg-muted rounded-control transition-colors"
          >
            {$t('common.close')}
          </button>
        </div>
      </div>

      <div
        class="absolute top-16 bottom-0 left-0 right-0 bg-surface-sunken rounded-b-card overflow-hidden"
      >
        {#if isLoading}
          <div class="flex items-center justify-center h-full">
            <div class="text-center">
              <div class="mb-4 flex justify-center text-fg-muted">
                <Hourglass size={48} />
              </div>
              <p class="text-fg-muted">{$t('files.loadingFile')}</p>
            </div>
          </div>
        {:else if errorMessage}
          <div class="flex items-center justify-center h-full">
            <div class="bg-danger-subtle border border-danger-line rounded-card p-6 max-w-md">
              <p class="text-danger-fg">{errorMessage}</p>
            </div>
          </div>
        {:else if blobUrl}
          {#if file.mime_type === 'application/pdf'}
            <iframe src={blobUrl} title={file.filename} class="w-full h-full"></iframe>
          {:else if file.mime_type.startsWith('image/')}
            <div class="flex items-center justify-center h-full p-4 overflow-auto">
              <img src={blobUrl} alt={file.filename} class="max-w-full max-h-full object-contain" />
            </div>
          {:else if isAudioMime(file.mime_type)}
            <div class="flex flex-col h-full gap-4 p-6 overflow-auto">
              <audio controls src={blobUrl} class="w-full shrink-0"></audio>
              <div class="flex items-center justify-between gap-3">
                <h3 class="text-heading font-semibold text-fg">{$t('dictation.transcript')}</h3>
                {#if !transcript}
                  <button
                    onclick={transcribe}
                    disabled={transcribing}
                    class="h-8 px-3 bg-accent hover:bg-accent-hover disabled:opacity-50 text-on-accent rounded-control transition-colors"
                  >
                    {#if transcribing}
                      {transcribeProgress
                        ? $t('dictation.transcribingFile')
                            .replace('{done}', String(transcribeProgress.done))
                            .replace('{total}', String(transcribeProgress.total))
                        : $t('dictation.transcribing')}
                    {:else}
                      {$t('dictation.transcribe')}
                    {/if}
                  </button>
                {/if}
              </div>
              {#if transcribeError}
                <p class="text-caption text-danger-fg">{transcribeError}</p>
              {/if}
              {#if transcript}
                <pre
                  class="whitespace-pre-wrap font-sans text-body text-fg bg-surface-sunken border border-line rounded-card p-4">{transcript}</pre>
              {/if}
            </div>
          {:else}
            <div class="flex items-center justify-center h-full">
              <div class="text-center">
                <div class="mb-4 flex justify-center text-fg-muted">
                  <FileText size={48} />
                </div>
                <p class="text-fg-muted mb-4">{$t('files.previewNotAvailable')}</p>
                <button
                  onclick={handleDownload}
                  class="h-8 px-3 bg-accent hover:bg-accent-hover text-on-accent rounded-control transition-colors"
                >
                  {$t('files.downloadToView')}
                </button>
              </div>
            </div>
          {/if}
        {/if}
      </div>
    </div>
  </div>
{/if}
