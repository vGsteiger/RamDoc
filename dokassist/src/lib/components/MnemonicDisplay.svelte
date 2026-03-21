<script lang="ts">
  interface Props {
    words: string[];
  }

  let { words }: Props = $props();
</script>

<div class="grid grid-cols-4 gap-3 max-w-2xl">
  {#each words as word, i}
    <div
      class="flex items-center gap-2 p-3 bg-white dark:bg-gray-800 rounded-lg border border-gray-200 dark:border-gray-700"
    >
      <span class="text-gray-400 text-sm font-mono w-6">{i + 1}.</span>
      <span class="text-gray-900 dark:text-gray-100 font-medium">{word}</span>
    </div>
  {/each}
</div>
