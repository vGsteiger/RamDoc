<script lang="ts">
  import { page } from '$app/stores';
  import { goto } from '$app/navigation';
  import { onMount, onDestroy } from 'svelte';
  import { listen, type UnlistenFn } from '@tauri-apps/api/event';
  import { getEngineStatus, getPatient, createReport, parseError, type LlmEngineStatus, type CreateReport, type Patient, type AppError } from '$lib/api';
  import { invoke } from '@tauri-apps/api/core';
  import ReportTypeSelector from '$lib/components/ReportTypeSelector.svelte';
  import ReportStream from '$lib/components/ReportStream.svelte';
  import ReportEditor from '$lib/components/ReportEditor.svelte';
  import ErrorDisplay from '$lib/components/ErrorDisplay.svelte';

  $: patientId = $page.params.id;

  let selectedType = '';
  let sessionNotes = '';
  let patientContext = '';
  let generatedContent = '';
  let editableContent = '';
  let isGenerating = false;
  let isEditing = false;
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

  async function generateReport() {
    if (!selectedType) {
      error = {
        code: 'VALIDATION_ERROR',
        message: 'Please select a report type',
        ref: 'VALIDATION'
      };
      return;
    }

    if (!llmStatus?.is_loaded) {
      error = {
        code: 'LLM_ERROR',
        message: 'LLM model not loaded. Please configure the model in Settings.',
        ref: 'LLM_NOT_LOADED'
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

      isGenerating = true;
      error = null;
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

      await invoke('generate_report', {
        patientContext,
        reportType: selectedType,
        sessionNotes,
        systemPrompt: null
      });
    } catch (e) {
      error = parseError(e);
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
      error = parseError(e);
    }
  }

  function reset() {
    selectedType = '';
    sessionNotes = '';
    patientContext = '';
    generatedContent = '';
    editableContent = '';
    isEditing = false;
    error = null;
  }

  function formatPatientContext(p: Patient): string {
    const lines: string[] = [];
    lines.push(`Name: ${p.first_name} ${p.last_name}`);
    if (p.date_of_birth) lines.push(`Geburtsdatum: ${p.date_of_birth}`);
    if (p.gender) lines.push(`Geschlecht: ${p.gender}`);
    if (p.ahv_number) lines.push(`AHV-Nummer: ${p.ahv_number}`);
    if (p.address) lines.push(`Adresse: ${p.address}`);
    if (p.phone) lines.push(`Telefon: ${p.phone}`);
    if (p.email) lines.push(`E-Mail: ${p.email}`);
    if (p.insurance) lines.push(`Versicherung: ${p.insurance}`);
    if (p.gp_name) lines.push(`Hausarzt: ${p.gp_name}`);
    if (p.gp_address) lines.push(`Hausarzt-Adresse: ${p.gp_address}`);
    if (p.notes) lines.push(`Notizen: ${p.notes}`);
    return lines.join('\n');
  }

  onMount(async () => {
    await checkLlmStatus();
    try {
      const patient = await getPatient(patientId);
      patientContext = formatPatientContext(patient);
    } catch (e) {
      // Non-fatal: user can still fill in patient context manually
      console.error('Failed to load patient data:', e);
    }
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

    <ErrorDisplay {error} showDetails={true} />

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
            Patientenkontext
            <span class="text-gray-500">(automatisch befüllt, bearbeitbar)</span>
          </label>
          <textarea
            bind:value={patientContext}
            class="w-full h-32 px-4 py-3 bg-gray-800 border border-gray-700 rounded-lg text-gray-100 focus:outline-none focus:border-blue-500"
            placeholder="Patientendaten werden geladen..."
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
