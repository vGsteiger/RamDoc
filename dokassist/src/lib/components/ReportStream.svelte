<script lang="ts">
  export let content: string = '';
  export let isStreaming: boolean = false;

  const THINK_START = '<think>';
  const THINK_END = '</think>';

  let thinkContent = '';
  let reportContent = '';

  $: {
    if (content.startsWith(THINK_START)) {
      const endIdx = content.indexOf(THINK_END);
      if (endIdx !== -1) {
        thinkContent = content.slice(THINK_START.length, endIdx).trim();
        reportContent = content.slice(endIdx + THINK_END.length).trim();
      } else {
        // Still streaming inside the think block
        thinkContent = content.slice(THINK_START.length).trim();
        reportContent = '';
      }
    } else {
      thinkContent = '';
      reportContent = content;
    }
  }
</script>

<div class="space-y-4">
  {#if thinkContent}
    <div class="bg-gray-800/50 border border-gray-700 rounded-lg p-4">
      <p class="text-xs font-medium text-gray-500 uppercase tracking-wide mb-2">Thinking</p>
      <pre class="whitespace-pre-wrap font-sans text-sm text-gray-400 italic">{thinkContent}</pre>
      {#if isStreaming && !reportContent}
        <div class="flex items-center space-x-2 text-gray-500 mt-2">
          <div class="animate-pulse text-xs">●</div>
          <span class="text-xs">Thinking...</span>
        </div>
      {/if}
    </div>
  {/if}

  <div class="bg-gray-900 border border-gray-700 rounded-lg p-6 min-h-[300px] relative">
    {#if isStreaming && reportContent}
      <div class="absolute top-4 right-4">
        <div class="flex items-center space-x-2 text-sm text-blue-400">
          <div class="animate-pulse">●</div>
          <span>Generating...</span>
        </div>
      </div>
    {/if}

    <div class="prose prose-invert max-w-none">
      {#if reportContent}
        <pre class="whitespace-pre-wrap font-sans text-gray-100">{reportContent}</pre>
      {:else if !isStreaming && !thinkContent}
        <p class="text-gray-500 italic">Report will appear here as it's generated...</p>
      {:else if isStreaming && !thinkContent && !reportContent}
        <div class="flex items-center space-x-2 text-gray-500">
          <div class="animate-spin rounded-full h-4 w-4 border-b-2 border-blue-500" />
          <span>Waiting for LLM...</span>
        </div>
      {:else if !isStreaming && thinkContent && !reportContent}
        <p class="text-gray-500 italic">No report content was generated.</p>
      {/if}
    </div>
  </div>
</div>
