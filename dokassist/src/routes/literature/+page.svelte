<script lang="ts">
  import { onMount } from 'svelte';
  import { listen, type UnlistenFn } from '@tauri-apps/api/event';
  import {
    listAllLiterature,
    uploadLiterature,
    deleteLiteratureDocument,
    downloadLiterature,
    processLiterature,
    updateLiteratureMetadata,
    type Literature,
    type AppError,
    parseError,
  } from '$lib/api';
  import ErrorDisplay from '$lib/components/ErrorDisplay.svelte';
  import { FileText, FileType, Check, AlertTriangle } from 'lucide-svelte';
  import { ListSkeleton } from '$lib/components/ui';
  import { t } from '$lib/translations';

  let literature: Literature[] = $state([]);
  let loading = $state(false);
  let error: AppError | null = $state(null);
  let uploadingFiles: Set<string> = $state(new Set());
  let processingFiles: Set<string> = $state(new Set());
  let editingDescription: string | null = $state(null);
  let descriptionText = $state('');
  let confirmingDelete: string | null = $state(null);

  let unlisten: UnlistenFn | null = null;

  onMount(() => {
    loadLiterature().then(() => {
      listen<string>('literature-processed', (event) => {
        const litId = event.payload;
        processingFiles.delete(litId);
        loadLiterature();
      }).then((fn) => {
        unlisten = fn;
      });
    });

    return () => {
      if (unlisten) {
        unlisten();
      }
    };
  });

  async function loadLiterature() {
    loading = true;
    error = null;
    try {
      literature = await listAllLiterature(100, 0);
    } catch (err) {
      error = parseError(err);
    } finally {
      loading = false;
    }
  }

  async function handleFileUpload(event: Event) {
    const input = event.target as HTMLInputElement;
    if (!input.files || input.files.length === 0) return;

    for (const file of Array.from(input.files)) {
      if (uploadingFiles.has(file.name)) continue;

      uploadingFiles.add(file.name);
      error = null;

      try {
        const arrayBuffer = await file.arrayBuffer();
        const data = new Uint8Array(arrayBuffer as ArrayBuffer);

        const uploaded = await uploadLiterature(
          file.name,
          data,
          file.type || 'application/octet-stream',
          null
        );

        processingFiles.add(uploaded.id);
        processLiterature(uploaded.id).catch((err) => {
          console.error('Failed to process literature:', err);
          processingFiles.delete(uploaded.id);
        });

        await loadLiterature();
      } catch (err) {
        error = parseError(err);
      } finally {
        uploadingFiles.delete(file.name);
      }
    }

    input.value = '';
  }

  async function handleDelete(id: string) {
    error = null;
    try {
      await deleteLiteratureDocument(id);
      confirmingDelete = null;
      await loadLiterature();
    } catch (err) {
      error = parseError(err);
    }
  }

  async function handleDownload(lit: Literature) {
    error = null;
    try {
      const data = await downloadLiterature(lit.id);
      const blob = new Blob([data as unknown as Uint8Array<ArrayBuffer>], { type: lit.mime_type });
      const url = URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = url;
      a.download = lit.filename;
      document.body.appendChild(a);
      a.click();
      document.body.removeChild(a);
      URL.revokeObjectURL(url);
    } catch (err) {
      error = parseError(err);
    }
  }

  function startEditingDescription(lit: Literature) {
    editingDescription = lit.id;
    descriptionText = lit.description || '';
  }

  async function saveDescription(id: string) {
    error = null;
    try {
      await updateLiteratureMetadata(id, descriptionText.trim() || null);
      editingDescription = null;
      await loadLiterature();
    } catch (err) {
      error = parseError(err);
    }
  }

  function cancelEditingDescription() {
    editingDescription = null;
    descriptionText = '';
  }

  function formatFileSize(bytes: number): string {
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  }

  function formatDate(dateStr: string): string {
    try {
      const date = new Date(dateStr);
      return date.toLocaleDateString('de-CH', {
        year: 'numeric',
        month: 'short',
        day: 'numeric',
      });
    } catch {
      return dateStr;
    }
  }
</script>

<div class="h-full flex flex-col bg-surface">
  <div class="border-b border-line-subtle p-6">
    <h1 class="text-display font-semibold text-fg">{$t('literature.title')}</h1>
    <p class="text-fg-muted mt-2">
      {$t('literature.description')}
    </p>
  </div>

  <div class="flex-1 overflow-auto p-6">
    {#if error}
      <div class="mb-4">
        <ErrorDisplay {error} showDetails={true} />
      </div>
    {/if}

    <!-- Upload Section -->
    <div class="mb-6">
      <label
        class="flex items-center justify-center w-full h-32 px-4 transition bg-surface-sunken border-2 border-line border-dashed rounded-card hover:border-accent cursor-pointer"
      >
        <div class="flex flex-col items-center space-y-2">
          <svg class="w-8 h-8 text-fg-muted" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path
              stroke-linecap="round"
              stroke-linejoin="round"
              stroke-width="2"
              d="M7 16a4 4 0 01-.88-7.903A5 5 0 1115.9 6L16 6a5 5 0 011 9.9M15 13l-3-3m0 0l-3 3m3-3v12"
            />
          </svg>
          <span class="text-body text-fg-muted">
            {$t('literature.clickToUpload')}
          </span>
          <span class="text-caption text-fg-subtle">{$t('literature.maxFileSize')}</span>
        </div>
        <input type="file" class="hidden" accept=".pdf,.txt" multiple onchange={handleFileUpload} />
      </label>
    </div>

    <!-- Loading State -->
    {#if loading}
      <ListSkeleton variant="tile" count={6} />
    {:else if literature.length === 0}
      <!-- Empty State -->
      <div class="text-center py-12">
        <svg
          class="mx-auto h-12 w-12 text-fg-subtle"
          fill="none"
          stroke="currentColor"
          viewBox="0 0 24 24"
        >
          <path
            stroke-linecap="round"
            stroke-linejoin="round"
            stroke-width="2"
            d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z"
          />
        </svg>
        <h3 class="mt-2 text-body font-medium text-fg-muted">
          {$t('literature.noLiterature')}
        </h3>
        <p class="mt-1 text-body text-fg-muted">
          {$t('literature.noLiteratureDesc')}
        </p>
      </div>
    {:else}
      <!-- Literature List -->
      <div class="grid gap-4 grid-cols-1 lg:grid-cols-2 xl:grid-cols-3">
        {#each literature as lit}
          <div class="bg-surface-raised border border-line-subtle rounded-card p-4">
            <div class="flex items-start justify-between mb-3">
              <div class="flex items-center gap-2">
                <span class="text-fg-muted">
                  {#if lit.mime_type === 'application/pdf'}
                    <FileText size={24} />
                  {:else}
                    <FileType size={24} />
                  {/if}
                </span>
                <div class="min-w-0">
                  <h3 class="text-body font-medium text-fg truncate">
                    {lit.filename}
                  </h3>
                  <p class="text-caption text-fg-subtle">
                    {formatFileSize(lit.size_bytes)} · {formatDate(lit.created_at)}
                  </p>
                </div>
              </div>

              {#if processingFiles.has(lit.id)}
                <div
                  class="inline-block animate-spin rounded-full h-4 w-4 border-b-2 border-accent"
                  title={$t('literature.processingTitle')}
                ></div>
              {:else if lit.chunk_count > 0}
                <span
                  class="text-success-fg"
                  title={$t('literature.processedTitle').replace(
                    '{count}',
                    String(lit.chunk_count)
                  )}
                >
                  <Check size={16} />
                </span>
              {:else}
                <span class="text-warning-fg" title={$t('literature.notProcessedTitle')}>
                  <AlertTriangle size={16} />
                </span>
              {/if}
            </div>

            <!-- Description -->
            <div class="mb-3">
              {#if editingDescription === lit.id}
                <div class="space-y-2">
                  <textarea
                    bind:value={descriptionText}
                    class="w-full px-3 py-2 bg-surface-hover border border-line rounded-control text-body text-fg"
                    rows="3"
                    placeholder={$t('literature.addDescription')}></textarea>
                  <div class="flex gap-2">
                    <button
                      onclick={() => saveDescription(lit.id)}
                      class="h-7 px-2.5 bg-accent hover:bg-accent-hover text-on-accent text-caption rounded-control"
                    >
                      {$t('common.save')}
                    </button>
                    <button
                      onclick={cancelEditingDescription}
                      class="h-7 px-2.5 bg-surface-selected hover:bg-surface-selected text-fg-muted text-caption rounded-control"
                    >
                      {$t('common.cancel')}
                    </button>
                  </div>
                </div>
              {:else if lit.description}
                <p class="text-caption text-fg-muted">{lit.description}</p>
              {:else}
                <p class="text-caption text-fg-subtle italic">
                  {$t('literature.noDescription')}
                </p>
              {/if}
            </div>

            <!-- Actions -->
            <div class="flex gap-2">
              <button
                onclick={() => handleDownload(lit)}
                class="flex-1 h-7 px-2.5 bg-surface-hover hover:bg-surface-hover text-fg-muted text-caption rounded-control transition-colors"
              >
                {$t('files.download')}
              </button>
              {#if editingDescription !== lit.id}
                <button
                  onclick={() => startEditingDescription(lit)}
                  class="flex-1 h-7 px-2.5 bg-surface-hover hover:bg-surface-hover text-fg-muted text-caption rounded-control transition-colors"
                >
                  {$t('common.edit')}
                </button>
              {/if}
              {#if confirmingDelete === lit.id}
                <button
                  onclick={() => handleDelete(lit.id)}
                  class="h-7 px-2.5 bg-danger hover:bg-danger-hover text-on-danger text-caption rounded-control transition-colors"
                >
                  {$t('common.confirm')}
                </button>
                <button
                  onclick={() => (confirmingDelete = null)}
                  class="h-7 px-2.5 bg-surface-hover hover:bg-surface-hover text-fg-muted text-caption rounded-control transition-colors"
                >
                  {$t('common.cancel')}
                </button>
              {:else}
                <button
                  onclick={() => (confirmingDelete = lit.id)}
                  class="h-7 px-2.5 bg-danger-subtle hover:bg-danger-subtle text-danger-fg text-caption rounded-control transition-colors"
                >
                  {$t('common.delete')}
                </button>
              {/if}
            </div>
          </div>
        {/each}
      </div>
    {/if}
  </div>
</div>
