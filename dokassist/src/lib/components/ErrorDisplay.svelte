<script lang="ts">
  import type { AppError } from '$lib/api';
  import { getUserFriendlyMessage } from '$lib/api';

  interface Props {
    error: AppError | null;
    showDetails?: boolean;
  }

  let { error, showDetails = false }: Props = $props();

  let expanded = $state(false);

  function copyErrorRef() {
    if (error?.ref) {
      navigator.clipboard.writeText(error.ref);
    }
  }

  function copyFullError() {
    if (error) {
      const text = `Error Code: ${error.code}\nError Reference: ${error.ref}\nMessage: ${error.message}`;
      navigator.clipboard.writeText(text);
    }
  }
</script>

{#if error}
  <div class="bg-red-900/20 border border-red-500 rounded-lg p-4">
    <div class="flex items-start justify-between mb-2">
      <div class="flex-1">
        <p class="text-red-400 text-sm font-medium mb-1">
          {getUserFriendlyMessage(error)}
        </p>
        <div class="flex items-center gap-2 text-xs text-gray-400">
          <span class="font-mono">{error.ref}</span>
          <button
            onclick={copyErrorRef}
            class="text-blue-400 hover:text-blue-300 underline"
            title="Copy error reference"
          >
            Copy
          </button>
        </div>
      </div>
      {#if showDetails}
        <button
          onclick={() => expanded = !expanded}
          class="text-gray-400 hover:text-gray-300 text-xs ml-4"
        >
          {expanded ? 'Hide Details' : 'Show Details'}
        </button>
      {/if}
    </div>

    {#if showDetails && expanded}
      <div class="mt-3 pt-3 border-t border-red-500/30">
        <div class="space-y-2 text-xs">
          <div>
            <span class="text-gray-500">Error Code:</span>
            <span class="ml-2 font-mono text-gray-300">{error.code}</span>
          </div>
          <div>
            <span class="text-gray-500">Technical Message:</span>
            <span class="ml-2 text-gray-300">{error.message}</span>
          </div>
          <div>
            <span class="text-gray-500">Reference ID:</span>
            <span class="ml-2 font-mono text-gray-300">{error.ref}</span>
          </div>
        </div>
        <button
          onclick={copyFullError}
          class="mt-3 text-xs text-blue-400 hover:text-blue-300 underline"
        >
          Copy Full Error Details
        </button>
      </div>
    {/if}

    <p class="text-xs text-gray-500 mt-2">
      Share the error reference with support if you need help resolving this issue.
    </p>
  </div>
{/if}
