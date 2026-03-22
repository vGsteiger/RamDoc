<script lang="ts">
  import type { ChatMessageRow } from '$lib/api';
  import { Zap, Check } from 'lucide-svelte';

  // Internal fields to hide from tool result display
  const HIDDEN_FIELDS = new Set(['id', 'patient_id', 'session_id', 'created_at', 'updated_at', 'vault_path', 'extracted_text', 'metadata_json', 'prompt_hash', 'amdp_data']);

  function toLabel(key: string): string {
    return key.replace(/_/g, ' ').replace(/\b\w/g, (c) => c.toUpperCase());
  }

  function formatValue(val: unknown): string {
    if (val === null || val === undefined) return '—';
    if (typeof val === 'boolean') return val ? 'Yes' : 'No';
    if (typeof val === 'string' && val.match(/^\d{4}-\d{2}-\d{2}/)) {
      return val.slice(0, 10).split('-').reverse().join('.');
    }
    return String(val);
  }

  type JsonObject = Record<string, unknown>;

  function parseToolResult(content: string): JsonObject | JsonObject[] | null {
    try {
      const parsed: unknown = JSON.parse(content);
      if (Array.isArray(parsed) || (typeof parsed === 'object' && parsed !== null)) {
        return parsed as JsonObject | JsonObject[];
      }
    } catch {
      // not JSON
    }
    return null;
  }

  function visibleEntries(obj: JsonObject): [string, unknown][] {
    return Object.entries(obj).filter(([k]) => !HIDDEN_FIELDS.has(k));
  }

  interface Props {
    message: ChatMessageRow;
    isStreaming?: boolean;
  }

  let { message, isStreaming = false }: Props = $props();

  const THINK_START = '<think>';
  const THINK_END = '</think>';

  let thinkContent = $derived(() => {
    if (!message.content.startsWith(THINK_START)) return '';
    const end = message.content.indexOf(THINK_END);
    return end !== -1
      ? message.content.slice(THINK_START.length, end).trim()
      : message.content.slice(THINK_START.length).trim();
  });

  let mainContent = $derived(() => {
    if (!message.content.startsWith(THINK_START)) return message.content;
    const end = message.content.indexOf(THINK_END);
    return end !== -1 ? message.content.slice(end + THINK_END.length).trim() : '';
  });

  let toolCallCollapsed = $state(true);
  let toolResultCollapsed = $state(true);
</script>

{#if message.role === 'user'}
  <div class="flex justify-end mb-3">
    <div
      class="max-w-[75%] bg-blue-600 text-white rounded-2xl rounded-br-sm px-4 py-2 text-sm whitespace-pre-wrap"
    >
      {message.content}
    </div>
  </div>
{:else if message.role === 'assistant'}
  <div class="flex justify-start mb-3">
    <div class="max-w-[80%] space-y-2">
      {#if thinkContent()}
        <div
          class="bg-gray-100 dark:bg-gray-800/50 border border-gray-200 dark:border-gray-700 rounded-lg px-3 py-2"
        >
          <p class="text-xs text-gray-400 dark:text-gray-500 uppercase tracking-wide mb-1">
            Thinking
          </p>
          <pre
            class="whitespace-pre-wrap font-sans text-xs text-gray-500 dark:text-gray-400 italic">{thinkContent()}</pre>
        </div>
      {/if}
      <div
        class="bg-gray-100 dark:bg-gray-800 border border-gray-200 dark:border-gray-700 rounded-2xl rounded-bl-sm px-4 py-2 text-sm text-gray-900 dark:text-gray-100 whitespace-pre-wrap"
      >
        {#if mainContent()}
          {mainContent()}
        {:else if isStreaming}
          <span class="animate-pulse text-gray-500">●</span>
        {/if}
      </div>
    </div>
  </div>
{:else if message.role === 'tool_call'}
  <div class="flex justify-start mb-2">
    <div class="max-w-[80%]">
      <button
        onclick={() => (toolCallCollapsed = !toolCallCollapsed)}
        aria-label={toolCallCollapsed ? 'Show tool call details' : 'Hide tool call details'}
        class="flex items-center gap-2 text-xs text-gray-500 hover:text-gray-400 transition-colors"
      >
        <Zap size={14} class="text-amber-500" />
        <span>Tool: {message.tool_name ?? 'unknown'}</span>
        <span>{toolCallCollapsed ? '▶' : '▼'}</span>
      </button>
      {#if !toolCallCollapsed}
        <div
          class="mt-1 bg-gray-100 dark:bg-gray-800/40 border border-gray-200 dark:border-gray-700 rounded-lg px-3 py-2"
        >
          <pre
            class="text-xs text-gray-500 dark:text-gray-400 whitespace-pre-wrap overflow-x-auto">{message.tool_args_json ??
              message.content}</pre>
        </div>
      {/if}
    </div>
  </div>
{:else if message.role === 'tool_result'}
  {@const parsed = parseToolResult(message.content)}
  <div class="flex justify-start mb-3">
    <div class="max-w-[80%]">
      <button
        onclick={() => (toolResultCollapsed = !toolResultCollapsed)}
        aria-label={toolResultCollapsed ? 'Ergebnis anzeigen' : 'Ergebnis ausblenden'}
        class="flex items-center gap-2 text-xs text-gray-500 hover:text-gray-400 transition-colors"
      >
        <Check size={14} class="text-green-500" />
        <span>Ergebnis</span>
        <span>{toolResultCollapsed ? '▶' : '▼'}</span>
      </button>
      {#if !toolResultCollapsed}
        <div
          class="mt-1 bg-green-50 dark:bg-green-900/20 border border-green-200 dark:border-green-800/50 rounded-lg px-3 py-2"
        >
          {#if parsed !== null}
            {#if Array.isArray(parsed)}
              <div class="space-y-2">
                {#each parsed as item, i}
                  <div class="text-xs text-gray-700 dark:text-gray-300 border-b border-green-200 dark:border-green-800/30 pb-1 last:border-0 last:pb-0">
                    <span class="text-[10px] text-gray-400 uppercase">{i + 1}</span>
                    {#each visibleEntries(item) as [key, val]}
                      <div class="flex gap-2">
                        <span class="text-gray-500 dark:text-gray-400 shrink-0">{toLabel(key)}:</span>
                        <span class="text-gray-800 dark:text-gray-200">{formatValue(val)}</span>
                      </div>
                    {/each}
                  </div>
                {/each}
              </div>
            {:else}
              <div class="space-y-1">
                {#each visibleEntries(parsed) as [key, val]}
                  <div class="flex gap-2 text-xs">
                    <span class="text-gray-500 dark:text-gray-400 shrink-0">{toLabel(key)}:</span>
                    <span class="text-gray-800 dark:text-gray-200">{formatValue(val)}</span>
                  </div>
                {/each}
              </div>
            {/if}
          {:else}
            <pre class="text-xs text-gray-500 dark:text-gray-400 whitespace-pre-wrap overflow-x-auto">{message.content}</pre>
          {/if}
        </div>
      {/if}
    </div>
  </div>
{/if}
