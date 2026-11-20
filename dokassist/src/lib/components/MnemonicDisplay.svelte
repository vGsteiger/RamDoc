<script lang="ts">
  interface Props {
    words: string[];
  }

  let { words }: Props = $props();
</script>

<!-- Numbered in reading order (row by row), the order the phrase is typed
     back in on /recover. -->
<ol class="grid grid-cols-2 gap-2 sm:grid-cols-3 md:grid-cols-4">
  {#each words as word, i (i)}
    <li
      class="flex min-w-0 items-center gap-2 rounded-control border border-line bg-surface-raised px-2.5 py-2"
    >
      <span class="w-5 shrink-0 text-right text-caption text-fg-subtle" data-numeric>{i + 1}</span>
      <span class="truncate font-mono text-body text-fg">{word}</span>
    </li>
  {/each}
</ol>
