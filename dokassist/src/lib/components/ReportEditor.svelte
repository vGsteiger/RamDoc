<script lang="ts">
  export let content: string = '';
  export let readonly: boolean = false;

  let showPreview = false;
</script>

<div class="flex flex-col h-full border border-gray-200 dark:border-gray-700 rounded-lg overflow-hidden">
  <div class="flex border-b border-gray-200 dark:border-gray-700 bg-gray-100 dark:bg-gray-800">
    <button
      on:click={() => (showPreview = false)}
      class="flex-1 px-4 py-2 text-sm font-medium transition-colors {!showPreview
        ? 'bg-white dark:bg-gray-900 text-gray-900 dark:text-gray-100'
        : 'text-gray-500 dark:text-gray-400 hover:text-gray-700 dark:hover:text-gray-300'}"
    >
      Edit
    </button>
    <button
      on:click={() => (showPreview = true)}
      class="flex-1 px-4 py-2 text-sm font-medium transition-colors {showPreview
        ? 'bg-white dark:bg-gray-900 text-gray-900 dark:text-gray-100'
        : 'text-gray-500 dark:text-gray-400 hover:text-gray-700 dark:hover:text-gray-300'}"
    >
      Preview
    </button>
  </div>

  <div class="flex-1 overflow-auto">
    {#if showPreview}
      <div class="p-6 prose dark:prose-invert max-w-none">
        {#if content}
          <pre class="whitespace-pre-wrap font-sans text-gray-900 dark:text-gray-100">{content}</pre>
        {:else}
          <p class="text-gray-400 dark:text-gray-500 italic">No content to preview</p>
        {/if}
      </div>
    {:else}
      <textarea
        bind:value={content}
        {readonly}
        class="w-full h-full p-6 bg-white dark:bg-gray-900 text-gray-900 dark:text-gray-100 font-mono text-sm resize-none focus:outline-none"
        placeholder="Report content will appear here..."
      ></textarea>
    {/if}
  </div>
</div>
