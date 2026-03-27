<script lang="ts">
  import { page } from '$app/stores';
  import { goto } from '$app/navigation';
  import { onMount, onDestroy } from 'svelte';
  import { listen, type UnlistenFn } from '@tauri-apps/api/event';
  import {
    getEngineStatus,
    getPatient,
    listDiagnosesForPatient,
    listMedicationsForPatient,
    listSessionsForPatient,
    createReport,
    parseError,
    type LlmEngineStatus,
    type CreateReport,
    type Patient,
    type Diagnosis,
    type Medication,
    type Session,
    type AppError,
  } from '$lib/api';
  import { invoke } from '@tauri-apps/api/core';
  import ReportTypeSelector from '$lib/components/ReportTypeSelector.svelte';
  import ReportStream from '$lib/components/ReportStream.svelte';
  import EnhancedReportEditor from '$lib/components/EnhancedReportEditor.svelte';
  import ErrorDisplay from '$lib/components/ErrorDisplay.svelte';
  import { get } from 'svelte/store';
  import { t } from '$lib/translations';

  $: patientId = $page.params.id;

  let selectedType = '';
  let sessionNotes = '';
  let patientContext = '';
  let generatedContent = '';
  let editableContent = '';
  let isGenerating = false;
  let isEditing = false;
  let createMode: 'generate' | 'direct' | null = null;
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
        message: get(t)('reports.selectTypeRequired'),
        ref: 'VALIDATION',
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

      isGenerating = true;
      error = null;
      generatedContent = '';

      // Set up event listeners for streaming
      unlistenChunk = await listen<string>('report-chunk', (event) => {
        generatedContent += event.payload;
      });

      unlistenDone = await listen('report-done', () => {
        isGenerating = false;
        editableContent = stripThinkTags(generatedContent);
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
        systemPrompt: null,
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
        session_ids: null,
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
    createMode = null;
    error = null;
  }

  function stripThinkTags(content: string): string {
    const THINK_START = '<think>';
    const THINK_END = '</think>';

    if (content.startsWith(THINK_START)) {
      const endIdx = content.indexOf(THINK_END);
      if (endIdx !== -1) {
        return content.slice(endIdx + THINK_END.length).trim();
      }
    }
    return content;
  }

  function startDirectCreation() {
    if (!selectedType) {
      error = {
        code: 'VALIDATION_ERROR',
        message: get(t)('reports.selectTypeRequired'),
        ref: 'VALIDATION',
      };
      return;
    }
    createMode = 'direct';
    isEditing = true;
    editableContent = '';
  }

  function formatClinicalContext(
    diagnoses: Diagnosis[],
    medications: Medication[],
    sessions: Session[]
  ): string {
    const lines: string[] = [];

    if (diagnoses.length > 0) {
      lines.push('\nDiagnosen:');
      for (const d of diagnoses) {
        const status =
          d.status === 'active' ? 'aktiv' : d.status === 'chronic' ? 'chronisch' : d.status;
        lines.push(`- ${d.icd10_code} ${d.description} (${status}, seit ${d.diagnosed_date})`);
      }
    }

    const currentMeds = medications.filter((m) => !m.end_date);
    if (currentMeds.length > 0) {
      lines.push('\nAktuelle Medikamente:');
      for (const m of currentMeds) {
        lines.push(`- ${m.substance} ${m.dosage}, ${m.frequency}`);
      }
    }

    if (sessions.length > 0) {
      lines.push('\nLetzte Sitzungen:');
      for (const s of sessions) {
        let line = `- ${s.session_date}: ${s.session_type}`;
        if (s.duration_minutes) line += ` (${s.duration_minutes} min)`;
        const summary = s.clinical_summary || s.notes;
        if (summary) line += ` — ${summary.slice(0, 400)}`;
        lines.push(line);
      }
    }

    return lines.join('\n');
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
      const [patient, diagnoses, medications, sessions] = await Promise.all([
        getPatient(patientId),
        listDiagnosesForPatient(patientId, 20),
        listMedicationsForPatient(patientId, 20),
        listSessionsForPatient(patientId, 5),
      ]);
      patientContext =
        formatPatientContext(patient) + formatClinicalContext(diagnoses, medications, sessions);
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
      <h2 class="text-2xl font-bold text-gray-900 dark:text-gray-100">
        {$t('reports.newReportTitle')}
      </h2>
      <a
        href={`/patients/${patientId}/reports`}
        class="text-sm text-gray-500 dark:text-gray-400 hover:text-gray-700 dark:hover:text-gray-300"
      >
        {$t('reports.backToReports')}
      </a>
    </div>

    <ErrorDisplay {error} showDetails={true} />

    {#if !isEditing}
      <div class="space-y-6">
        <ReportTypeSelector bind:selectedType />

        {#if !llmStatus?.is_loaded && !error}
          <div
            class="p-6 bg-yellow-50 dark:bg-yellow-900/20 border border-yellow-300 dark:border-yellow-500 rounded"
          >
            <h3 class="text-lg font-semibold text-yellow-700 dark:text-yellow-400 mb-2">
              {$t('reports.llmNotConfigured')}
            </h3>
            <p class="text-gray-600 dark:text-gray-300 mb-4">
              {$t('reports.llmNotConfiguredDesc')}
            </p>
            <a
              href="/settings"
              class="inline-block px-4 py-2 bg-blue-600 text-white rounded hover:bg-blue-700 transition-colors"
            >
              {$t('reports.goToSettings')}
            </a>
          </div>
        {/if}

        <!-- Creation mode selection -->
        {#if selectedType && !isGenerating}
          <div
            class="bg-gray-100 dark:bg-gray-800 border border-gray-200 dark:border-gray-700 rounded-lg p-6"
          >
            <h3 class="text-lg font-semibold text-gray-900 dark:text-gray-100 mb-4">
              {$t('reports.howToCreate')}
            </h3>
            <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
              <button
                on:click={() => {
                  createMode = 'generate';
                }}
                disabled={!llmStatus?.is_loaded}
                class="p-6 bg-white dark:bg-gray-900 border-2 border-gray-200 dark:border-gray-700 rounded-lg hover:border-blue-500 transition-colors text-left disabled:opacity-50 disabled:cursor-not-allowed"
              >
                <div class="flex items-center gap-3 mb-2">
                  <svg
                    class="w-6 h-6 text-blue-500 dark:text-blue-400"
                    fill="none"
                    stroke="currentColor"
                    viewBox="0 0 24 24"
                  >
                    <path
                      stroke-linecap="round"
                      stroke-linejoin="round"
                      stroke-width="2"
                      d="M13 10V3L4 14h7v7l9-11h-7z"
                    ></path>
                  </svg>
                  <h4 class="text-lg font-semibold text-gray-900 dark:text-gray-100">
                    {$t('reports.generateWithLlm')}
                  </h4>
                </div>
                <p class="text-sm text-gray-500 dark:text-gray-400">
                  {$t('reports.generateWithLlmDesc')}
                </p>
              </button>

              <button
                on:click={startDirectCreation}
                class="p-6 bg-white dark:bg-gray-900 border-2 border-gray-200 dark:border-gray-700 rounded-lg hover:border-green-500 transition-colors text-left"
              >
                <div class="flex items-center gap-3 mb-2">
                  <svg
                    class="w-6 h-6 text-green-500 dark:text-green-400"
                    fill="none"
                    stroke="currentColor"
                    viewBox="0 0 24 24"
                  >
                    <path
                      stroke-linecap="round"
                      stroke-linejoin="round"
                      stroke-width="2"
                      d="M11 5H6a2 2 0 00-2 2v11a2 2 0 002 2h11a2 2 0 002-2v-5m-1.414-9.414a2 2 0 112.828 2.828L11.828 15H9v-2.828l8.586-8.586z"
                    ></path>
                  </svg>
                  <h4 class="text-lg font-semibold text-gray-900 dark:text-gray-100">
                    {$t('reports.writeManually')}
                  </h4>
                </div>
                <p class="text-sm text-gray-500 dark:text-gray-400">
                  {$t('reports.writeManuallyDesc')}
                </p>
              </button>
            </div>
          </div>
        {/if}

        {#if createMode === 'generate'}
          <div>
            <label
              for="patient-context"
              class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2"
            >
              {$t('reports.patientContext')}
              <span class="text-gray-400 dark:text-gray-500"
                >{$t('reports.patientContextHint')}</span
              >
            </label>
            <textarea
              id="patient-context"
              bind:value={patientContext}
              class="w-full h-32 px-4 py-3 bg-white dark:bg-gray-800 border border-gray-300 dark:border-gray-700 rounded-lg text-gray-900 dark:text-gray-100 focus:outline-none focus:border-blue-500"
              placeholder={$t('reports.patientContextPlaceholder')}
            ></textarea>
          </div>

          <div>
            <label
              for="session-notes"
              class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2"
            >
              {$t('reports.sessionNotes')}
              <span class="text-gray-400 dark:text-gray-500">{$t('reports.optional')}</span>
            </label>
            <textarea
              id="session-notes"
              bind:value={sessionNotes}
              class="w-full h-48 px-4 py-3 bg-white dark:bg-gray-800 border border-gray-300 dark:border-gray-700 rounded-lg text-gray-900 dark:text-gray-100 focus:outline-none focus:border-blue-500 font-mono text-sm"
              placeholder={$t('reports.sessionNotesPlaceholder')}
            ></textarea>
          </div>

          {#if isGenerating}
            <div class="space-y-4">
              <h3 class="text-lg font-semibold text-gray-100">
                {$t('reports.generatedReport')}
              </h3>
              <ReportStream content={generatedContent} isStreaming={isGenerating} />
            </div>
          {/if}

          <div class="flex justify-end space-x-4">
            <button
              on:click={reset}
              class="px-6 py-2 bg-gray-200 dark:bg-gray-700 text-gray-700 dark:text-gray-300 rounded hover:bg-gray-300 dark:hover:bg-gray-600 transition-colors"
              disabled={isGenerating}
            >
              {$t('reports.reset')}
            </button>
            <button
              on:click={generateReport}
              disabled={!selectedType || isGenerating}
              class="px-6 py-2 bg-blue-600 text-white rounded hover:bg-blue-700 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
            >
              {isGenerating ? $t('reports.generating') : $t('reports.generate')}
            </button>
          </div>
        {/if}
      </div>
    {:else}
      <div class="space-y-6">
        <div>
          <h3 class="text-lg font-semibold text-gray-900 dark:text-gray-100 mb-2">
            {createMode === 'generate' ? $t('reports.editGenerated') : $t('reports.writeNew')}
          </h3>
          <p class="text-sm text-gray-500 dark:text-gray-400 mb-4">
            {createMode === 'generate'
              ? $t('reports.reviewBeforeSaving')
              : $t('reports.writeWithSuggestions')}
          </p>
        </div>

        <div class="h-[600px]">
          <EnhancedReportEditor bind:content={editableContent} />
        </div>

        <div class="flex justify-end space-x-4">
          <button
            on:click={() => {
              isEditing = false;
              generatedContent = '';
              editableContent = '';
              createMode = null;
            }}
            class="px-6 py-2 bg-gray-200 dark:bg-gray-700 text-gray-700 dark:text-gray-300 rounded hover:bg-gray-300 dark:hover:bg-gray-600 transition-colors"
          >
            {createMode === 'generate' ? $t('reports.regenerate') : $t('common.cancel')}
          </button>
          <button
            on:click={saveReport}
            class="px-6 py-2 bg-green-600 text-white rounded hover:bg-green-700 transition-colors"
          >
            {$t('reports.save')}
          </button>
        </div>
      </div>
    {/if}
  </div>
</div>
