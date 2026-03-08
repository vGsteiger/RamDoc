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
    formatError
  } from '$lib/api';
  import ErrorDisplay from '$lib/components/ErrorDisplay.svelte';
  import { FileText, FileType, Check, AlertTriangle } from 'lucide-svelte';

  let literature: Literature[] = $state([]);
  let loading = $state(false);
  let error: AppError | null = $state(null);
  let uploadingFiles: Set<string> = $state(new Set());
  let processingFiles: Set<string> = $state(new Set());
  let editingDescription: string | null = $state(null);
  let descriptionText = $state('');

  let unlisten: UnlistenFn | null = null;

  onMount(async () => {
    await loadLiterature();

    // Listen for literature processing complete events
    unlisten = await listen<string>('literature-processed', (event) => {
      const litId = event.payload;
      processingFiles.delete(litId);
      loadLiterature();
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
        const data = new Uint8Array(arrayBuffer);

        const uploaded = await uploadLiterature(
          file.name,
          data,
          file.type || 'application/octet-stream',
          null
        );

        // Start processing in the background
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

    // Reset input
    input.value = '';
  }

  async function handleDelete(id: string) {
    if (!confirm('Are you sure you want to delete this literature document?')) {
      return;
    }

    error = null;
    try {
      await deleteLiteratureDocument(id);
      await loadLiterature();
    } catch (err) {
      error = parseError(err);
    }
  }

  async function handleDownload(lit: Literature) {
    error = null;
    try {
      const data = await downloadLiterature(lit.id);
      const blob = new Blob([data], { type: lit.mime_type });
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
      return date.toLocaleDateString('en-US', {
        year: 'numeric',
        month: 'short',
        day: 'numeric'
      });
    } catch {
      return dateStr;
    }
  }
</script>

<div class="h-full flex flex-col bg-gray-950">
  <div class="border-b border-gray-800 p-6">
    <h1 class="text-2xl font-bold text-gray-100">Literature</h1>
    <p class="text-gray-400 mt-2">
      Upload reference documents, medication guidelines, and treatment protocols for AI-powered search.
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
        class="flex items-center justify-center w-full h-32 px-4 transition bg-gray-900 border-2 border-gray-700 border-dashed rounded-lg hover:border-blue-500 cursor-pointer"
      >
        <div class="flex flex-col items-center space-y-2">
          <svg
            class="w-8 h-8 text-gray-400"
            fill="none"
            stroke="currentColor"
            viewBox="0 0 24 24"
          >
            <path
              stroke-linecap="round"
              stroke-linejoin="round"
              stroke-width="2"
              d="M7 16a4 4 0 01-.88-7.903A5 5 0 1115.9 6L16 6a5 5 0 011 9.9M15 13l-3-3m0 0l-3 3m3-3v12"
            />
          </svg>
          <span class="text-sm text-gray-400">
            Click to upload PDF or text files
          </span>
          <span class="text-xs text-gray-500">Max 500 MB per file</span>
        </div>
        <input
          type="file"
          class="hidden"
          accept=".pdf,.txt"
          multiple
          onchange={handleFileUpload}
        />
      </label>
    </div>

    <!-- Loading State -->
    {#if loading}
      <div class="text-center py-8">
        <div class="inline-block animate-spin rounded-full h-8 w-8 border-b-2 border-blue-500" />
        <p class="text-gray-400 mt-2">Loading literature...</p>
      </div>
    {:else if literature.length === 0}
      <!-- Empty State -->
      <div class="text-center py-12">
        <svg
          class="mx-auto h-12 w-12 text-gray-600"
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
        <h3 class="mt-2 text-sm font-medium text-gray-300">No literature documents</h3>
        <p class="mt-1 text-sm text-gray-500">
          Upload reference documents to enable AI-powered search in chat and reports.
        </p>
      </div>
    {:else}
      <!-- Literature List -->
      <div class="grid gap-4 grid-cols-1 lg:grid-cols-2 xl:grid-cols-3">
        {#each literature as lit}
          <div class="bg-gray-900 border border-gray-800 rounded-lg p-4">
            <div class="flex items-start justify-between mb-3">
              <div class="flex items-center gap-2">
                <span class="text-gray-400">
                  {#if lit.mime_type === 'application/pdf'}
                    <FileText size={24} />
                  {:else}
                    <FileType size={24} />
                  {/if}
                </span>
                <div class="min-w-0">
                  <h3 class="text-sm font-medium text-gray-100 truncate">
                    {lit.filename}
                  </h3>
                  <p class="text-xs text-gray-500">
                    {formatFileSize(lit.size_bytes)} · {formatDate(lit.created_at)}
                  </p>
                </div>
              </div>

              {#if processingFiles.has(lit.id)}
                <div
                  class="inline-block animate-spin rounded-full h-4 w-4 border-b-2 border-blue-500"
                  title="Processing…"
                />
              {:else if lit.chunk_count > 0}
                <span class="text-green-500" title="Extracted and embedded ({lit.chunk_count} chunks)">
                  <Check size={16} />
                </span>
              {:else}
                <span class="text-yellow-500" title="Not yet processed">
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
                    class="w-full px-3 py-2 bg-gray-800 border border-gray-700 rounded text-sm text-gray-100"
                    rows="3"
                    placeholder="Add a description..."
                  />
                  <div class="flex gap-2">
                    <button
                      onclick={() => saveDescription(lit.id)}
                      class="px-3 py-1 bg-blue-600 hover:bg-blue-700 text-white text-xs rounded"
                    >
                      Save
                    </button>
                    <button
                      onclick={cancelEditingDescription}
                      class="px-3 py-1 bg-gray-700 hover:bg-gray-600 text-gray-300 text-xs rounded"
                    >
                      Cancel
                    </button>
                  </div>
                </div>
              {:else if lit.description}
                <p class="text-xs text-gray-400">{lit.description}</p>
              {:else}
                <p class="text-xs text-gray-600 italic">No description</p>
              {/if}
            </div>

            <!-- Actions -->
            <div class="flex gap-2">
              <button
                onclick={() => handleDownload(lit)}
                class="flex-1 px-3 py-1.5 bg-gray-800 hover:bg-gray-700 text-gray-300 text-xs rounded transition-colors"
              >
                Download
              </button>
              {#if editingDescription !== lit.id}
                <button
                  onclick={() => startEditingDescription(lit)}
                  class="flex-1 px-3 py-1.5 bg-gray-800 hover:bg-gray-700 text-gray-300 text-xs rounded transition-colors"
                >
                  Edit
                </button>
              {/if}
              <button
                onclick={() => handleDelete(lit.id)}
                class="px-3 py-1.5 bg-red-900/30 hover:bg-red-900/50 text-red-400 text-xs rounded transition-colors"
              >
                Delete
              </button>
            </div>
          </div>
        {/each}
      </div>
    {/if}
  </div>
</div>
