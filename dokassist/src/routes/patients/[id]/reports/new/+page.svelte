<script lang="ts">
  import { page } from '$app/stores';
  import { goto } from '$app/navigation';
  import { onMount, onDestroy } from 'svelte';
  import { listen, type UnlistenFn } from '@tauri-apps/api/event';
  import { getEngineStatus, createReport, type LlmEngineStatus, type CreateReport } from '$lib/api';
  import { invoke } from '@tauri-apps/api/core';
  import ReportTypeSelector from '$lib/components/ReportTypeSelector.svelte';
  import ReportStream from '$lib/components/ReportStream.svelte';
  import ReportEditor from '$lib/components/ReportEditor.svelte';

  $: patientId = $page.params.id;

  let selectedType = '';
  let sessionNotes = '';
  let patientContext = '';
  let generatedContent = '';
  let editableContent = '';
  let isGenerating = false;
  let isEditing = false;
  let error = '';
  let llmStatus: LlmEngineStatus | null = null;

  let unlistenChunk: UnlistenFn | null = null;
  let unlistenDone: UnlistenFn | null = null;

  async function checkLlmStatus() {
    try {
      llmStatus = await getEngineStatus();
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    }
  }

  async function generateReport() {
    if (!selectedType) {
      error = 'Please select a report type';
      return;
    }

    if (!llmStatus?.is_loaded) {
      error = 'LLM model not loaded. Please configure the model in Settings.';
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

      isGenerating = true;
      error = '';
      generatedContent = '';

      // Set up event listeners for streaming
      unlistenChunk = await listen<string>('report-chunk', (event) => {
        generatedContent += event.payload;
      });

      unlistenDone = await listen('report-done', () => {
        isGenerating = false;
        editableContent = generatedContent;
        isEditing = true;
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

      // Start generation with snake_case keys
      await invoke('generate_report', {
        patient_context: patientContext,
        report_type: selectedType,
        session_notes: sessionNotes,
        system_prompt: null
      });
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
      isGenerating = false;
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

  async function saveReport() {
    try {
      const input: CreateReport = {
        patient_id: patientId,
        report_type: selectedType,
        content: editableContent,
        model_name: llmStatus?.model_name || null,
        prompt_hash: null,
        session_ids: null
      };

      await createReport(input);
      await goto(`/patients/${patientId}/reports`);
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    }
  }

  function reset() {
    selectedType = '';
    sessionNotes = '';
    patientContext = '';
    generatedContent = '';
    editableContent = '';
    isEditing = false;
    error = '';
  }

  onMount(() => {
    checkLlmStatus();
  });

  onDestroy(() => {
    if (unlistenChunk) unlistenChunk();
    if (unlistenDone) unlistenDone();
  });
</script>

<div class="p-8">
  <div class="max-w-5xl mx-auto">
    <div class="flex items-center justify-between mb-6">
      <h2 class="text-2xl font-bold text-gray-100">Generate Report</h2>
      <a
        href={`/patients/${patientId}/reports`}
        class="text-sm text-gray-400 hover:text-gray-300"
      >
        ← Back to Reports
      </a>
    </div>

    {#if error}
      <div class="mb-6 p-4 bg-red-900/20 border border-red-500 rounded text-red-400">
        {error}
      </div>
    {/if}

    {#if !llmStatus?.is_loaded && !error}
      <div class="p-6 bg-yellow-900/20 border border-yellow-500 rounded">
        <h3 class="text-lg font-semibold text-yellow-400 mb-2">LLM Not Configured</h3>
        <p class="text-gray-300 mb-4">
          You need to download and load a language model before generating reports.
        </p>
        <a
          href="/settings"
          class="inline-block px-4 py-2 bg-blue-600 text-white rounded hover:bg-blue-700 transition-colors"
        >
          Go to Settings
        </a>
      </div>
    {:else if !isEditing}
      <div class="space-y-6">
        <ReportTypeSelector bind:selectedType />

        <div>
          <label class="block text-sm font-medium text-gray-300 mb-2">
            Patient Context
            <span class="text-gray-500">(optional)</span>
          </label>
          <textarea
            bind:value={patientContext}
            class="w-full h-32 px-4 py-3 bg-gray-800 border border-gray-700 rounded-lg text-gray-100 focus:outline-none focus:border-blue-500"
            placeholder="Enter relevant patient information, diagnoses, medications..."
          />
        </div>

        <div>
          <label class="block text-sm font-medium text-gray-300 mb-2">
            Session Notes
            <span class="text-gray-500">(optional)</span>
          </label>
          <textarea
            bind:value={sessionNotes}
            class="w-full h-48 px-4 py-3 bg-gray-800 border border-gray-700 rounded-lg text-gray-100 focus:outline-none focus:border-blue-500 font-mono text-sm"
            placeholder="Enter session notes or select sessions to include..."
          />
        </div>

        {#if isGenerating}
          <div class="space-y-4">
            <h3 class="text-lg font-semibold text-gray-100">Generated Report</h3>
            <ReportStream content={generatedContent} isStreaming={isGenerating} />
          </div>
        {/if}

        <div class="flex justify-end space-x-4">
          <button
            on:click={reset}
            class="px-6 py-2 bg-gray-700 text-gray-300 rounded hover:bg-gray-600 transition-colors"
            disabled={isGenerating}
          >
            Reset
          </button>
          <button
            on:click={generateReport}
            disabled={!selectedType || isGenerating}
            class="px-6 py-2 bg-blue-600 text-white rounded hover:bg-blue-700 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
          >
            {isGenerating ? 'Generating...' : 'Generate Report'}
          </button>
        </div>
      </div>
    {:else}
      <div class="space-y-6">
        <div>
          <h3 class="text-lg font-semibold text-gray-100 mb-2">
            Edit Generated Report
          </h3>
          <p class="text-sm text-gray-400 mb-4">
            Review and edit the generated report before saving.
          </p>
        </div>

        <div class="h-[500px]">
          <ReportEditor bind:content={editableContent} />
        </div>

        <div class="flex justify-end space-x-4">
          <button
            on:click={() => {
              isEditing = false;
              generatedContent = '';
              editableContent = '';
            }}
            class="px-6 py-2 bg-gray-700 text-gray-300 rounded hover:bg-gray-600 transition-colors"
          >
            Regenerate
          </button>
          <button
            on:click={saveReport}
            class="px-6 py-2 bg-green-600 text-white rounded hover:bg-green-700 transition-colors"
          >
            Save Report
          </button>
        </div>
      </div>
    {/if}
  </div>
</div>
