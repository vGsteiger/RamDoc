<script lang="ts">
  import { page } from '$app/stores';
  import { goto } from '$app/navigation';
  import { resolve } from '$app/paths';
  import { onMount, onDestroy } from 'svelte';
  import { listen, type UnlistenFn } from '@tauri-apps/api/event';
  import {
    getEngineStatus,
    getPatient,
    listDiagnosesForPatient,
    listMedicationsForPatient,
    listSessionsForPatient,
    listTreatmentGoalsForPlan,
    listTreatmentPlansForPatient,
    createReport,
    parseError,
    type LlmEngineStatus,
    type CreateReport,
    type Patient,
    type Diagnosis,
    type Medication,
    type Session,
    type TreatmentGoal,
    type TreatmentPlan,
    type AppError,
  } from '$lib/api';
  import { invoke } from '@tauri-apps/api/core';
  import ReportTypeSelector from '$lib/components/ReportTypeSelector.svelte';
  import ReportStream from '$lib/components/ReportStream.svelte';
  import EnhancedReportEditor from '$lib/components/EnhancedReportEditor.svelte';
  import ErrorDisplay from '$lib/components/ErrorDisplay.svelte';
  import { get } from 'svelte/store';
  import { t } from '$lib/translations';
  import ThinkingEffortSelect from '$lib/components/ThinkingEffortSelect.svelte';
  import { thinkingEffort } from '$lib/stores/thinking';
  import { cleanGeneratedReport } from '$lib/llm/clean-generated-report';

  $: patientId = $page.params.id!;

  let selectedType = '';
  let sessionNotes = '';
  let patientContext = '';
  let instructions = '';
  let referralRecipient = '';
  let referralQuestion = '';
  let uploadedFileContent = '';
  let uploadedFileName = '';
  let generatedContent = '';
  let editableContent = '';
  let isGenerating = false;
  let isEditing = false;
  let createMode: 'generate' | 'direct' | null = null;
  let error: AppError | null = null;
  let llmStatus: LlmEngineStatus | null = null;

  let isSummarizing = false;
  let unlistenChunk: UnlistenFn | null = null;
  let unlistenDone: UnlistenFn | null = null;
  let unlistenSummarizing: UnlistenFn | null = null;

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

    if (selectedType === 'Ueberweisungsschreiben' && !referralQuestion.trim()) {
      error = {
        code: 'VALIDATION_ERROR',
        message: get(t)('reports.referralQuestionRequired'),
        ref: 'VALIDATION',
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
      isSummarizing = false;
      error = null;
      generatedContent = '';

      // Set up event listeners for streaming
      unlistenSummarizing = await listen('report-summarizing', () => {
        isSummarizing = true;
      });

      unlistenChunk = await listen<string>('report-chunk', (event) => {
        isSummarizing = false;
        generatedContent += event.payload;
      });

      unlistenDone = await listen('report-done', () => {
        isGenerating = false;
        isSummarizing = false;
        editableContent = cleanGeneratedReport(generatedContent);
        isEditing = true;
        // Unlisten after completion
        if (unlistenSummarizing) {
          unlistenSummarizing();
          unlistenSummarizing = null;
        }
        if (unlistenChunk) {
          unlistenChunk();
          unlistenChunk = null;
        }
        if (unlistenDone) {
          unlistenDone();
          unlistenDone = null;
        }
      });

      const generationInstructions = buildGenerationInstructions();
      await invoke('generate_report', {
        patientContext,
        reportType: selectedType,
        sessionNotes,
        additionalContext: uploadedFileContent || null,
        instructions: generationInstructions || null,
        systemPrompt: null,
        thinkingEffort: get(thinkingEffort),
      });
    } catch (e) {
      error = parseError(e);
      isGenerating = false;
      isSummarizing = false;
      // Unlisten on error
      if (unlistenSummarizing) {
        unlistenSummarizing();
        unlistenSummarizing = null;
      }
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
      await goto(resolve('/patients/[id]/reports', { id: patientId }));
    } catch (e) {
      error = parseError(e);
    }
  }

  function minifyText(text: string): string {
    return text.replace(/\s+/g, ' ').trim();
  }

  function handleFileUpload(event: Event) {
    const file = (event.target as HTMLInputElement).files?.[0];
    if (!file) return;
    const reader = new FileReader();
    reader.onload = (e) => {
      uploadedFileContent = minifyText((e.target?.result as string) ?? '');
      uploadedFileName = file.name;
    };
    reader.readAsText(file);
  }

  function clearFile() {
    uploadedFileContent = '';
    uploadedFileName = '';
  }

  function reset() {
    selectedType = '';
    sessionNotes = '';
    patientContext = '';
    instructions = '';
    referralRecipient = '';
    referralQuestion = '';
    uploadedFileContent = '';
    uploadedFileName = '';
    generatedContent = '';
    editableContent = '';
    isEditing = false;
    createMode = null;
    error = null;
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

  function buildGenerationInstructions(): string {
    const parts: string[] = [];
    if (selectedType === 'Ueberweisungsschreiben') {
      if (referralRecipient.trim()) {
        parts.push(`Empfänger oder Fachgebiet: ${referralRecipient.trim()}`);
      }
      parts.push(`Überweisungsgrund und konkrete Fragestellung: ${referralQuestion.trim()}`);
    }
    if (instructions.trim()) parts.push(`Weitere Vorgaben: ${instructions.trim()}`);
    return parts.join('\n\n');
  }

  function formatClinicalContext(
    diagnoses: Diagnosis[],
    medications: Medication[],
    sessions: Session[],
    treatmentPlans: TreatmentPlan[],
    goalsByPlan: Map<string, TreatmentGoal[]>
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
        let line = `- ${m.substance} ${m.dosage}, ${m.frequency}`;
        if (m.notes) line += ` (${m.notes})`;
        lines.push(line);
      }
    }

    const activePlans = treatmentPlans.filter((plan) => plan.status === 'active');
    if (activePlans.length > 0) {
      lines.push('\nAktuelle Behandlungspläne:');
      for (const plan of activePlans) {
        let line = `- ${plan.title} (${plan.status}, seit ${plan.start_date})`;
        if (plan.description) line += `: ${plan.description}`;
        lines.push(line);
        for (const goal of goalsByPlan.get(plan.id) ?? []) {
          lines.push(`  Therapieziel (${goal.status}): ${goal.description}`);
        }
      }
    }

    if (sessions.length > 0) {
      lines.push('\nLetzte Sitzungen:');
      for (const s of sessions) {
        let line = `- ${s.session_date}: ${s.session_type}`;
        if (s.duration_minutes) line += ` (${s.duration_minutes} min)`;
        const summary = s.clinical_summary || s.notes;
        if (summary) line += ` — ${summary.slice(0, 800)}`;
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
      let treatmentPlans: TreatmentPlan[] = [];
      try {
        treatmentPlans = await listTreatmentPlansForPatient(patientId, 10);
      } catch (e) {
        console.warn('Failed to load optional treatment-plan context:', e);
      }
      const goalEntries = await Promise.all(
        treatmentPlans.map(async (plan) => {
          try {
            return [plan.id, await listTreatmentGoalsForPlan(plan.id, 20)] as const;
          } catch {
            return [plan.id, [] as TreatmentGoal[]] as const;
          }
        })
      );
      patientContext =
        formatPatientContext(patient) +
        formatClinicalContext(
          diagnoses,
          medications,
          sessions,
          treatmentPlans,
          new Map(goalEntries)
        );
    } catch (e) {
      // Non-fatal: user can still fill in patient context manually
      console.error('Failed to load patient data:', e);
    }
  });

  onDestroy(() => {
    if (unlistenSummarizing) unlistenSummarizing();
    if (unlistenChunk) unlistenChunk();
    if (unlistenDone) unlistenDone();
  });
</script>

<div class="p-8">
  <div class="max-w-5xl mx-auto">
    <div class="flex items-center justify-between mb-6">
      <h2 class="text-display font-semibold text-fg">
        {$t('reports.newReportTitle')}
      </h2>
      <a
        href={resolve('/patients/[id]/reports', { id: patientId })}
        class="text-body text-fg-muted hover:text-fg"
      >
        {$t('reports.backToReports')}
      </a>
    </div>

    <ErrorDisplay {error} showDetails={true} />

    {#if !isEditing}
      <div class="space-y-6">
        <ReportTypeSelector bind:selectedType />

        {#if !llmStatus?.is_loaded && !error}
          <div class="p-6 bg-warning-subtle border border-warning-line rounded-card">
            <h3 class="text-heading font-semibold text-warning-fg mb-2">
              {$t('reports.llmNotConfigured')}
            </h3>
            <p class="text-fg-muted mb-4">
              {$t('reports.llmNotConfiguredDesc')}
            </p>
            <a
              href={resolve('/settings')}
              class="inline-flex items-center inline-block h-8 px-3 bg-accent text-on-accent rounded-card hover:bg-accent-hover transition-colors"
            >
              {$t('reports.goToSettings')}
            </a>
          </div>
        {/if}

        <!-- Creation mode selection -->
        {#if selectedType && !isGenerating}
          <div class="bg-surface-hover border border-line rounded-card p-6">
            <h3 class="text-heading font-semibold text-fg mb-4">
              {$t('reports.howToCreate')}
            </h3>
            <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
              <button
                on:click={() => {
                  createMode = 'generate';
                }}
                disabled={!llmStatus?.is_loaded}
                class="p-6 bg-surface-raised border-2 border-line rounded-control hover:border-accent transition-colors text-left disabled:opacity-50 disabled:cursor-not-allowed"
              >
                <div class="flex items-center gap-3 mb-2">
                  <svg
                    class="w-6 h-6 text-accent-fg"
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
                  <h4 class="text-heading font-semibold text-fg">
                    {$t('reports.generateWithLlm')}
                  </h4>
                </div>
                <p class="text-body text-fg-muted">
                  {$t('reports.generateWithLlmDesc')}
                </p>
              </button>

              <button
                on:click={startDirectCreation}
                class="p-6 bg-surface-raised border-2 border-line rounded-control hover:border-success transition-colors text-left"
              >
                <div class="flex items-center gap-3 mb-2">
                  <svg
                    class="w-6 h-6 text-success-fg"
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
                  <h4 class="text-heading font-semibold text-fg">
                    {$t('reports.writeManually')}
                  </h4>
                </div>
                <p class="text-body text-fg-muted">
                  {$t('reports.writeManuallyDesc')}
                </p>
              </button>
            </div>
          </div>
        {/if}

        {#if createMode === 'generate'}
          {#if selectedType === 'Ueberweisungsschreiben'}
            <section class="bg-accent-subtle border border-accent/30 rounded-card p-5 space-y-4">
              <div>
                <h3 class="text-heading font-semibold text-fg">
                  {$t('reports.referralDetails')}
                </h3>
                <p class="text-body text-fg-muted mt-1">
                  {$t('reports.referralDetailsHint')}
                </p>
              </div>
              <div>
                <label
                  for="referral-recipient"
                  class="block text-body font-medium text-fg-muted mb-2"
                >
                  {$t('reports.referralRecipient')}
                  <span class="text-fg-subtle">{$t('reports.optional')}</span>
                </label>
                <input
                  id="referral-recipient"
                  bind:value={referralRecipient}
                  class="w-full px-4 py-3 bg-surface-raised border border-line rounded-control text-fg focus:outline-none focus:border-accent"
                  placeholder={$t('reports.referralRecipientPlaceholder')}
                />
              </div>
              <div>
                <label
                  for="referral-question"
                  class="block text-body font-medium text-fg-muted mb-2"
                >
                  {$t('reports.referralQuestion')}
                </label>
                <textarea
                  id="referral-question"
                  bind:value={referralQuestion}
                  required
                  class="w-full h-28 px-4 py-3 bg-surface-raised border border-line rounded-control text-fg focus:outline-none focus:border-accent"
                  placeholder={$t('reports.referralQuestionPlaceholder')}></textarea>
              </div>
            </section>
          {/if}

          <div>
            <label for="patient-context" class="block text-body font-medium text-fg-muted mb-2">
              {$t('reports.patientContext')}
              <span class="text-fg-subtle">{$t('reports.patientContextHint')}</span>
            </label>
            <textarea
              id="patient-context"
              bind:value={patientContext}
              class="w-full h-32 px-4 py-3 bg-surface-raised border border-line rounded-control text-fg focus:outline-none focus:border-accent"
              placeholder={$t('reports.patientContextPlaceholder')}></textarea>
          </div>

          <div>
            <label for="session-notes" class="block text-body font-medium text-fg-muted mb-2">
              {$t('reports.sessionNotes')}
              <span class="text-fg-subtle">{$t('reports.optional')}</span>
            </label>
            <textarea
              id="session-notes"
              bind:value={sessionNotes}
              class="w-full h-48 px-4 py-3 bg-surface-raised border border-line rounded-control text-fg focus:outline-none focus:border-accent font-mono text-body"
              placeholder={$t('reports.sessionNotesPlaceholder')}></textarea>
          </div>

          <div>
            <!-- Section caption, not a control label: the file input below is wrapped
                 by its own <label>. -->
            <p class="block text-body font-medium text-fg-muted mb-2">
              {$t('reports.additionalContext')}
              <span class="text-fg-subtle">{$t('reports.additionalContextHint')}</span>
            </p>
            {#if uploadedFileName}
              <div
                class="flex items-center gap-3 px-4 py-2 bg-success-subtle border border-success rounded-card"
              >
                <svg
                  class="w-4 h-4 text-success-fg flex-shrink-0"
                  fill="none"
                  stroke="currentColor"
                  viewBox="0 0 24 24"
                >
                  <path
                    stroke-linecap="round"
                    stroke-linejoin="round"
                    stroke-width="2"
                    d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z"
                  ></path>
                </svg>
                <span class="text-body text-success-fg flex-1">
                  {$t('reports.uploadedFile').replace('{name}', uploadedFileName)}
                  <span class="text-fg-muted ml-2">
                    {$t('reports.fileChars').replace('{count}', String(uploadedFileContent.length))}
                  </span>
                </span>
                <button on:click={clearFile} class="text-body text-danger-fg hover:underline">
                  {$t('reports.clearFile')}
                </button>
              </div>
            {:else}
              <label
                class="flex items-center gap-2 px-4 py-2 bg-surface-raised border border-line rounded-card cursor-pointer hover:border-accent transition-colors w-fit"
              >
                <svg
                  class="w-4 h-4 text-fg-muted"
                  fill="none"
                  stroke="currentColor"
                  viewBox="0 0 24 24"
                >
                  <path
                    stroke-linecap="round"
                    stroke-linejoin="round"
                    stroke-width="2"
                    d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-8l-4-4m0 0L8 8m4-4v12"
                  ></path>
                </svg>
                <span class="text-body text-fg-muted">{$t('reports.uploadTxtFile')}</span>
                <input
                  type="file"
                  accept=".txt,text/plain"
                  class="hidden"
                  on:change={handleFileUpload}
                />
              </label>
            {/if}
          </div>

          <div>
            <label for="report-instructions" class="block text-body font-medium text-fg-muted mb-2">
              {$t('reports.instructions')}
              <span class="text-fg-subtle">{$t('reports.instructionsHint')}</span>
            </label>
            <textarea
              id="report-instructions"
              bind:value={instructions}
              class="w-full h-20 px-4 py-3 bg-surface-raised border border-line rounded-control text-fg focus:outline-none focus:border-accent text-body"
              placeholder={$t('reports.instructionsPlaceholder')}></textarea>
          </div>

          {#if isGenerating}
            <div class="space-y-4">
              <h3 class="text-heading font-semibold text-fg">
                {$t('reports.generatedReport')}
              </h3>
              <ReportStream content={generatedContent} isStreaming={isGenerating} {isSummarizing} />
            </div>
          {/if}

          <div class="flex flex-wrap items-center justify-end gap-3">
            <ThinkingEffortSelect disabled={isGenerating} />
            <button
              on:click={reset}
              class="h-8 px-3 bg-surface-selected text-fg-muted rounded-control hover:bg-surface-selected transition-colors"
              disabled={isGenerating}
            >
              {$t('reports.reset')}
            </button>
            <button
              on:click={generateReport}
              disabled={!selectedType || isGenerating}
              class="h-8 px-3 bg-accent text-on-accent rounded-control hover:bg-accent-hover transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
            >
              {isGenerating ? $t('reports.generating') : $t('reports.generate')}
            </button>
          </div>
        {/if}
      </div>
    {:else}
      <div class="space-y-6">
        <div>
          <h3 class="text-heading font-semibold text-fg mb-2">
            {createMode === 'generate' ? $t('reports.editGenerated') : $t('reports.writeNew')}
          </h3>
          <p class="text-body text-fg-muted mb-4">
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
            class="h-8 px-3 bg-surface-selected text-fg-muted rounded-control hover:bg-surface-selected transition-colors"
          >
            {createMode === 'generate' ? $t('reports.regenerate') : $t('common.cancel')}
          </button>
          <button
            on:click={saveReport}
            class="h-8 px-3 bg-success text-on-success rounded-control hover:bg-success-hover transition-colors"
          >
            {$t('reports.save')}
          </button>
        </div>
      </div>
    {/if}
  </div>
</div>
