<script lang="ts">
  import { page } from "$app/stores";
  import { goto } from "$app/navigation";
  import { onMount, onDestroy } from "svelte";
  import { listen, type UnlistenFn } from "@tauri-apps/api/event";
  import {
    getSession,
    getPatient,
    updateSession,
    deleteSession,
    getEngineStatus,
    listDiagnosesForPatient,
    listScoresForSession,
    createOutcomeScore,
    updateOutcomeScore,
    deleteOutcomeScore,
    parseError,
    type Session,
    type Patient,
    type UpdateSession,
    type AppError,
    type LlmEngineStatus,
    type Diagnosis,
    type OutcomeScore,
    type CreateOutcomeScore,
    type UpdateOutcomeScore,
  } from "$lib/api";
  import { invoke } from "@tauri-apps/api/core";
  import ReportStream from "$lib/components/ReportStream.svelte";
  import ErrorDisplay from "$lib/components/ErrorDisplay.svelte";
  import OutcomeScoreCard from "$lib/components/OutcomeScoreCard.svelte";
  import OutcomeScoreForm from "$lib/components/OutcomeScoreForm.svelte";
  import { t } from "$lib/translations";

  const patientId = $derived($page.params.id);
  const sessionId = $derived($page.params.sessionId);

  let session = $state<Session | null>(null);
  let patient = $state<Patient | null>(null);
  let diagnoses = $state<Diagnosis[]>([]);
  let scores = $state<OutcomeScore[]>([]);
  let isLoading = $state(true);
  let isEditing = $state(false);
  let isGenerating = $state(false);
  let isSaving = $state(false);
  let isDeleting = $state(false);
  let loadingScores = $state(false);
  let showDeleteConfirm = $state(false);
  let showAddForm = $state(false);
  let editingScore = $state<OutcomeScore | null>(null);
  let savingScore = $state(false);
  let error = $state<AppError | null>(null);
  let llmStatus = $state<LlmEngineStatus | null>(null);

  let editedNotes = $state("");
  let editedDuration = $state<number | null>(null);
  let editedSessionType = $state("");
  let editedSessionDate = $state("");

  let generatedSummary = $state("");
  let editableSummary = $state("");
  let showSummaryEditor = $state(false);

  let unlistenChunk: UnlistenFn | null = null;
  let unlistenDone: UnlistenFn | null = null;

  onMount(async () => {
    await Promise.all([loadSession(), loadPatient(), loadDiagnoses(), checkLlmStatus(), loadScores()]);
  });

  onDestroy(() => {
    if (unlistenChunk) unlistenChunk();
    if (unlistenDone) unlistenDone();
  });

  async function loadSession() {
    try {
      isLoading = true;
      error = null;
      session = await getSession(sessionId);
      editedNotes = session.notes || "";
      editedDuration = session.duration_minutes;
      editedSessionType = session.session_type;
      editedSessionDate = session.session_date;
      editableSummary = session.clinical_summary || "";
    } catch (e) {
      error = parseError(e);
      console.error("Failed to load session:", e);
    } finally {
      isLoading = false;
    }
  }

  async function loadPatient() {
    try {
      patient = await getPatient(patientId);
    } catch (e) {
      console.error("Failed to load patient:", e);
    }
  }

  async function loadDiagnoses() {
    try {
      diagnoses = await listDiagnosesForPatient(patientId);
    } catch (e) {
      console.error("Failed to load diagnoses:", e);
    }
  }

  async function checkLlmStatus() {
    try {
      llmStatus = await getEngineStatus();
    } catch (e) {
      console.error("Failed to check LLM status:", e);
    }
  }

  async function loadScores() {
    try {
      loadingScores = true;
      scores = await listScoresForSession(sessionId);
    } catch (e) {
      console.error("Failed to load scores:", e);
    } finally {
      loadingScores = false;
    }
  }

  async function handleUpdate() {
    if (!session) return;

    try {
      isSaving = true;
      error = null;
      const updateData: UpdateSession = {
        notes: editedNotes,
        duration_minutes: editedDuration,
        session_type: editedSessionType,
        session_date: editedSessionDate,
        clinical_summary: editableSummary || undefined,
      };
      session = await updateSession(sessionId, updateData);
      editableSummary = session.clinical_summary || "";
      isEditing = false;
      showSummaryEditor = false;
    } catch (e) {
      error = parseError(e);
      console.error("Failed to update session:", e);
    } finally {
      isSaving = false;
    }
  }

  async function handleDelete() {
    try {
      isDeleting = true;
      error = null;
      await deleteSession(sessionId);
      goto(`/patients/${patientId}/sessions`);
    } catch (e) {
      error = parseError(e);
      console.error("Failed to delete session:", e);
      isDeleting = false;
      showDeleteConfirm = false;
    }
  }

  async function handleSaveScore(input: CreateOutcomeScore | { id: string; update: UpdateOutcomeScore }) {
    try {
      savingScore = true;
      error = null;
      if ('id' in input) {
        await updateOutcomeScore(input.id, input.update);
      } else {
        await createOutcomeScore(input);
      }
      await loadScores();
      showAddForm = false;
      editingScore = null;
    } catch (e) {
      error = parseError(e);
      console.error("Failed to save score:", e);
    } finally {
      savingScore = false;
    }
  }

  async function handleDeleteScore(id: string) {
    try {
      error = null;
      await deleteOutcomeScore(id);
      await loadScores();
    } catch (e) {
      error = parseError(e);
      console.error("Failed to delete score:", e);
    }
  }

  function handleEditScore(score: OutcomeScore) {
    editingScore = score;
    showAddForm = false;
  }

  function handleCancelEditScore() {
    editingScore = null;
    showAddForm = false;
  }

  async function generateSummary() {
    if (!session || !patient) return;

    if (!llmStatus?.is_loaded) {
      error = {
        code: "LLM_ERROR",
        message: "LLM model not loaded. Please configure the model in Settings.",
        ref: "LLM_NOT_LOADED",
      };
      return;
    }

    try {
      if (unlistenChunk) { unlistenChunk(); unlistenChunk = null; }
      if (unlistenDone) { unlistenDone(); unlistenDone = null; }

      isGenerating = true;
      error = null;
      generatedSummary = "";

      const activeDiagnoses = diagnoses
        .filter((d) => d.status === "active")
        .map((d) => `${d.icd10_code}: ${d.description}`)
        .join("\n");

      const patientContext = `
Patient: ${patient.first_name} ${patient.last_name}
Geburtsdatum: ${patient.date_of_birth}
${activeDiagnoses ? `Aktive Diagnosen:\n${activeDiagnoses}` : ""}
      `.trim();

      unlistenChunk = await listen<string>("session-summary-chunk", (event) => {
        generatedSummary += event.payload;
      });

      unlistenDone = await listen("session-summary-done", () => {
        isGenerating = false;
        editableSummary = generatedSummary;
        showSummaryEditor = true;
        if (unlistenChunk) { unlistenChunk(); unlistenChunk = null; }
        if (unlistenDone) { unlistenDone(); unlistenDone = null; }
      });

      await invoke("generate_session_summary", {
        patientContext,
        sessionNotes: editedNotes || session.notes || "",
        systemPrompt: null,
      });
    } catch (e) {
      error = parseError(e);
      isGenerating = false;
      if (unlistenChunk) { unlistenChunk(); unlistenChunk = null; }
      if (unlistenDone) { unlistenDone(); unlistenDone = null; }
    }
  }

  function formatDate(dateStr: string): string {
    try {
      const date = new Date(dateStr);
      return date.toLocaleDateString("de-CH", {
        year: "numeric",
        month: "2-digit",
        day: "2-digit",
      });
    } catch {
      return dateStr;
    }
  }

  function formatDateTime(dateStr: string): string {
    try {
      const date = new Date(dateStr);
      return date.toLocaleString("de-CH", {
        year: "numeric",
        month: "2-digit",
        day: "2-digit",
        hour: "2-digit",
        minute: "2-digit",
      });
    } catch {
      return dateStr;
    }
  }
</script>

<div class="p-8">
  <div class="max-w-4xl mx-auto">
    {#if isLoading}
      <div class="flex justify-center items-center py-12">
        <div class="animate-spin rounded-full h-8 w-8 border-b-2 border-blue-500" />
      </div>
    {:else if !session}
      <div class="text-center py-12">
        <p class="text-gray-500 dark:text-gray-400">{$t('common.notFound')}</p>
      </div>
    {:else}
      <div class="mb-6">
        <button
          onclick={() => goto(`/patients/${patientId}/sessions`)}
          class="text-blue-600 dark:text-blue-400 hover:underline"
        >
          ← {$t('common.back')}
        </button>
      </div>

      {#if error}
        <div class="mb-6">
          <ErrorDisplay {error} />
        </div>
      {/if}

      <!-- Session Detail Card -->
      <div class="bg-white dark:bg-gray-800 rounded-lg shadow-md p-6 mb-6">
        <div class="flex justify-between items-start mb-6">
          <div>
            <h1 class="text-2xl font-bold text-gray-900 dark:text-gray-100">
              {session.session_type}
            </h1>
            <p class="text-gray-500 dark:text-gray-400 mt-1">
              {formatDate(session.session_date)}
              {#if session.duration_minutes}
                • {session.duration_minutes} {$t('sessions.duration')}
              {/if}
            </p>
            {#if patient}
              <p class="text-sm text-gray-500 dark:text-gray-400 mt-1">
                {$t('common.patient')}: {patient.first_name} {patient.last_name}
              </p>
            {/if}
          </div>
          <div class="flex space-x-2">
            {#if !isEditing}
              <button
                onclick={() => (isEditing = true)}
                class="px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 transition-colors"
              >
                {$t('common.edit')}
              </button>
            {/if}
            <button
              onclick={() => (showDeleteConfirm = true)}
              class="px-4 py-2 bg-red-600 text-white rounded-lg hover:bg-red-700 transition-colors"
              disabled={isDeleting}
            >
              {isDeleting ? $t('common.deleting') : $t('common.delete')}
            </button>
          </div>
        </div>

        {#if isEditing}
          <div class="space-y-4 mb-6">
            <div>
              <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
                {$t('sessions.sessionType')}
              </label>
              <input
                type="text"
                bind:value={editedSessionType}
                class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-gray-100"
              />
            </div>
            <div>
              <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
                {$t('sessions.date')}
              </label>
              <input
                type="date"
                bind:value={editedSessionDate}
                class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-gray-100"
              />
            </div>
            <div>
              <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
                {$t('sessions.duration')}
              </label>
              <input
                type="number"
                bind:value={editedDuration}
                class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-gray-100"
              />
            </div>
          </div>
        {/if}

        <!-- Notes -->
        <div class="mb-6">
          <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
            {$t('sessions.notes')}
          </label>
          {#if isEditing}
            <textarea
              bind:value={editedNotes}
              rows="10"
              class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-gray-100 font-mono text-sm"
              placeholder={$t('sessions.notesPlaceholder')}
            />
          {:else}
            <div class="bg-gray-50 dark:bg-gray-900 rounded-lg p-4 border border-gray-200 dark:border-gray-700">
              {#if session.notes}
                <pre class="whitespace-pre-wrap font-sans text-gray-900 dark:text-gray-100">{session.notes}</pre>
              {:else}
                <p class="text-gray-400 dark:text-gray-500 italic">{$t('sessions.noNotes')}</p>
              {/if}
            </div>
          {/if}
        </div>

        <!-- Clinical Summary -->
        <div class="border-t border-gray-200 dark:border-gray-700 pt-6 mb-6">
          <div class="flex justify-between items-center mb-4">
            <h2 class="text-lg font-semibold text-gray-900 dark:text-gray-100">
              {$t('sessions.clinicalSummary')}
            </h2>
            {#if !isGenerating && llmStatus?.is_loaded}
              <button
                onclick={generateSummary}
                class="px-4 py-2 bg-green-600 text-white rounded-lg hover:bg-green-700 transition-colors"
                disabled={isGenerating || !session.notes}
              >
                {$t('sessions.generateSummary')}
              </button>
            {/if}
          </div>

          {#if isGenerating}
            <ReportStream content={generatedSummary} isStreaming={isGenerating} />
          {:else if showSummaryEditor || editableSummary}
            <div class="space-y-4">
              <textarea
                bind:value={editableSummary}
                rows="15"
                class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-gray-100 font-sans text-sm"
                placeholder={$t('sessions.clinicalSummaryPlaceholder')}
              />
            </div>
          {:else if session.clinical_summary}
            <div class="bg-gray-50 dark:bg-gray-900 rounded-lg p-4 border border-gray-200 dark:border-gray-700">
              <pre class="whitespace-pre-wrap font-sans text-gray-900 dark:text-gray-100">{session.clinical_summary}</pre>
            </div>
          {:else}
            <div class="bg-gray-50 dark:bg-gray-900 rounded-lg p-4 border border-gray-200 dark:border-gray-700">
              <p class="text-gray-400 dark:text-gray-500 italic">{$t('sessions.noSummary')}</p>
            </div>
          {/if}
        </div>

        {#if isEditing || showSummaryEditor}
          <div class="flex justify-end space-x-3 pt-4 border-t border-gray-200 dark:border-gray-700">
            <button
              onclick={() => {
                isEditing = false;
                showSummaryEditor = false;
                editedNotes = session?.notes || "";
                editedDuration = session?.duration_minutes || null;
                editedSessionType = session?.session_type || "";
                editedSessionDate = session?.session_date || "";
                editableSummary = session?.clinical_summary || "";
              }}
              class="px-4 py-2 border border-gray-300 dark:border-gray-600 text-gray-700 dark:text-gray-300 rounded-lg hover:bg-gray-50 dark:hover:bg-gray-700 transition-colors"
              disabled={isSaving}
            >
              {$t('common.cancel')}
            </button>
            <button
              onclick={handleUpdate}
              class="px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 transition-colors"
              disabled={isSaving}
            >
              {isSaving ? $t('common.saving') : $t('common.save')}
            </button>
          </div>
        {/if}

        <div class="mt-6 pt-6 border-t border-gray-200 dark:border-gray-700 text-sm text-gray-500 dark:text-gray-400">
          <p>{$t('common.createdAt')}: {formatDateTime(session.created_at)}</p>
          <p>{$t('common.updatedAt')}: {formatDateTime(session.updated_at)}</p>
        </div>
      </div>

      <!-- Outcome Scores Section -->
      <div class="border-t border-gray-200 dark:border-gray-700 pt-6">
        <div class="flex justify-between items-center mb-4">
          <h2 class="text-xl font-semibold text-gray-900 dark:text-gray-100">{$t('outcomeScores.title')}</h2>
          {#if !showAddForm && !editingScore}
            <button
              onclick={() => (showAddForm = true)}
              class="px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 transition-colors"
            >
              + {$t('outcomeScores.newScore')}
            </button>
          {/if}
        </div>

        {#if showAddForm}
          <div class="mb-6 p-6 bg-white dark:bg-gray-800 rounded-lg border border-gray-200 dark:border-gray-700">
            <h3 class="text-lg font-medium text-gray-900 dark:text-gray-100 mb-4">{$t('outcomeScores.newScore')}</h3>
            <OutcomeScoreForm
              sessionId={sessionId}
              onSave={handleSaveScore}
              onCancel={handleCancelEditScore}
            />
          </div>
        {/if}

        {#if editingScore}
          <div class="mb-6 p-6 bg-white dark:bg-gray-800 rounded-lg border border-gray-200 dark:border-gray-700">
            <h3 class="text-lg font-medium text-gray-900 dark:text-gray-100 mb-4">{$t('common.edit')}</h3>
            <OutcomeScoreForm
              outcomeScore={editingScore}
              onSave={handleSaveScore}
              onCancel={handleCancelEditScore}
            />
          </div>
        {/if}

        {#if loadingScores}
          <div class="flex justify-center items-center py-12">
            <div class="text-gray-500 dark:text-gray-400">{$t('common.loading')}</div>
          </div>
        {:else if scores.length === 0 && !showAddForm}
          <div class="text-center py-12 bg-gray-50 dark:bg-gray-800 rounded-lg border border-gray-200 dark:border-gray-700">
            <p class="text-gray-500 dark:text-gray-400 mb-4">{$t('outcomeScores.noScores')}</p>
            <button
              onclick={() => (showAddForm = true)}
              class="px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 transition-colors"
            >
              {$t('outcomeScores.newScore')}
            </button>
          </div>
        {:else}
          <div class="grid gap-4">
            {#each scores as score (score.id)}
              <OutcomeScoreCard
                outcomeScore={score}
                onEdit={() => handleEditScore(score)}
                onDelete={() => handleDeleteScore(score.id)}
              />
            {/each}
          </div>
        {/if}
      </div>
    {/if}
  </div>
</div>

{#if showDeleteConfirm}
  <div class="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50">
    <div class="bg-white dark:bg-gray-800 rounded-lg p-6 max-w-md w-full mx-4">
      <h3 class="text-lg font-semibold text-gray-900 dark:text-gray-100 mb-4">
        {$t('sessions.confirmDelete')}
      </h3>
      <p class="text-gray-600 dark:text-gray-400 mb-6">
        {$t('sessions.confirmDeleteMessage')}
      </p>
      <div class="flex justify-end space-x-3">
        <button
          onclick={() => (showDeleteConfirm = false)}
          class="px-4 py-2 border border-gray-300 dark:border-gray-600 text-gray-700 dark:text-gray-300 rounded-lg hover:bg-gray-50 dark:hover:bg-gray-700 transition-colors"
          disabled={isDeleting}
        >
          {$t('common.cancel')}
        </button>
        <button
          onclick={handleDelete}
          class="px-4 py-2 bg-red-600 text-white rounded-lg hover:bg-red-700 transition-colors"
          disabled={isDeleting}
        >
          {isDeleting ? $t('common.deleting') : $t('common.delete')}
        </button>
      </div>
    </div>
  </div>
{/if}
