<script lang="ts">
  import { sessionTypeLabel } from '$lib/translations/labels';
  import { page } from '$app/stores';
  import { goto } from '$app/navigation';
  import { onMount, onDestroy } from 'svelte';
  import { listen, type UnlistenFn } from '@tauri-apps/api/event';
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
  } from '$lib/api';
  import { invoke } from '@tauri-apps/api/core';
  import ReportStream from '$lib/components/ReportStream.svelte';
  import ThinkingEffortSelect from '$lib/components/ThinkingEffortSelect.svelte';
  import { thinkingEffort } from '$lib/stores/thinking';
  import { get } from 'svelte/store';
  import { stripThinkTags } from '$lib/llm/strip-think';
  import ErrorDisplay from '$lib/components/ErrorDisplay.svelte';
  import OutcomeScoreCard from '$lib/components/OutcomeScoreCard.svelte';
  import OutcomeScoreForm from '$lib/components/OutcomeScoreForm.svelte';
  import { t } from '$lib/translations';

  const patientId = $derived($page.params.id!);
  const sessionId = $derived($page.params.sessionId!);

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

  let editedNotes = $state('');
  let editedDuration = $state<number | null>(null);
  let editedSessionType = $state('');
  let editedSessionDate = $state('');

  let generatedSummary = $state('');
  let editableSummary = $state('');
  let showSummaryEditor = $state(false);

  let unlistenChunk: UnlistenFn | null = null;
  let unlistenDone: UnlistenFn | null = null;

  onMount(async () => {
    await Promise.all([
      loadSession(),
      loadPatient(),
      loadDiagnoses(),
      checkLlmStatus(),
      loadScores(),
    ]);
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
      editedNotes = session.notes || '';
      editedDuration = session.duration_minutes;
      editedSessionType = session.session_type;
      editedSessionDate = session.session_date;
      editableSummary = session.clinical_summary || '';
    } catch (e) {
      error = parseError(e);
      console.error('Failed to load session:', e);
    } finally {
      isLoading = false;
    }
  }

  async function loadPatient() {
    try {
      patient = await getPatient(patientId);
    } catch (e) {
      console.error('Failed to load patient:', e);
    }
  }

  async function loadDiagnoses() {
    try {
      diagnoses = await listDiagnosesForPatient(patientId);
    } catch (e) {
      console.error('Failed to load diagnoses:', e);
    }
  }

  async function checkLlmStatus() {
    try {
      llmStatus = await getEngineStatus();
    } catch (e) {
      console.error('Failed to check LLM status:', e);
    }
  }

  async function loadScores() {
    try {
      loadingScores = true;
      error = null;
      scores = await listScoresForSession(sessionId);
    } catch (e) {
      error = parseError(e);
      console.error('Failed to load scores:', e);
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
        duration_minutes: editedDuration ?? undefined,
        session_type: editedSessionType,
        session_date: editedSessionDate,
        clinical_summary: editableSummary ?? '',
      };
      session = await updateSession(sessionId, updateData);
      editableSummary = session.clinical_summary || '';
      isEditing = false;
      showSummaryEditor = false;
    } catch (e) {
      error = parseError(e);
      console.error('Failed to update session:', e);
    } finally {
      isSaving = false;
    }
  }

  async function handleDelete() {
    try {
      isDeleting = true;
      error = null;
      await deleteSession(sessionId);
      showDeleteConfirm = false;
      await goto(`/patients/${patientId}/sessions`);
    } catch (e) {
      error = parseError(e);
      console.error('Failed to delete session:', e);
    } finally {
      isDeleting = false;
      showDeleteConfirm = false;
    }
  }

  async function handleSaveScore(
    input: CreateOutcomeScore | { id: string; update: UpdateOutcomeScore }
  ) {
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
      console.error('Failed to save score:', e);
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
      console.error('Failed to delete score:', e);
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
        code: 'LLM_ERROR',
        message: $t('sessions.llmModelNotLoaded'),
        ref: 'LLM_NOT_LOADED',
      };
      return;
    }

    try {
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
      generatedSummary = '';

      const activeDiagnoses = diagnoses
        .filter((d) => d.status === 'active')
        .map((d) => `${d.icd10_code}: ${d.description}`)
        .join('\n');

      const patientContext = `
Patient: ${patient.first_name} ${patient.last_name}
Geburtsdatum: ${patient.date_of_birth}
${activeDiagnoses ? `Aktive Diagnosen:\n${activeDiagnoses}` : ''}
      `.trim();

      unlistenChunk = await listen<string>('session-summary-chunk', (event) => {
        generatedSummary += event.payload;
      });

      unlistenDone = await listen('session-summary-done', () => {
        isGenerating = false;
        editableSummary = stripThinkTags(generatedSummary);
        showSummaryEditor = true;
        if (unlistenChunk) {
          unlistenChunk();
          unlistenChunk = null;
        }
        if (unlistenDone) {
          unlistenDone();
          unlistenDone = null;
        }
      });

      await invoke('generate_session_summary', {
        patientContext,
        sessionNotes: editedNotes || session.notes || '',
        systemPrompt: null,
        thinkingEffort: get(thinkingEffort),
      });
    } catch (e) {
      error = parseError(e);
      isGenerating = false;
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

  function formatDate(dateStr: string): string {
    try {
      const date = new Date(dateStr);
      return date.toLocaleDateString('de-CH', {
        year: 'numeric',
        month: '2-digit',
        day: '2-digit',
      });
    } catch {
      return dateStr;
    }
  }

  function formatDateTime(dateStr: string): string {
    try {
      const date = new Date(dateStr);
      return date.toLocaleString('de-CH', {
        year: 'numeric',
        month: '2-digit',
        day: '2-digit',
        hour: '2-digit',
        minute: '2-digit',
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
        <div class="animate-spin rounded-full h-8 w-8 border-b-2 border-accent"></div>
      </div>
    {:else if !session}
      <div class="text-center py-12">
        <p class="text-fg-muted">{$t('common.notFound')}</p>
      </div>
    {:else}
      <div class="mb-6">
        <button
          onclick={() => goto(`/patients/${patientId}/sessions`)}
          class="text-accent-fg hover:underline"
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
      <div class="bg-surface-raised rounded-card shadow-popover p-6 mb-6">
        <div class="flex justify-between items-start mb-6">
          <div>
            <h1 class="text-display font-semibold text-fg">
              {$sessionTypeLabel(session.session_type)}
            </h1>
            <p class="text-fg-muted mt-1">
              {formatDate(session.session_date)}
              {#if session.duration_minutes}
                • {session.duration_minutes} {$t('sessions.duration')}
              {/if}
            </p>
            {#if patient}
              <p class="text-body text-fg-muted mt-1">
                {$t('common.patient')}: {patient.first_name}
                {patient.last_name}
              </p>
            {/if}
          </div>
          <div class="flex space-x-2">
            {#if !isEditing}
              <button
                onclick={() => (isEditing = true)}
                class="h-8 px-3 bg-accent text-on-accent rounded-control hover:bg-accent-hover transition-colors"
              >
                {$t('common.edit')}
              </button>
            {/if}
            <button
              onclick={() => (showDeleteConfirm = true)}
              class="h-8 px-3 bg-danger text-on-danger rounded-control hover:bg-danger-hover transition-colors"
              disabled={isDeleting}
            >
              {isDeleting ? $t('common.deleting') : $t('common.delete')}
            </button>
          </div>
        </div>

        {#if isEditing}
          <div class="space-y-4 mb-6">
            <div>
              <label for="session-type" class="block text-body font-medium text-fg-muted mb-1">
                {$t('sessions.sessionType')}
              </label>
              <input
                id="session-type"
                type="text"
                bind:value={editedSessionType}
                class="w-full px-3 py-2 border border-line rounded-control bg-surface-raised text-fg"
              />
            </div>
            <div>
              <label for="session-date" class="block text-body font-medium text-fg-muted mb-1">
                {$t('sessions.date')}
              </label>
              <input
                id="session-date"
                type="date"
                bind:value={editedSessionDate}
                class="w-full px-3 py-2 border border-line rounded-control bg-surface-raised text-fg"
              />
            </div>
            <div>
              <label for="session-duration" class="block text-body font-medium text-fg-muted mb-1">
                {$t('sessions.duration')}
              </label>
              <input
                id="session-duration"
                type="number"
                bind:value={editedDuration}
                class="w-full px-3 py-2 border border-line rounded-control bg-surface-raised text-fg"
              />
            </div>
          </div>
        {/if}

        <!-- Notes -->
        <div class="mb-6">
          {#if isEditing}
            <label for="session-notes" class="block text-body font-medium text-fg-muted mb-2">
              {$t('sessions.notes')}
            </label>
            <textarea
              id="session-notes"
              bind:value={editedNotes}
              rows="10"
              class="w-full px-3 py-2 border border-line rounded-control bg-surface-raised text-fg font-mono text-body"
              placeholder={$t('sessions.notesPlaceholder')}></textarea>
          {:else}
            <p class="block text-body font-medium text-fg-muted mb-2">
              {$t('sessions.notes')}
            </p>
            <div class="bg-surface-sunken rounded-card p-4 border border-line">
              {#if session.notes}
                <pre class="whitespace-pre-wrap font-sans text-fg">{session.notes}</pre>
              {:else}
                <p class="text-fg-subtle italic">{$t('sessions.noNotes')}</p>
              {/if}
            </div>
          {/if}
        </div>

        <!-- Clinical Summary -->
        <div class="border-t border-line pt-6 mb-6">
          <div class="flex flex-wrap justify-between items-center gap-3 mb-4">
            <h2 class="text-heading font-semibold text-fg">
              {$t('sessions.clinicalSummary')}
            </h2>
            {#if !isGenerating && llmStatus?.is_loaded}
              <div class="flex flex-wrap items-center gap-3">
                <ThinkingEffortSelect />
                <button
                  onclick={generateSummary}
                  class="h-8 px-3 bg-success text-on-success rounded-control hover:bg-success-hover transition-colors"
                  disabled={isGenerating || !editedNotes || editedNotes.trim().length === 0}
                >
                  {$t('sessions.generateSummary')}
                </button>
              </div>
            {/if}
          </div>

          {#if isGenerating}
            <ReportStream content={generatedSummary} isStreaming={isGenerating} />
          {:else if isEditing || showSummaryEditor}
            <div class="space-y-4">
              <textarea
                bind:value={editableSummary}
                rows="15"
                class="w-full px-3 py-2 border border-line rounded-control bg-surface-raised text-fg font-sans text-body"
                placeholder={$t('sessions.clinicalSummaryPlaceholder')}></textarea>
            </div>
          {:else if session.clinical_summary}
            <div class="bg-surface-sunken rounded-card p-4 border border-line">
              <pre class="whitespace-pre-wrap font-sans text-fg">{session.clinical_summary}</pre>
            </div>
          {:else}
            <div class="bg-surface-sunken rounded-card p-4 border border-line">
              <p class="text-fg-subtle italic">{$t('sessions.noSummary')}</p>
            </div>
          {/if}
        </div>

        {#if isEditing || showSummaryEditor}
          <div class="flex justify-end space-x-3 pt-4 border-t border-line">
            <button
              onclick={() => {
                isEditing = false;
                showSummaryEditor = false;
                editedNotes = session?.notes || '';
                editedDuration = session?.duration_minutes || null;
                editedSessionType = session?.session_type || '';
                editedSessionDate = session?.session_date || '';
                editableSummary = session?.clinical_summary || '';
              }}
              class="h-8 px-3 border border-line text-fg-muted rounded-control hover:bg-surface-hover transition-colors"
              disabled={isSaving}
            >
              {$t('common.cancel')}
            </button>
            <button
              onclick={handleUpdate}
              class="h-8 px-3 bg-accent text-on-accent rounded-control hover:bg-accent-hover transition-colors"
              disabled={isSaving}
            >
              {isSaving ? $t('common.saving') : $t('common.save')}
            </button>
          </div>
        {/if}

        <div class="mt-6 pt-6 border-t border-line text-body text-fg-muted">
          <p>{$t('common.createdAt')}: {formatDateTime(session.created_at)}</p>
          <p>{$t('common.updatedAt')}: {formatDateTime(session.updated_at)}</p>
        </div>
      </div>

      <!-- Outcome Scores Section -->
      <div class="border-t border-line pt-6">
        <div class="flex justify-between items-center mb-4">
          <h2 class="text-title font-semibold text-fg">{$t('outcomeScores.title')}</h2>
          {#if !showAddForm && !editingScore}
            <button
              onclick={() => (showAddForm = true)}
              class="h-8 px-3 bg-accent text-on-accent rounded-control hover:bg-accent-hover transition-colors"
            >
              + {$t('outcomeScores.newScore')}
            </button>
          {/if}
        </div>

        {#if showAddForm}
          <div class="mb-6 p-6 bg-surface-raised rounded-card border border-line">
            <h3 class="text-heading font-medium text-fg mb-4">{$t('outcomeScores.newScore')}</h3>
            <OutcomeScoreForm
              {sessionId}
              onSave={handleSaveScore}
              onCancel={handleCancelEditScore}
            />
          </div>
        {/if}

        {#if editingScore}
          <div class="mb-6 p-6 bg-surface-raised rounded-card border border-line">
            <h3 class="text-heading font-medium text-fg mb-4">{$t('common.edit')}</h3>
            <!-- Keyed so switching to a different score remounts the form: it
                 snapshots its props and would otherwise keep the previous values. -->
            {#key editingScore.id}
              <OutcomeScoreForm
                outcomeScore={editingScore}
                onSave={handleSaveScore}
                onCancel={handleCancelEditScore}
              />
            {/key}
          </div>
        {/if}

        {#if loadingScores}
          <div class="flex justify-center items-center py-12">
            <div class="text-fg-muted">{$t('common.loading')}</div>
          </div>
        {:else if scores.length === 0 && !showAddForm}
          <div class="text-center py-12 bg-surface-sunken rounded-card border border-line">
            <p class="text-fg-muted mb-4">{$t('outcomeScores.noScores')}</p>
            <button
              onclick={() => (showAddForm = true)}
              class="h-8 px-3 bg-accent text-on-accent rounded-control hover:bg-accent-hover transition-colors"
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
  <div class="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
    <div class="bg-surface-raised rounded-card p-6 max-w-md w-full mx-4">
      <h3 class="text-heading font-semibold text-fg mb-4">
        {$t('sessions.confirmDelete')}
      </h3>
      <p class="text-fg-muted mb-6">
        {$t('sessions.confirmDeleteMessage')}
      </p>
      <div class="flex justify-end space-x-3">
        <button
          onclick={() => (showDeleteConfirm = false)}
          class="h-8 px-3 border border-line text-fg-muted rounded-control hover:bg-surface-hover transition-colors"
          disabled={isDeleting}
        >
          {$t('common.cancel')}
        </button>
        <button
          onclick={handleDelete}
          class="h-8 px-3 bg-danger text-on-danger rounded-control hover:bg-danger-hover transition-colors"
          disabled={isDeleting}
        >
          {isDeleting ? $t('common.deleting') : $t('common.delete')}
        </button>
      </div>
    </div>
  </div>
{/if}
