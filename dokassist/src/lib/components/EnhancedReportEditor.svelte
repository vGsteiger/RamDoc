<script lang="ts">
  import { onDestroy } from 'svelte';
  import { listen, type UnlistenFn } from '@tauri-apps/api/event';
  import { invoke } from '@tauri-apps/api/core';
  import { getEngineStatus, parseError, type LlmEngineStatus, type AppError } from '$lib/api';
  import { get } from 'svelte/store';
  import { t } from '$lib/translations';

  export let content: string = '';
  export let readonly: boolean = false;

  let showPreview = false;
  let showSuggestions = false;
  let selectedText = '';
  let suggestionInstruction = get(t)('reports.editor.defaultInstruction');
  let generatedSuggestion = '';
  let isGeneratingSuggestion = false;
  let error: AppError | null = null;
  let llmStatus: LlmEngineStatus | null = null;

  let unlistenChunk: UnlistenFn | null = null;
  let unlistenDone: UnlistenFn | null = null;

  async function checkLlmStatus() {
    try {
      llmStatus = await getEngineStatus();
    } catch (e) {
      error = parseError(e);
    }
  }

  function handleTextSelection() {
    const textarea = document.getElementById('report-textarea') as HTMLTextAreaElement;
    if (textarea) {
      const start = textarea.selectionStart;
      const end = textarea.selectionEnd;
      selectedText = content.substring(start, end);
    }
  }

  async function generateSuggestion() {
    if (!selectedText && !content) {
      error = {
        code: 'VALIDATION_ERROR',
        message: get(t)('reports.editor.noTextSelected'),
        ref: 'NO_TEXT',
      };
      return;
    }

    if (!llmStatus?.is_loaded) {
      error = {
        code: 'LLM_ERROR',
        message: get(t)('reports.editor.modelNotLoaded'),
        ref: 'LLM_NOT_LOADED',
      };
      return;
    }

    try {
      // Unlisten from previous listeners if they exist
      if (unlistenChunk) {
        unlistenChunk();
        unlistenChunk = null;
      }
      if (unlistenDone) {
        unlistenDone();
        unlistenDone = null;
      }

      isGeneratingSuggestion = true;
      error = null;
      generatedSuggestion = '';
      showSuggestions = true;

      // Set up event listeners for streaming
      unlistenChunk = await listen<string>('text-improvement-chunk', (event) => {
        generatedSuggestion += event.payload;
      });

      unlistenDone = await listen('text-improvement-done', () => {
        isGeneratingSuggestion = false;
        // Unlisten after completion
        if (unlistenChunk) {
          unlistenChunk();
          unlistenChunk = null;
        }
        if (unlistenDone) {
          unlistenDone();
          unlistenDone = null;
        }
      });

      const textToImprove = selectedText || content;
      await invoke('improve_text', {
        text: textToImprove,
        instruction: suggestionInstruction,
        systemPrompt: null,
      });
    } catch (e) {
      error = parseError(e);
      isGeneratingSuggestion = false;
      // Unlisten on error
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

  function applySuggestion() {
    if (!generatedSuggestion) return;

    if (selectedText) {
      // Replace selected text with suggestion
      content = content.replace(selectedText, generatedSuggestion);
    } else {
      // Replace entire content with suggestion
      content = generatedSuggestion;
    }

    // Clear selection and suggestion
    selectedText = '';
    generatedSuggestion = '';
    showSuggestions = false;
  }

  function clearSuggestion() {
    generatedSuggestion = '';
    selectedText = '';
    showSuggestions = false;
  }

  // Check LLM status on mount
  checkLlmStatus();

  onDestroy(() => {
    if (unlistenChunk) unlistenChunk();
    if (unlistenDone) unlistenDone();
  });
</script>

<div class="flex h-full gap-4">
  <!-- Main editor panel -->
  <div
    class="flex-1 flex flex-col border border-gray-200 dark:border-gray-700 rounded-lg overflow-hidden"
  >
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
            <pre
              class="whitespace-pre-wrap font-sans text-gray-900 dark:text-gray-100">{content}</pre>
          {:else}
            <p class="text-gray-400 dark:text-gray-500 italic">Kein Inhalt zur Vorschau</p>
          {/if}
        </div>
      {:else}
        <textarea
          id="report-textarea"
          bind:value={content}
          on:select={handleTextSelection}
          on:mouseup={handleTextSelection}
          {readonly}
          class="w-full h-full p-6 bg-white dark:bg-gray-900 text-gray-900 dark:text-gray-100 font-mono text-sm resize-none focus:outline-none"
          placeholder="Berichtinhalt wird hier angezeigt..."
        ></textarea>
      {/if}
    </div>

    {#if !readonly && !showPreview}
      <div class="border-t border-gray-200 dark:border-gray-700 bg-gray-100 dark:bg-gray-800 p-4">
        <div class="flex items-center gap-4">
          <button
            on:click={() => (showSuggestions = !showSuggestions)}
            class="px-4 py-2 bg-blue-600 text-white rounded hover:bg-blue-700 transition-colors text-sm"
            disabled={!llmStatus?.is_loaded}
          >
            {showSuggestions ? $t('common.close') : $t('reports.editor.suggestions')}
          </button>
          {#if selectedText}
            <span class="text-xs text-gray-500 dark:text-gray-400">
              {$t('reports.editor.charsSelected').replace('{count}', String(selectedText.length))}
            </span>
          {/if}
        </div>
      </div>
    {/if}
  </div>

  <!-- Suggestions panel -->
  {#if showSuggestions && !readonly}
    <div
      class="w-1/3 flex flex-col border border-gray-200 dark:border-gray-700 rounded-lg overflow-hidden bg-gray-50 dark:bg-gray-800"
    >
      <div class="p-4 border-b border-gray-200 dark:border-gray-700">
        <h3 class="text-sm font-semibold text-gray-900 dark:text-gray-100 mb-2">
          {$t('reports.editor.suggestions')}
        </h3>
        {#if error}
          <div
            class="p-2 bg-red-50 dark:bg-red-900/20 border border-red-300 dark:border-red-500 rounded text-xs text-red-600 dark:text-red-400 mb-2"
          >
            {error.message}
          </div>
        {/if}
        {#if !llmStatus?.is_loaded}
          <div
            class="p-2 bg-yellow-50 dark:bg-yellow-900/20 border border-yellow-300 dark:border-yellow-500 rounded text-xs text-yellow-700 dark:text-yellow-400"
          >
            {$t('reports.editor.modelNotLoaded')}
          </div>
        {:else}
          <textarea
            bind:value={suggestionInstruction}
            class="w-full h-16 px-3 py-2 bg-white dark:bg-gray-900 border border-gray-300 dark:border-gray-700 rounded text-gray-900 dark:text-gray-100 text-xs focus:outline-none focus:border-blue-500"
            placeholder={$t('reports.editor.instructionPlaceholder')}
          ></textarea>
          <button
            on:click={generateSuggestion}
            disabled={isGeneratingSuggestion}
            class="w-full mt-2 px-3 py-2 bg-green-600 text-white rounded hover:bg-green-700 transition-colors text-sm disabled:opacity-50"
          >
            {isGeneratingSuggestion
              ? $t('reports.editor.generating')
              : $t('reports.editor.generateSuggestion')}
          </button>
        {/if}
      </div>

      <div class="flex-1 overflow-auto p-4">
        {#if isGeneratingSuggestion}
          <div class="text-sm text-gray-500 dark:text-gray-400">
            <div class="flex items-center gap-2">
              <div class="animate-pulse h-2 w-2 bg-blue-500 rounded-full"></div>
              <span>{$t('reports.editor.generatingSuggestion')}</span>
            </div>
            {#if generatedSuggestion}
              <pre
                class="mt-4 whitespace-pre-wrap font-sans text-gray-900 dark:text-gray-100 text-sm">{generatedSuggestion}</pre>
            {/if}
          </div>
        {:else if generatedSuggestion}
          <div>
            <pre
              class="whitespace-pre-wrap font-sans text-gray-900 dark:text-gray-100 text-sm">{generatedSuggestion}</pre>
            <div class="flex gap-2 mt-4">
              <button
                on:click={applySuggestion}
                class="flex-1 px-3 py-2 bg-blue-600 text-white rounded hover:bg-blue-700 transition-colors text-sm"
              >
                {$t('reports.editor.apply')}
              </button>
              <button
                on:click={clearSuggestion}
                class="flex-1 px-3 py-2 bg-gray-200 dark:bg-gray-700 text-gray-700 dark:text-gray-300 rounded hover:bg-gray-300 dark:hover:bg-gray-600 transition-colors text-sm"
              >
                {$t('reports.editor.discard')}
              </button>
            </div>
          </div>
        {:else}
          <p class="text-sm text-gray-400 dark:text-gray-500 italic">
            {$t('reports.editor.suggestionHint')}
          </p>
        {/if}
      </div>
    </div>
  {/if}
</div>
