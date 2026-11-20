<script lang="ts">
  import { onDestroy } from 'svelte';
  import { listen, type UnlistenFn } from '@tauri-apps/api/event';
  import { invoke } from '@tauri-apps/api/core';
  import { getEngineStatus, parseError, type LlmEngineStatus, type AppError } from '$lib/api';
  import { get } from 'svelte/store';
  import { t } from '$lib/translations';
  import { thinkingEffort } from '$lib/stores/thinking';
  import { stripThinkTags } from '$lib/llm/strip-think';
  import {
    Alert,
    Button,
    Markdown,
    SegmentedControl,
    StatusIndicator,
    Textarea,
    ThinkingIndicator,
  } from '$lib/components/ui';
  import DictationButton from '$lib/components/DictationButton.svelte';
  import { insertDictation } from '$lib/dictation';

  /*
   * Report text editor with an Edit/Preview switch, dictation, and optional
   * LLM suggestions for the selection (or the whole text). Sized for the
   * prose measure: the suggestions panel opens below the text, not beside it.
   */
  let {
    content = $bindable(''),
    readonly = false,
  }: {
    content?: string;
    readonly?: boolean;
  } = $props();

  let mode = $state<'edit' | 'preview'>('edit');
  let showSuggestions = $state(false);
  let selectedText = $state('');
  let suggestionInstruction = $state(get(t)('reports.editor.defaultInstruction'));
  let generatedSuggestion = $state('');
  let isGeneratingSuggestion = $state(false);
  let activityStartedAt: number | null = $state(null);
  let error: AppError | null = $state(null);
  let llmStatus: LlmEngineStatus | null = $state(null);

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

  let modeOptions = $derived([
    { value: 'edit', label: $t('reports.edit') },
    { value: 'preview', label: $t('reports.preview') },
  ]);
</script>

<div class="space-y-3">
  <div class="flex flex-wrap items-center gap-2">
    <SegmentedControl
      options={modeOptions}
      bind:value={mode}
      label={$t('reports.editorMode')}
      size="sm"
    />
    {#if !readonly && mode === 'edit'}
      <DictationButton
        onTranscript={(text) =>
          (content = insertDictation(
            document.getElementById('report-textarea') as HTMLTextAreaElement | null,
            content,
            text
          ))}
      />
      <Button
        size="sm"
        variant={showSuggestions ? 'secondary' : 'ghost'}
        aria-expanded={showSuggestions}
        onclick={() => (showSuggestions = !showSuggestions)}
        disabled={!llmStatus?.is_loaded}
      >
        {showSuggestions ? $t('reports.editor.closeSuggestions') : $t('reports.editor.suggestions')}
      </Button>
      {#if selectedText}
        <span class="text-caption text-fg-subtle" data-numeric>
          {$t('reports.editor.charsSelected').replace('{count}', String(selectedText.length))}
        </span>
      {/if}
    {/if}
  </div>

  {#if mode === 'preview'}
    <div
      class="min-h-80 card-surface rounded-panel px-5 py-4 sm:px-8 sm:py-6"
    >
      {#if content}
        <Markdown source={content} headingOffset={1} />
      {:else}
        <p class="text-body text-fg-subtle">{$t('reports.noPreviewContent')}</p>
      {/if}
    </div>
  {:else}
    <Textarea
      id="report-textarea"
      bind:value={content}
      onselect={handleTextSelection}
      onmouseup={handleTextSelection}
      onkeyup={handleTextSelection}
      {readonly}
      rows={22}
      class="resize-y px-4 py-3 leading-relaxed"
      placeholder={$t('reports.contentPlaceholder')}
      aria-label={$t('reports.edit')}
    />
  {/if}

  {#if showSuggestions && !readonly}
    <section
      class="card-surface rounded-panel"
      aria-label={$t('reports.editor.suggestions')}
    >
      <div class="space-y-3 border-b border-line-subtle p-4">
        <h3 class="text-heading text-fg">
          {selectedText
            ? $t('reports.editor.suggestionsFor')
            : $t('reports.editor.suggestionsForAll')}
        </h3>
        {#if error}
          <Alert tone="danger">{error.message}</Alert>
        {/if}
        {#if !llmStatus?.is_loaded}
          <StatusIndicator tone="warn" label={$t('reports.editor.modelNotLoaded')} />
        {:else}
          <Textarea
            bind:value={suggestionInstruction}
            rows={2}
            placeholder={$t('reports.editor.instructionPlaceholder')}
            aria-label={$t('reports.editor.instructionPlaceholder')}
          />
          <Button size="sm" onclick={generateSuggestion} loading={isGeneratingSuggestion}>
            {isGeneratingSuggestion
              ? $t('reports.editor.generating')
              : $t('reports.editor.generateSuggestion')}
          </Button>
        {/if}
      </div>

      <div class="p-4">
        {#if isGeneratingSuggestion}
          <div class="space-y-3" role="status" aria-live="polite">
            {#if activityStartedAt}
              <ThinkingIndicator
                startedAt={activityStartedAt}
                label={$t('reports.editor.activityWriting')}
              />
            {/if}
            {#if generatedSuggestion}
              <pre
                class="whitespace-pre-wrap font-sans text-body text-fg">{generatedSuggestion}</pre>
            {/if}
          </div>
        {:else if generatedSuggestion}
          <pre class="whitespace-pre-wrap font-sans text-body text-fg">{generatedSuggestion}</pre>
          <div class="mt-4 flex flex-wrap gap-2">
            <Button size="sm" onclick={applySuggestion}>{$t('reports.editor.apply')}</Button>
            <Button size="sm" variant="ghost" onclick={clearSuggestion}>
              {$t('reports.editor.discard')}
            </Button>
          </div>
        {:else}
          <p class="text-label text-fg-subtle">{$t('reports.editor.suggestionHint')}</p>
        {/if}
      </div>
    </section>
  {/if}
</div>
