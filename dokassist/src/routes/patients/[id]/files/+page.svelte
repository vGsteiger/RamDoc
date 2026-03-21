<script lang="ts">
  import { onMount } from 'svelte';
  import { page } from '$app/stores';
  import { listFiles, deleteFile, downloadFile, type FileRecord } from '$lib/api';
  import FileUploader from '$lib/components/FileUploader.svelte';
  import FileCard from '$lib/components/FileCard.svelte';
  import FileViewer from '$lib/components/FileViewer.svelte';
  import { Hourglass, FolderOpen } from 'lucide-svelte';
  import { t } from '$lib/translations';

  let patientId = $derived($page.params.id);
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
  <h2 class="text-xl font-bold text-gray-900 dark:text-gray-100 mb-6">{$t('files.title')}</h2>

  <div class="mb-8">
    <FileUploader {patientId} onUpload={handleUpload} />
  </div>

  {#if errorMessage}
    <div
      class="bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 rounded-lg p-4 mb-6"
    >
      <p class="text-sm text-red-600 dark:text-red-400">{errorMessage}</p>
    </div>
  {/if}

  {#if isLoading}
    <div class="flex items-center justify-center py-12">
      <div class="text-center">
        <div class="mb-4 flex justify-center text-gray-400">
          <Hourglass size={48} />
        </div>
        <p class="text-gray-500 dark:text-gray-400">{$t('files.loading')}</p>
      </div>
    </div>
  {:else if files.length === 0}
    <div
      class="text-center py-12 bg-gray-50 dark:bg-gray-900 rounded-lg border border-gray-200 dark:border-gray-800"
    >
      <div class="mb-4 flex justify-center text-gray-400">
        <FolderOpen size={48} />
      </div>
      <p class="text-gray-500 dark:text-gray-400">{$t('files.noFilesUploaded')}</p>
      <p class="text-sm text-gray-400 dark:text-gray-500 mt-2">{$t('files.uploadHint')}</p>
    </div>
  {:else}
    <div class="space-y-4">
      {#each files as file (file.id)}
        <FileCard {file} onView={handleView} onDownload={handleDownload} onDelete={handleDelete} />
      {/each}
    </div>
  {/if}
</div>

<FileViewer file={viewingFile} onClose={handleCloseViewer} />
