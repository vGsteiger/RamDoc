<script lang="ts">
  import type { FileRecord } from '$lib/api';

  interface Props {
    file: FileRecord;
    onView?: (file: FileRecord) => void;
    onDownload?: (file: FileRecord) => void;
    onDelete?: (file: FileRecord) => void;
  }

  let { file, onView, onDownload, onDelete }: Props = $props();

  function formatFileSize(bytes: number): string {
    if (bytes < 1024) return bytes + ' B';
    if (bytes < 1024 * 1024) return (bytes / 1024).toFixed(1) + ' KB';
    return (bytes / (1024 * 1024)).toFixed(1) + ' MB';
  }

  function formatDate(dateString: string): string {
    const date = new Date(dateString);
    return date.toLocaleDateString() + ' ' + date.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
  }

  function getFileIcon(mimeType: string): string {
    if (mimeType.startsWith('image/')) return '🖼️';
    if (mimeType === 'application/pdf') return '📄';
    if (mimeType.includes('word')) return '📝';
    return '📎';
  }

  function getFileExtension(filename: string): string {
    const parts = filename.split('.');
    return parts.length > 1 ? parts[parts.length - 1].toUpperCase() : '';
  }
</script>

<div class="bg-gray-900 border border-gray-800 rounded-lg p-4 hover:border-gray-700 transition-colors">
  <div class="flex items-start gap-4">
    <div class="text-3xl flex-shrink-0">
      {getFileIcon(file.mime_type)}
    </div>

    <div class="flex-1 min-w-0">
      <div class="flex items-start justify-between gap-2">
        <div class="flex-1 min-w-0">
          <h3 class="text-gray-100 font-medium truncate" title={file.filename}>
            {file.filename}
          </h3>
          <div class="flex items-center gap-3 mt-1 text-sm text-gray-400">
            <span>{formatFileSize(file.size_bytes)}</span>
            <span>•</span>
            <span>{getFileExtension(file.filename)}</span>
            <span>•</span>
            <span>{formatDate(file.created_at)}</span>
          </div>
        </div>
      </div>

      <div class="flex items-center gap-2 mt-3">
        {#if onView}
          <button
            onclick={() => onView?.(file)}
            class="px-3 py-1.5 bg-blue-600 hover:bg-blue-700 text-white text-sm rounded transition-colors"
          >
            View
          </button>
        {/if}

        {#if onDownload}
          <button
            onclick={() => onDownload?.(file)}
            class="px-3 py-1.5 bg-gray-800 hover:bg-gray-700 text-gray-300 text-sm rounded transition-colors"
          >
            Download
          </button>
        {/if}

        {#if onDelete}
          <button
            onclick={() => onDelete?.(file)}
            class="px-3 py-1.5 bg-red-900/20 hover:bg-red-900/30 text-red-400 text-sm rounded transition-colors ml-auto"
          >
            Delete
          </button>
        {/if}
      </div>
    </div>
  </div>
</div>
