<script lang="ts">
  import { parseMarkdown, type MdBlock, type MdInline } from './markdown';

  /*
   * Safe markdown display for chat replies, reports and release notes.
   * No {@html}: the source is tokenised by `marked` and mapped to a closed set
   * of elements (see markdown.ts). Raw HTML shows as text, only http(s) and
   * mailto links are clickable, and images never load (alt text is shown).
   *
   * Headings are shifted by `headingOffset` (default 2: `#` renders as h3)
   * so model output never competes with the page's own h1/h2.
   */
  let {
    source,
    breaks = true,
    headingOffset = 2,
    class: className = '',
  }: {
    source: string | null | undefined;
    /** Single newlines become line breaks (chat-style). */
    breaks?: boolean;
    headingOffset?: number;
    class?: string;
  } = $props();

  let tree = $derived(parseMarkdown(source, breaks));

  function headingTag(depth: number) {
    return `h${Math.min(6, Math.max(1, depth + headingOffset))}`;
  }

  const headingClass: Record<number, string> = {
    1: 'text-title text-fg',
    2: 'text-heading text-fg',
    3: 'text-body font-semibold text-fg',
  };

  const alignClass = { left: 'text-left', center: 'text-center', right: 'text-right' };
</script>

{#snippet inlines(nodes: MdInline[])}
  {#each nodes as node, i (i)}
    {#if node.t === 'text'}{node.text}{:else if node.t === 'strong'}<strong class="font-semibold"
        >{@render inlines(node.children)}</strong
      >{:else if node.t === 'em'}<em>{@render inlines(node.children)}</em
      >{:else if node.t === 'del'}<del>{@render inlines(node.children)}</del
      >{:else if node.t === 'code'}<code
        class="rounded-control bg-surface-sunken px-1 py-px font-mono text-[0.9em]"
        >{node.text}</code
      >{:else if node.t === 'br'}<br />{:else if node.t === 'link'}{#if node.href}<a
          href={node.href}
          target="_blank"
          rel="noopener noreferrer"
          class="text-link underline hover:text-link-hover">{@render inlines(node.children)}</a
        >{:else}{@render inlines(node.children)}{/if}{/if}
  {/each}
{/snippet}

{#snippet blocks(nodes: MdBlock[])}
  {#each nodes as node, i (i)}
    {#if node.t === 'p'}
      <p>{@render inlines(node.children)}</p>
    {:else if node.t === 'text'}
      <span class="block">{@render inlines(node.children)}</span>
    {:else if node.t === 'h'}
      <svelte:element
        this={headingTag(node.depth)}
        class="{headingClass[node.depth] ?? headingClass[3]} mt-4 first:mt-0"
      >
        {@render inlines(node.children)}
      </svelte:element>
    {:else if node.t === 'code'}
      <pre
        class="overflow-x-auto [contain:inline-size] rounded-control border border-line-subtle bg-surface-sunken p-3 font-mono text-label"><code
          >{node.text}</code
        ></pre>
    {:else if node.t === 'hr'}
      <hr class="border-line" />
    {:else if node.t === 'quote'}
      <blockquote class="space-y-2 border-l-2 border-line-strong pl-3 text-fg-muted">
        {@render blocks(node.children)}
      </blockquote>
    {:else if node.t === 'list'}
      {#if node.ordered}
        <ol class="list-decimal space-y-1 pl-5" start={node.start ?? undefined}>
          {@render items(node.items)}
        </ol>
      {:else}
        <ul class="list-disc space-y-1 pl-5">
          {@render items(node.items)}
        </ul>
      {/if}
    {:else if node.t === 'table'}
      <div class="overflow-x-auto [contain:inline-size]">
        <table class="w-full border-collapse text-label">
          <thead>
            <tr>
              {#each node.header as cell, c (c)}
                <th
                  class="border-b border-line px-2 py-1.5 font-semibold text-fg {alignClass[
                    node.align[c] ?? 'left'
                  ]}"
                >
                  {@render inlines(cell)}
                </th>
              {/each}
            </tr>
          </thead>
          <tbody>
            {#each node.rows as row, r (r)}
              <tr>
                {#each row as cell, c (c)}
                  <td
                    class="border-b border-line-subtle px-2 py-1.5 align-top {alignClass[
                      node.align[c] ?? 'left'
                    ]}"
                  >
                    {@render inlines(cell)}
                  </td>
                {/each}
              </tr>
            {/each}
          </tbody>
        </table>
      </div>
    {/if}
  {/each}
{/snippet}

{#snippet items(list: { checked: boolean | null; children: MdBlock[] }[])}
  {#each list as item, i (i)}
    <li class={item.checked !== null ? 'flex list-none items-start gap-2 -ml-5' : ''}>
      {#if item.checked !== null}
        <input
          type="checkbox"
          checked={item.checked}
          disabled
          class="mt-1 shrink-0"
          aria-hidden="true"
        />
        <div class="min-w-0 space-y-2">{@render blocks(item.children)}</div>
      {:else}
        <div class="space-y-2">{@render blocks(item.children)}</div>
      {/if}
    </li>
  {/each}
{/snippet}

<div class="min-w-0 space-y-3 text-body text-fg [overflow-wrap:anywhere] {className}">
  {@render blocks(tree)}
</div>
