<script lang="ts">
  import { downloadFile, type FileRecord } from '$lib/api';
  import { onMount, onDestroy } from 'svelte';

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
    }
  });
</script>

{#if file}
  <div class="fixed inset-0 z-50 flex items-center justify-center bg-black/80" onclick={handleClose}>
    <div class="relative w-full h-full max-w-6xl max-h-[90vh] m-4" onclick={(e) => e.stopPropagation()}>
      <div class="absolute top-0 left-0 right-0 bg-gray-900 border-b border-gray-800 p-4 flex items-center justify-between rounded-t-lg">
        <div class="flex-1 min-w-0">
          <h2 class="text-gray-100 font-medium truncate" title={file.filename}>
            {file.filename}
          </h2>
        </div>

        <div class="flex items-center gap-2 ml-4">
          <button
            onclick={handleDownload}
            class="px-4 py-2 bg-blue-600 hover:bg-blue-700 text-white rounded transition-colors"
            disabled={!blobUrl}
          >
            Download
          </button>

          <button
            onclick={handleClose}
            class="px-4 py-2 bg-gray-800 hover:bg-gray-700 text-gray-300 rounded transition-colors"
          >
            Close
          </button>
        </div>
      </div>

      <div class="absolute top-16 bottom-0 left-0 right-0 bg-gray-950 rounded-b-lg overflow-hidden">
        {#if isLoading}
          <div class="flex items-center justify-center h-full">
            <div class="text-center">
              <div class="text-4xl mb-4">⏳</div>
              <p class="text-gray-400">Loading file...</p>
            </div>
          </div>
        {:else if errorMessage}
          <div class="flex items-center justify-center h-full">
            <div class="bg-red-900/20 border border-red-800 rounded-lg p-6 max-w-md">
              <p class="text-red-400">{errorMessage}</p>
            </div>
          </div>
        {:else if blobUrl}
          {#if file.mime_type === 'application/pdf'}
            <iframe
              src={blobUrl}
              title={file.filename}
              class="w-full h-full"
            ></iframe>
          {:else if file.mime_type.startsWith('image/')}
            <div class="flex items-center justify-center h-full p-4 overflow-auto">
              <img
                src={blobUrl}
                alt={file.filename}
                class="max-w-full max-h-full object-contain"
              />
            </div>
          {:else}
            <div class="flex items-center justify-center h-full">
              <div class="text-center">
                <div class="text-4xl mb-4">📄</div>
                <p class="text-gray-400 mb-4">Preview not available for this file type</p>
                <button
                  onclick={handleDownload}
                  class="px-4 py-2 bg-blue-600 hover:bg-blue-700 text-white rounded transition-colors"
                >
                  Download to View
                </button>
              </div>
            </div>
          {/if}
        {/if}
      </div>
    </div>
  </div>
{/if}
