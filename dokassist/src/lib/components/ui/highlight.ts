/**
 * Search-snippet highlighting without HTML injection.
 *
 * Snippets are split into plain text segments; the component renders each as
 * a text node (or a <mark> element around a text node), so nothing in the
 * snippet is ever parsed as markup. See HighlightedSnippet.svelte.
 */
export interface SnippetSegment {
  text: string;
  match: boolean;
}

/**
 * Parses a backend snippet in which matches are wrapped in literal
 * `<mark>…</mark>` (SQLite FTS5 `snippet()` / `highlight()`). Only the exact
 * tags `<mark>` and `</mark>` are recognised (case-insensitive); every other
 * character, including any other angle-bracket text, stays literal text.
 * Unbalanced tags degrade gracefully: an unclosed `<mark>` highlights to the
 * end, a stray `</mark>` is dropped.
 */
export function parseMarkedSnippet(snippet: string | null | undefined): SnippetSegment[] {
  if (!snippet) return [];
  const segments: SnippetSegment[] = [];
  const re = /<(\/?)mark>/gi;
  let inMark = false;
  let last = 0;
  for (const m of snippet.matchAll(re)) {
    const index = m.index ?? 0;
    push(segments, snippet.slice(last, index), inMark);
    inMark = m[1] !== '/';
    last = index + m[0].length;
  }
  push(segments, snippet.slice(last), inMark);
  return segments;
}

/**
 * Builds segments from plain text plus `[start, end)` character ranges
 * (e.g. client-side matches). Ranges may overlap or be unsorted; they are
 * clamped to the text and merged.
 */
export function segmentsFromRanges(
  text: string | null | undefined,
  ranges: ReadonlyArray<readonly [number, number]> = []
): SnippetSegment[] {
  if (!text) return [];
  const merged = ranges
    .map(([s, e]) => [Math.max(0, Math.min(s, e)), Math.min(text.length, Math.max(s, e))] as const)
    .filter(([s, e]) => e > s)
    .sort((a, b) => a[0] - b[0])
    .reduce<[number, number][]>((acc, [s, e]) => {
      const prev = acc[acc.length - 1];
      if (prev && s <= prev[1]) prev[1] = Math.max(prev[1], e);
      else acc.push([s, e]);
      return acc;
    }, []);
  const segments: SnippetSegment[] = [];
  let last = 0;
  for (const [s, e] of merged) {
    push(segments, text.slice(last, s), false);
    push(segments, text.slice(s, e), true);
    last = e;
  }
  push(segments, text.slice(last), false);
  return segments;
}

/**
 * Case-insensitive ranges of every occurrence of each whitespace-separated
 * term in `query`, for highlighting client-side filtered lists.
 */
export function findMatchRanges(text: string, query: string): [number, number][] {
  const terms = query
    .toLocaleLowerCase()
    .split(/\s+/)
    .filter((term) => term.length > 0);
  if (!text || terms.length === 0) return [];
  const haystack = text.toLocaleLowerCase();
  const ranges: [number, number][] = [];
  for (const term of terms) {
    let from = 0;
    for (;;) {
      const at = haystack.indexOf(term, from);
      if (at === -1) break;
      ranges.push([at, at + term.length]);
      from = at + term.length;
    }
  }
  return ranges;
}

function push(segments: SnippetSegment[], text: string, match: boolean) {
  if (!text) return;
  const prev = segments[segments.length - 1];
  if (prev && prev.match === match) prev.text += text;
  else segments.push({ text, match });
}
