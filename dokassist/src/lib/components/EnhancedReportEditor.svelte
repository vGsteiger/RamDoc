<script lang="ts">
  import { onDestroy } from 'svelte';
  import { listen, type UnlistenFn } from '@tauri-apps/api/event';
  import { invoke } from '@tauri-apps/api/core';
  import { getEngineStatus, parseError, type LlmEngineStatus, type AppError } from '$lib/api';
  import { get } from 'svelte/store';
  import { t } from '$lib/translations';
  import { thinkingEffort } from '$lib/stores/thinking';
  import { stripThinkTags } from '$lib/llm/strip-think';
  import { ThinkingIndicator } from '$lib/components/ui';

  export let content: string = '';
  export let readonly: boolean = false;

  let showPreview = false;
  let showSuggestions = false;
  let selectedText = '';
  let suggestionInstruction = get(t)('reports.editor.defaultInstruction');
  let generatedSuggestion = '';
  let isGeneratingSuggestion = false;
  let activityStartedAt: number | null = null;
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
      activityStartedAt = Date.now();
      error = null;
      generatedSuggestion = '';
      showSuggestions = true;

      // Set up event listeners for streaming
      unlistenChunk = await listen<string>('text-improvement-chunk', (event) => {
        generatedSuggestion += event.payload;
      });

      unlistenDone = await listen('text-improvement-done', () => {
        generatedSuggestion = stripThinkTags(generatedSuggestion);
        isGeneratingSuggestion = false;
        activityStartedAt = null;
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
        thinkingEffort: get(thinkingEffort),
      });
    } catch (e) {
      error = parseError(e);
      isGeneratingSuggestion = false;
      activityStartedAt = null;
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

    const suggestion = stripThinkTags(generatedSuggestion);
    if (selectedText) {
      content = content.replace(selectedText, suggestion);
    } else {
      content = suggestion;
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
  <div class="flex-1 flex flex-col border border-line rounded-card overflow-hidden">
    <div class="flex border-b border-line bg-surface-hover">
      <button
        on:click={() => (showPreview = false)}
        class="flex-1 h-8 px-3 text-body font-medium transition-colors {!showPreview
          ? 'bg-surface-raised text-fg'
          : 'text-fg-muted hover:text-fg'}"
      >
        {$t('reports.edit')}
      </button>
      <button
        on:click={() => (showPreview = true)}
        class="flex-1 h-8 px-3 text-body font-medium transition-colors {showPreview
          ? 'bg-surface-raised text-fg'
          : 'text-fg-muted hover:text-fg'}"
      >
        {$t('reports.preview')}
      </button>
    </div>

    <div class="flex-1 overflow-auto">
      {#if showPreview}
        <div class="p-6 prose dark:prose-invert max-w-none">
          {#if content}
            <pre class="whitespace-pre-wrap font-sans text-fg">{content}</pre>
          {:else}
            <p class="text-fg-subtle italic">{$t('reports.noPreviewContent')}</p>
          {/if}
        </div>
      {:else}
        <textarea
          id="report-textarea"
          bind:value={content}
          on:select={handleTextSelection}
          on:mouseup={handleTextSelection}
          {readonly}
          class="w-full h-full p-6 bg-surface-raised text-fg font-mono text-body resize-none focus:outline-none"
          placeholder={$t('reports.contentPlaceholder')}></textarea>
      {/if}
    </div>

    {#if !readonly && !showPreview}
      <div class="border-t border-line bg-surface-hover p-4">
        <div class="flex items-center gap-4">
          <button
            on:click={() => (showSuggestions = !showSuggestions)}
            class="h-8 px-3 bg-accent text-on-accent rounded-control hover:bg-accent-hover transition-colors text-body"
            disabled={!llmStatus?.is_loaded}
          >
            {showSuggestions ? $t('common.close') : $t('reports.editor.suggestions')}
          </button>
          {#if selectedText}
            <span class="text-caption text-fg-muted">
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
      class="w-1/3 flex flex-col border border-line rounded-card overflow-hidden bg-surface-sunken"
    >
      <div class="p-4 border-b border-line">
        <h3 class="text-body font-semibold text-fg mb-2">
          {$t('reports.editor.suggestions')}
        </h3>
        {#if error}
          <div
            class="p-2 bg-danger-subtle border border-danger-line rounded-card text-caption text-danger-fg mb-2"
          >
            {error.message}
          </div>
        {/if}
        {#if !llmStatus?.is_loaded}
          <div
            class="p-2 bg-warning-subtle border border-warning-line rounded-card text-caption text-warning-fg"
          >
            {$t('reports.editor.modelNotLoaded')}
          </div>
        {:else}
          <textarea
            bind:value={suggestionInstruction}
            class="w-full h-16 px-3 py-2 bg-surface-raised border border-line rounded-control text-fg text-caption focus:outline-none focus:border-accent"
            placeholder={$t('reports.editor.instructionPlaceholder')}></textarea>
          <button
            on:click={generateSuggestion}
            disabled={isGeneratingSuggestion}
            class="w-full mt-2 h-8 px-3 bg-success text-on-success rounded-control hover:bg-success-hover transition-colors text-body disabled:opacity-50"
          >
            {isGeneratingSuggestion
              ? $t('reports.editor.generating')
              : $t('reports.editor.generateSuggestion')}
          </button>
        {/if}
      </div>

      <div class="flex-1 overflow-auto p-4">
        {#if isGeneratingSuggestion}
          <div class="text-body text-fg-muted">
            {#if activityStartedAt}
              <ThinkingIndicator
                startedAt={activityStartedAt}
                label={$t('reports.editor.activityWriting')}
              />
            {/if}
            {#if generatedSuggestion}
              <pre
                class="mt-4 whitespace-pre-wrap font-sans text-fg text-body">{generatedSuggestion}</pre>
            {/if}
          </div>
        {:else if generatedSuggestion}
          <div>
            <pre class="whitespace-pre-wrap font-sans text-fg text-body">{generatedSuggestion}</pre>
            <div class="flex gap-2 mt-4">
              <button
                on:click={applySuggestion}
                class="flex-1 h-8 px-3 bg-accent text-on-accent rounded-control hover:bg-accent-hover transition-colors text-body"
              >
                {$t('reports.editor.apply')}
              </button>
              <button
                on:click={clearSuggestion}
                class="flex-1 h-8 px-3 bg-surface-selected text-fg-muted rounded-control hover:bg-surface-selected transition-colors text-body"
              >
                {$t('reports.editor.discard')}
              </button>
            </div>
          </div>
        {:else}
          <p class="text-body text-fg-subtle italic">
            {$t('reports.editor.suggestionHint')}
          </p>
        {/if}
      </div>
    </div>
  {/if}
</div>
