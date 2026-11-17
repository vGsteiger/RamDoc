<script lang="ts">
  import { parseMarkedSnippet, segmentsFromRanges, findMatchRanges } from './highlight';

  /*
   * Renders a search snippet with highlighted matches. Never uses {@html}:
   * every segment is a text node, so a snippet containing `<script>` or
   * `<img onerror>` shows those characters literally.
   *
   * Give exactly one source of matches:
   *   - `snippet`: backend text with literal <mark>…</mark> tags (FTS5)
   *   - `text` + `ranges`: plain text and [start, end) character ranges
   *   - `text` + `query`: plain text, highlighting each query term
   */
  let {
    snippet = undefined,
    text = undefined,
    ranges = undefined,
    query = undefined,
    class: className = '',
  }: {
    snippet?: string | null;
    text?: string | null;
    ranges?: ReadonlyArray<readonly [number, number]>;
    query?: string;
    class?: string;
  } = $props();

  let segments = $derived(
    snippet !== undefined && snippet !== null
      ? parseMarkedSnippet(snippet)
      : segmentsFromRanges(text, ranges ?? (query ? findMatchRanges(text ?? '', query) : []))
  );
</script>

<span class={className}
  >{#each segments as segment, i (i)}{#if segment.match}<mark class="bg-highlight px-px text-fg"
        >{segment.text}</mark
      >{:else}{segment.text}{/if}{/each}</span
>
