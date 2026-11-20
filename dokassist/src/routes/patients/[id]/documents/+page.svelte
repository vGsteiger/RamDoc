<script lang="ts">
  import { onMount } from 'svelte';
  import { page } from '$app/state';
  import { goto } from '$app/navigation';
  import {
    ChevronDown,
    ClipboardList,
    FolderOpen,
    Mail,
    Plus,
    ScrollText,
    Upload,
  } from 'lucide-svelte';
  import {
    listFiles,
    listReports,
    listLetters,
    listEmails,
    deleteFile,
    deleteReport,
    deleteLetter,
    deleteEmail,
    downloadFile,
    type Email,
    type FileRecord,
    type Letter,
    type Report,
  } from '$lib/api';
  import { t } from '$lib/translations';
  import { errorText, reportTypeLabel } from '$lib/translations/labels';
  import {
    Alert,
    ConfirmDialog,
    EmptyState,
    ListSkeleton,
    Menu,
    PageShell,
    SegmentedControl,
    type MenuItem,
    type SegmentOption,
  } from '$lib/components/ui';
  import FileUploader from '$lib/components/FileUploader.svelte';
  import FileCard from '$lib/components/FileCard.svelte';
  import FileViewer from '$lib/components/FileViewer.svelte';
  import DocumentRow from '$lib/components/documents/DocumentRow.svelte';
  import {
    DOCUMENT_FILTERS,
    documentsUrl,
    filterDocuments,
    mergeDocuments,
    parseDocumentFilter,
    type DocumentFilter,
    type DocumentItem,
  } from '$lib/components/documents/documents';
  import { documentTitle } from '$lib/components/documents/titles';

  /*
   * One list of everything filed for the patient: uploaded files, reports,
   * letters and emails, newest first. `?type=files|reports|letters|emails`
   * selects a filter; the old per-type index routes redirect here with it.
   */
  let patientId = $derived(page.params.id!);
  let filter = $derived(parseDocumentFilter(page.url.searchParams.get('type')));

  let files = $state<FileRecord[]>([]);
  let reports = $state<Report[]>([]);
  let letters = $state<Letter[]>([]);
  let emails = $state<Email[]>([]);
  let loading = $state(true);
  let failedKinds = $state<DocumentFilter[]>([]);
  let actionError = $state('');

  let uploader = $state<{ browse: () => void } | null>(null);
  let viewingFile = $state<FileRecord | null>(null);
  let pendingDelete = $state<DocumentItem | null>(null);
  let confirmOpen = $state(false);

  let merged = $derived(mergeDocuments({ files, reports, letters, emails }));
  let visible = $derived(filterDocuments(merged, filter));
  let showUploader = $derived(filter === 'all' || filter === 'files');

  let filterOptions = $derived<SegmentOption[]>(
    DOCUMENT_FILTERS.map((value) => ({ value, label: $t(`documents.filters.${value}`) }))
  );

  let newItems = $derived<MenuItem[]>([
    { label: $t('documents.uploadFile'), icon: Upload, onSelect: () => uploader?.browse() },
    {
      label: $t('documents.newReport'),
      icon: ClipboardList,
      separator: true,
      onSelect: () => void goto(`/patients/${patientId}/reports/new`),
    },
    {
      label: $t('documents.newLetter'),
      icon: ScrollText,
      onSelect: () => void goto(`/patients/${patientId}/letters/new`),
    },
    {
      label: $t('documents.newEmail'),
      icon: Mail,
      onSelect: () => void goto(`/patients/${patientId}/email/new`),
    },
  ]);

  async function load() {
    loading = true;
    const id = patientId;
    const [f, r, l, e] = await Promise.allSettled([
      listFiles(id),
      listReports(id),
      listLetters(id),
      listEmails(id),
    ]);
    files = f.status === 'fulfilled' ? f.value : [];
    reports = r.status === 'fulfilled' ? r.value : [];
    letters = l.status === 'fulfilled' ? l.value : [];
    emails = e.status === 'fulfilled' ? e.value : [];
    const results: [DocumentFilter, PromiseSettledResult<unknown>][] = [
      ['files', f],
      ['reports', r],
      ['letters', l],
      ['emails', e],
    ];
    failedKinds = results.filter(([, res]) => res.status === 'rejected').map(([kind]) => kind);
    for (const [kind, res] of results) {
      if (res.status === 'rejected') console.error(`Failed to load ${kind}:`, res.reason);
    }
    loading = false;
  }

  function setFilter(next: string) {
    void goto(documentsUrl(patientId, parseDocumentFilter(next)), {
      replace: true,
      reset: false,
    });
  }

  function handleUpload(file: FileRecord) {
    files = [file, ...files];
  }

  async function handleDownload(file: FileRecord) {
    try {
      const data = await downloadFile(file.id);
      const blob = new Blob([new Uint8Array(data)], { type: file.mime_type });
      const url = URL.createObjectURL(blob);

      const a = document.createElement('a');
      a.href = url;
      a.download = file.filename;
      document.body.appendChild(a);
      a.click();
      document.body.removeChild(a);

      URL.revokeObjectURL(url);
    } catch (error) {
      console.error('Failed to download file:', error);
      actionError = $t('files.failedToDownload')
        .replace('{filename}', file.filename)
        .replace('{error}', String(error));
    }
  }

  function requestDelete(item: DocumentItem) {
    actionError = '';
    pendingDelete = item;
    confirmOpen = true;
  }

  function requestFileDelete(file: FileRecord) {
    const item = merged.find((d) => d.kind === 'file' && d.id === file.id);
    if (item) requestDelete(item);
  }

  async function confirmDelete() {
    const item = pendingDelete;
    if (!item) return;
    try {
      switch (item.kind) {
        case 'file':
          await deleteFile(item.id);
          files = files.filter((x) => x.id !== item.id);
          break;
        case 'report':
          await deleteReport(item.id);
          reports = reports.filter((x) => x.id !== item.id);
          break;
        case 'letter':
          await deleteLetter(item.id);
          letters = letters.filter((x) => x.id !== item.id);
          break;
        case 'email':
          await deleteEmail(item.id);
          emails = emails.filter((x) => x.id !== item.id);
          break;
      }
    } catch (error) {
      console.error('Failed to delete document:', error);
      actionError = $t('documents.deleteFailed')
        .replace('{name}', pendingName)
        .replace('{error}', $errorText(error));
    } finally {
      pendingDelete = null;
    }
  }

  let pendingName = $derived(
    pendingDelete ? documentTitle(pendingDelete, $t, $reportTypeLabel) : ''
  );

  onMount(() => {
    void load();
  });
</script>

<PageShell title={$t('documents.title')}>
  {#snippet actions()}
    <Menu items={newItems} triggerVariant="secondary" label={$t('documents.newMenu')}>
      {#snippet trigger()}
        <Plus size={14} aria-hidden="true" />
        {$t('documents.new')}
        <ChevronDown size={14} aria-hidden="true" class="text-fg-subtle" />
      {/snippet}
    </Menu>
  {/snippet}

  <div class="mb-4 flex min-w-0 flex-wrap items-center gap-3">
    <SegmentedControl
      options={filterOptions}
      value={filter}
      label={$t('documents.filterLabel')}
      onchange={setFilter}
      class="overflow-x-auto"
    />
  </div>

  <!-- Always mounted so "New ▾ Upload file" can open the picker from any
       filter; only shown where files are listed. -->
  <div class="mb-4 {showUploader ? '' : 'hidden'}">
    <FileUploader bind:this={uploader} {patientId} onUpload={handleUpload} variant="compact" />
  </div>

  {#if failedKinds.length > 0}
    <Alert tone="danger" class="mb-4">
      {$t('documents.partialError').replace(
        '{kinds}',
        failedKinds.map((kind) => $t(`documents.filters.${kind}`)).join(', ')
      )}
    </Alert>
  {/if}

  {#if actionError}
    <Alert tone="danger" class="mb-4">{actionError}</Alert>
  {/if}

  {#if loading}
    <ListSkeleton count={5} />
  {:else if visible.length === 0}
    <EmptyState
      icon={FolderOpen}
      title={$t(`documents.empty.${filter}`)}
      description={$t(`documents.emptyHint.${filter}`)}
    />
  {:else}
    <ul class="divide-y divide-line-subtle" aria-label={$t('documents.title')}>
      {#each visible as item (`${item.kind}:${item.id}`)}
        <li class="py-0.5">
          {#if item.kind === 'file'}
            <FileCard
              file={item.record}
              showKind={filter === 'all'}
              onView={(file) => (viewingFile = file)}
              onDownload={handleDownload}
              onDelete={requestFileDelete}
            />
          {:else}
            <DocumentRow {item} {patientId} onDelete={requestDelete} />
          {/if}
        </li>
      {/each}
    </ul>
  {/if}
</PageShell>

<FileViewer file={viewingFile} onClose={() => (viewingFile = null)} />

<ConfirmDialog
  bind:open={confirmOpen}
  tone="danger"
  title={$t('documents.deleteTitle')}
  message={$t('documents.confirmDelete').replace('{name}', pendingName)}
  onConfirm={confirmDelete}
  onCancel={() => (pendingDelete = null)}
/>
