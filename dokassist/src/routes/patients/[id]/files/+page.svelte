<script lang="ts">
  import { onMount } from 'svelte';
  import { page } from '$app/stores';
  import { listFiles, deleteFile, downloadFile, type FileRecord } from '$lib/api';
  import FileUploader from '$lib/components/FileUploader.svelte';
  import FileCard from '$lib/components/FileCard.svelte';
  import FileViewer from '$lib/components/FileViewer.svelte';
  import { FolderOpen } from 'lucide-svelte';
  import { t } from '$lib/translations';
  import { Alert, EmptyState, ListSkeleton, PageHeader } from '$lib/components/ui';

  let patientId = $derived($page.params.id!);
  let files = $state<FileRecord[]>([]);
  let isLoading = $state(true);
  let errorMessage = $state('');
  let viewingFile = $state<FileRecord | null>(null);
  let _deletingFileId = $state<string | null>(null);

  async function loadFiles() {
    try {
      isLoading = true;
      errorMessage = '';
      files = await listFiles(patientId);
    } catch (error) {
      console.error('Failed to load files:', error);
      errorMessage = $t('files.failedToLoad');
    } finally {
      isLoading = false;
    }
  }

  function handleUpload(file: FileRecord) {
    files = [file, ...files];
  }

  function handleView(file: FileRecord) {
    viewingFile = file;
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
      errorMessage = $t('files.failedToDownload')
        .replace('{filename}', file.filename)
        .replace('{error}', String(error));
      setTimeout(() => (errorMessage = ''), 5000);
    }
  }

  async function handleDelete(file: FileRecord) {
    if (!confirm($t('files.confirmDelete').replace('{filename}', file.filename))) {
      return;
    }

    try {
      _deletingFileId = file.id;
      await deleteFile(file.id);
      files = files.filter((f) => f.id !== file.id);
    } catch (error) {
      console.error('Failed to delete file:', error);
      errorMessage = $t('files.failedToDelete')
        .replace('{filename}', file.filename)
        .replace('{error}', String(error));
      setTimeout(() => (errorMessage = ''), 5000);
    } finally {
      _deletingFileId = null;
    }
  }

  function handleCloseViewer() {
    viewingFile = null;
  }

  onMount(() => {
    loadFiles();
  });
</script>

<div class="p-8">
  <PageHeader title={$t('files.title')} />

  <div class="mb-6">
    <FileUploader {patientId} onUpload={handleUpload} />
  </div>

  {#if errorMessage}
    <Alert tone="danger" class="mb-4">{errorMessage}</Alert>
  {/if}

  {#if isLoading}
    <ListSkeleton variant="file" count={4} />
  {:else if files.length === 0}
    <EmptyState
      icon={FolderOpen}
      title={$t('files.noFilesUploaded')}
      description={$t('files.uploadHint')}
    />
  {:else}
    <div class="space-y-2">
      {#each files as file (file.id)}
        <FileCard {file} onView={handleView} onDownload={handleDownload} onDelete={handleDelete} />
      {/each}
    </div>
  {/if}
</div>

<FileViewer file={viewingFile} onClose={handleCloseViewer} />
