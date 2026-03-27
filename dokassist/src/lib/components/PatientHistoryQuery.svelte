<script lang="ts">
  import { onDestroy } from 'svelte';
  import { listen, type UnlistenFn } from '@tauri-apps/api/event';
  import { queryPatientHistory } from '$lib/api';
  import { Loader2, Send, ChevronDown, ChevronUp } from 'lucide-svelte';

  interface Props {
    patientId: string;
  }

  let { patientId }: Props = $props();

  let question = $state('');
  let response = $state('');
  let isQuerying = $state(false);
  let error = $state('');
  let isExpanded = $state(true);

  let unlistenChunk: UnlistenFn | null = null;
  let unlistenDone: UnlistenFn | null = null;

  // Suggested queries
  const suggestedQueries = [
    'Wann wurde die Medikation zuletzt geändert?',
    'Was war der PHQ-9-Trend im letzten Quartal?',
    'Wie hat sich die Stimmung in den letzten Sitzungen entwickelt?',
    'Welche Behandlungsziele wurden erreicht?',
    'Gab es Nebenwirkungen bei der aktuellen Medikation?',
  ];

  async function handleQuery() {
    if (!question.trim() || isQuerying) return;

    try {
      isQuerying = true;
      error = '';
      response = '';

      // Setup event listeners before invoking
      if (unlistenChunk) {
        unlistenChunk();
        unlistenChunk = null;
      }
      if (unlistenDone) {
        unlistenDone();
        unlistenDone = null;
      }

      unlistenChunk = await listen<string>('patient-history-chunk', (event) => {
        response += event.payload;
      });

      unlistenDone = await listen('patient-history-done', () => {
        isQuerying = false;
        // Clean up listeners
        if (unlistenChunk) {
          unlistenChunk();
          unlistenChunk = null;
        }
        if (unlistenDone) {
          unlistenDone();
          unlistenDone = null;
        }
      });

      // Invoke the command
      await queryPatientHistory(patientId, question);
    } catch (e) {
      error = e instanceof Error ? e.message : 'Failed to query patient history';
      console.error('Error querying patient history:', e);
      isQuerying = false;
      // Clean up listeners on error
      if (unlistenChunk) {
        unlistenChunk();
        unlistenChunk = null;
      }
      if (unlistenDone) {
        unlistenDone();
        unlistenDone = null;
      }
    }
  }

  function handleSuggestedQuery(suggestedQuestion: string) {
    question = suggestedQuestion;
    handleQuery();
  }

  function handleKeydown(event: KeyboardEvent) {
    if (event.key === 'Enter' && !event.shiftKey) {
      event.preventDefault();
      handleQuery();
    }
  }

  onDestroy(() => {
    if (unlistenChunk) {
      unlistenChunk();
    }
    if (unlistenDone) {
      unlistenDone();
    }
  });
</script>

<div class="bg-white dark:bg-gray-800 rounded-lg border border-gray-200 dark:border-gray-700">
  <button
    onclick={() => (isExpanded = !isExpanded)}
    class="flex items-center justify-between w-full p-4 hover:bg-gray-50 dark:hover:bg-gray-700/50 transition-colors rounded-t-lg"
  >
    <h3 class="text-lg font-semibold text-gray-900 dark:text-gray-100">
      Ask about this patient
    </h3>
    {#if isExpanded}
      <ChevronUp class="w-5 h-5 text-gray-500 dark:text-gray-400" />
    {:else}
      <ChevronDown class="w-5 h-5 text-gray-500 dark:text-gray-400" />
    {/if}
  </button>

  {#if isExpanded}
    <div class="p-4 border-t border-gray-200 dark:border-gray-700 space-y-4">
      <!-- Suggested queries -->
      <div class="space-y-2">
        <p class="text-sm text-gray-600 dark:text-gray-400">Suggested queries:</p>
        <div class="flex flex-wrap gap-2">
          {#each suggestedQueries as suggested}
            <button
              onclick={() => handleSuggestedQuery(suggested)}
              disabled={isQuerying}
              class="px-3 py-1.5 text-sm bg-gray-100 dark:bg-gray-700 text-gray-700 dark:text-gray-300 rounded-full hover:bg-gray-200 dark:hover:bg-gray-600 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
            >
              {suggested}
            </button>
          {/each}
        </div>
      </div>

      <!-- Query input -->
      <div class="flex gap-2">
        <textarea
          bind:value={question}
          onkeydown={handleKeydown}
          disabled={isQuerying}
          placeholder="Ask a question about this patient's history..."
          class="flex-1 px-4 py-2 border border-gray-300 dark:border-gray-600 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-transparent bg-white dark:bg-gray-700 text-gray-900 dark:text-gray-100 placeholder-gray-500 dark:placeholder-gray-400 disabled:opacity-50 disabled:cursor-not-allowed resize-none"
          rows="2"
        ></textarea>
        <button
          onclick={handleQuery}
          disabled={isQuerying || !question.trim()}
          class="px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 transition-colors disabled:opacity-50 disabled:cursor-not-allowed flex items-center gap-2 self-start"
        >
          {#if isQuerying}
            <Loader2 class="w-4 h-4 animate-spin" />
            <span>Querying...</span>
          {:else}
            <Send class="w-4 h-4" />
            <span>Ask</span>
          {/if}
        </button>
      </div>

      <!-- Error display -->
      {#if error}
        <div
          class="bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 rounded-lg p-4 text-red-600 dark:text-red-400"
        >
          {error}
        </div>
      {/if}

      <!-- Response display -->
      {#if response || isQuerying}
        <div class="bg-gray-50 dark:bg-gray-900/50 rounded-lg p-4 border border-gray-200 dark:border-gray-700">
          <div class="flex items-center justify-between mb-2">
            <h4 class="text-sm font-semibold text-gray-700 dark:text-gray-300">Response:</h4>
            {#if isQuerying}
              <div class="flex items-center gap-2 text-sm text-gray-600 dark:text-gray-400">
                <Loader2 class="w-4 h-4 animate-spin" />
                <span>Generating...</span>
              </div>
            {/if}
          </div>
          <div class="prose prose-sm dark:prose-invert max-w-none">
            {#if response}
              <div class="whitespace-pre-wrap text-gray-900 dark:text-gray-100">
                {response}
              </div>
            {:else}
              <div class="text-gray-500 dark:text-gray-400 italic">Waiting for response...</div>
            {/if}
          </div>
        </div>
      {/if}
    </div>
  {/if}
</div>
