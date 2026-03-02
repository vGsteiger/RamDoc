<script lang="ts">
  export let content: string = '';
  export let isStreaming: boolean = false;
</script>

<div class="bg-gray-900 border border-gray-700 rounded-lg p-6 min-h-[300px] relative">
  {#if isStreaming}
    <div class="absolute top-4 right-4">
      <div class="flex items-center space-x-2 text-sm text-blue-400">
        <div class="animate-pulse">●</div>
        <span>Generating...</span>
      </div>
    </div>
  {/if}

  <div class="prose prose-invert max-w-none">
    {#if content}
      <pre class="whitespace-pre-wrap font-sans text-gray-100">{content}</pre>
    {:else if !isStreaming}
      <p class="text-gray-500 italic">Report will appear here as it's generated...</p>
    {:else}
      <div class="flex items-center space-x-2 text-gray-500">
        <div class="animate-spin rounded-full h-4 w-4 border-b-2 border-blue-500" />
        <span>Waiting for LLM...</span>
      </div>
    {/if}
  </div>
</div>
