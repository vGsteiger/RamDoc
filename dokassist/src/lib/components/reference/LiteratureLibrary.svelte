<script lang="ts">
  import { onDestroy, onMount } from 'svelte';
  import { SvelteSet } from 'svelte/reactivity';
  import { listen, type UnlistenFn } from '@tauri-apps/api/event';
  import { BookOpen, Download, FileText, Pencil, Search, Trash2, Upload } from 'lucide-svelte';
  import {
    listAllLiterature,
    uploadLiterature,
    deleteLiteratureDocument,
    downloadLiterature,
    processLiterature,
    updateLiteratureMetadata,
    searchLiterature,
    parseError,
    type AppError,
    type Literature,
    type LiteratureChunkResult,
  } from '$lib/api';
  import ErrorDisplay from '$lib/components/ErrorDisplay.svelte';
  import {
    Button,
    ConfirmDialog,
    EmptyState,
    HighlightedSnippet,
    Input,
    ListSkeleton,
    Menu,
    SectionHeader,
    StatusIndicator,
    Textarea,
    type MenuItem,
  } from '$lib/components/ui';
  import { fmt } from '$lib/format';
  import { t } from '$lib/translations';

  /*
   * Reference → Literature: the uploaded guideline/protocol library that
   * chat and reports search. Upload, see indexing status per document,
   * (re)process, search passages, and manage descriptions.
   */
  let { initialQuery = '' }: { initialQuery?: string } = $props();

  let literature: Literature[] = $state([]);
  let loading = $state(true);
  let error: AppError | null = $state(null);
  let uploading = $state(0);
  const processing = new SvelteSet<string>();
  let editingId: string | null = $state(null);
  let descriptionText = $state('');
  let deleteTarget: Literature | null = $state(null);
  let deleteOpen = $state(false);

  let query = $state('');
  let results: LiteratureChunkResult[] = $state([]);
  let searchedFor = $state('');
  let searching = $state(false);
  let searchSequence = 0;

  let fileInput: HTMLInputElement | null = $state(null);
  let unlisten: UnlistenFn | null = null;
  let destroyed = false;

  onMount(() => {
    void loadLiterature();
    listen<string>('literature-processed', (event) => {
      processing.delete(event.payload);
      void loadLiterature();
    }).then((fn) => {
      if (destroyed) fn();
      else unlisten = fn;
    });
    if (initialQuery) {
      query = initialQuery;
      void runSearch();
    }
  });

  onDestroy(() => {
    destroyed = true;
    unlisten?.();
  });

  async function loadLiterature() {
    error = null;
    try {
      literature = await listAllLiterature(100, 0);
    } catch (err) {
      error = parseError(err);
    } finally {
      loading = false;
    }
  }

  function startProcessing(id: string) {
    processing.add(id);
    processLiterature(id).catch((err) => {
      console.error('Failed to process literature:', err);
      processing.delete(id);
      error = parseError(err);
    });
  }

  async function handleFiles(event: Event) {
    const input = event.target as HTMLInputElement;
    if (!input.files || input.files.length === 0) return;

    for (const file of Array.from(input.files)) {
      uploading += 1;
      error = null;
      try {
        const data = new Uint8Array((await file.arrayBuffer()) as ArrayBuffer);
        const uploaded = await uploadLiterature(
          file.name,
          data,
          file.type || 'application/octet-stream',
          null
        );
        startProcessing(uploaded.id);
        await loadLiterature();
      } catch (err) {
        error = parseError(err);
      } finally {
        uploading -= 1;
      }
    }
    input.value = '';
  }

  async function confirmDelete() {
    if (!deleteTarget) return;
    error = null;
    try {
      await deleteLiteratureDocument(deleteTarget.id);
      deleteTarget = null;
      await loadLiterature();
    } catch (err) {
      error = parseError(err);
      throw err;
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

  function startEditing(lit: Literature) {
    editingId = lit.id;
    descriptionText = lit.description || '';
  }

  async function saveDescription(id: string) {
    error = null;
    try {
      await updateLiteratureMetadata(id, descriptionText.trim() || null);
      editingId = null;
      await loadLiterature();
    } catch (err) {
      error = parseError(err);
    }
  }

  async function runSearch() {
    const q = query.trim();
    const seq = ++searchSequence;
    if (q.length < 2) {
      results = [];
      searchedFor = '';
      return;
    }
    searching = true;
    error = null;
    try {
      const found = await searchLiterature(q, 8);
      if (seq !== searchSequence) return;
      results = found;
      searchedFor = q;
    } catch (err) {
      if (seq !== searchSequence) return;
      results = [];
      error = parseError(err);
    } finally {
      if (seq === searchSequence) searching = false;
    }
  }

  function clearSearch() {
    query = '';
    results = [];
    searchedFor = '';
  }

  function formatFileSize(bytes: number): string {
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  }

  function menuItems(lit: Literature): MenuItem[] {
    return [
      { label: $t('files.download'), icon: Download, onSelect: () => handleDownload(lit) },
      { label: $t('reference.editDescription'), icon: Pencil, onSelect: () => startEditing(lit) },
      {
        label: $t('common.delete'),
        icon: Trash2,
        danger: true,
        onSelect: () => {
          deleteTarget = lit;
          deleteOpen = true;
        },
      },
    ];
  }

  let indexedCount = $derived(literature.filter((l) => l.chunk_count > 0).length);
</script>

<!-- The native file picker is the only way to choose files; it stays hidden
     and is opened by the Upload buttons. -->
<input
  bind:this={fileInput}
  type="file"
  class="hidden"
  accept=".pdf,.txt"
  multiple
  onchange={handleFiles}
/>

<div class="space-y-6">
  {#if error}
    <ErrorDisplay {error} showDetails={true} />
  {/if}

  {#if loading}
    <ListSkeleton count={4} />
  {:else if literature.length === 0}
    <EmptyState
      icon={BookOpen}
      title={$t('literature.noLiterature')}
      description={$t('literature.noLiteratureDesc')}
    >
      {#snippet action()}
        <div class="flex flex-col items-center gap-2">
          <Button variant="primary" loading={uploading > 0} onclick={() => fileInput?.click()}>
            <Upload size={14} aria-hidden="true" />
            {$t('reference.uploadDocuments')}
          </Button>
          <span class="text-caption text-fg-subtle">
            {$t('reference.acceptedFiles')} · {$t('literature.maxFileSize')}
          </span>
        </div>
      {/snippet}
    </EmptyState>
  {:else}
    <section aria-labelledby="literature-search-heading">
      <SectionHeader
        id="literature-search-heading"
        title={$t('reference.searchTitle')}
        description={$t('reference.searchDesc')}
      />
      <form
        class="flex max-w-xl gap-2"
        onsubmit={(e) => {
          e.preventDefault();
          void runSearch();
        }}
      >
        <div class="relative min-w-0 flex-1">
          <Search
            size={16}
            class="pointer-events-none absolute left-2.5 top-1/2 -translate-y-1/2 text-fg-subtle"
            aria-hidden="true"
          />
          <Input
            type="search"
            bind:value={query}
            aria-label={$t('literature.search')}
            placeholder={$t('literature.search')}
            class="pl-8"
            disabled={indexedCount === 0}
          />
        </div>
        <Button type="submit" loading={searching} disabled={indexedCount === 0}>
          {$t('reference.searchAction')}
        </Button>
      </form>
      {#if indexedCount === 0}
        <p class="mt-2 text-caption text-fg-muted">{$t('reference.searchNeedsIndex')}</p>
      {/if}

      {#if searchedFor}
        <div class="mt-4" aria-live="polite">
          <div class="mb-2 flex flex-wrap items-center justify-between gap-2">
            <p class="text-label text-fg-muted">
              {$t('reference.searchResultsFor')
                .replace('{count}', String(results.length))
                .replace('{query}', searchedFor)}
            </p>
            <Button size="sm" variant="ghost" onclick={clearSearch}>
              {$t('reference.clearSearch')}
            </Button>
          </div>
          {#if results.length === 0}
            <p
              class="rounded-card border border-dashed border-line px-4 py-6 text-center text-body text-fg-muted"
            >
              {$t('reference.searchNoResults')}
            </p>
          {:else}
            <ol
              class="divide-y divide-line-subtle rounded-card border border-line bg-surface-raised"
            >
              {#each results as hit (hit.chunk_id)}
                <li class="px-4 py-3">
                  <div class="flex flex-wrap items-baseline justify-between gap-x-3 gap-y-0.5">
                    <span class="min-w-0 truncate text-label font-medium text-fg">
                      {hit.filename}
                    </span>
                    <span class="shrink-0 text-caption text-fg-subtle" data-numeric>
                      {$t('reference.passage').replace('{n}', String(hit.chunk_index + 1))} ·
                      {$t('reference.relevance').replace(
                        '{percent}',
                        String(Math.round(hit.similarity * 100))
                      )}
                    </span>
                  </div>
                  <p class="mt-1 max-w-measure text-body leading-relaxed text-fg-muted">
                    <HighlightedSnippet text={hit.content} query={searchedFor} />
                  </p>
                </li>
              {/each}
            </ol>
          {/if}
        </div>
      {/if}
    </section>

    <section aria-labelledby="literature-docs-heading">
      <SectionHeader
        id="literature-docs-heading"
        title={$t('reference.documentsTitle')}
        description={`${$t('reference.acceptedFiles')} · ${$t('literature.maxFileSize')}`}
      >
        {#snippet actions()}
          <Button variant="primary" loading={uploading > 0} onclick={() => fileInput?.click()}>
            <Upload size={14} aria-hidden="true" />
            {$t('reference.uploadDocuments')}
          </Button>
        {/snippet}
      </SectionHeader>

      <ul class="divide-y divide-line-subtle rounded-card border border-line bg-surface-raised">
        {#each literature as lit (lit.id)}
          {@const isProcessing = processing.has(lit.id)}
          <li class="flex items-start gap-3 px-4 py-3">
            <FileText size={16} class="mt-0.5 shrink-0 text-fg-subtle" aria-hidden="true" />
            <div class="min-w-0 flex-1">
              <p class="truncate text-body font-medium text-fg" title={lit.filename}>
                {lit.filename}
              </p>
              <p class="mt-0.5 text-caption text-fg-subtle" data-numeric>
                {formatFileSize(lit.size_bytes)} · {$fmt.date(lit.created_at)}
              </p>

              {#if editingId === lit.id}
                <div class="mt-2 max-w-measure space-y-2">
                  <Textarea
                    bind:value={descriptionText}
                    rows={3}
                    aria-label={$t('reference.editDescription')}
                    placeholder={$t('literature.addDescription')}
                  />
                  <div class="flex gap-2">
                    <Button size="sm" onclick={() => saveDescription(lit.id)}>
                      {$t('common.save')}
                    </Button>
                    <Button size="sm" variant="ghost" onclick={() => (editingId = null)}>
                      {$t('common.cancel')}
                    </Button>
                  </div>
                </div>
              {:else if lit.description}
                <p class="mt-1 max-w-measure text-label text-fg-muted">{lit.description}</p>
              {/if}

              <div class="mt-2 flex flex-wrap items-center gap-x-3 gap-y-1">
                {#if isProcessing}
                  <StatusIndicator tone="busy" label={$t('literature.processingTitle')} live />
                {:else if lit.chunk_count > 0}
                  <StatusIndicator
                    tone="ok"
                    label={$t('reference.indexed')}
                    detail={$t('reference.passages').replace('{count}', String(lit.chunk_count))}
                  />
                {:else}
                  <StatusIndicator tone="warn" label={$t('literature.notProcessedTitle')} />
                  <Button size="sm" onclick={() => startProcessing(lit.id)}>
                    {$t('reference.process')}
                  </Button>
                {/if}
              </div>
            </div>
            <Menu
              items={menuItems(lit)}
              size="sm"
              label={$t('reference.documentActions').replace('{name}', lit.filename)}
            />
          </li>
        {/each}
      </ul>
    </section>
  {/if}
</div>

<ConfirmDialog
  bind:open={deleteOpen}
  tone="danger"
  title={$t('reference.deleteTitle')}
  message={deleteTarget
    ? $t('reference.deleteMessage').replace('{name}', deleteTarget.filename)
    : undefined}
  onConfirm={confirmDelete}
  onCancel={() => (deleteTarget = null)}
/>
