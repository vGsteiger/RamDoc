<script lang="ts">
  import { sessionTypeLabel } from '$lib/translations/labels';
  import { page } from '$app/state';
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
  import DictationButton from '$lib/components/DictationButton.svelte';
  import { insertDictation } from '$lib/dictation';
  import { t } from '$lib/translations';
  import { fmt } from '$lib/format';
  import { ChevronLeft, Pencil, Plus, Trash2 } from 'lucide-svelte';
  import {
    Button,
    ConfirmDialog,
    Dialog,
    EmptyState,
    Field,
    Input,
    Markdown,
    Menu,
    PageShell,
    SectionHeader,
    Select,
    Spinner,
    Textarea,
  } from '$lib/components/ui';

  const patientId = $derived(page.params.id!);
  const sessionId = $derived(page.params.sessionId!);

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
  let notesEl = $state<HTMLTextAreaElement>();
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
        patientId,
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

  // Persisted (German) session types offered in the editor; an older free-text
  // value that is not in the list stays selectable, so editing never changes it.
  const SESSION_TYPES = [
    'Erstgespräch',
    'Verlaufskontrolle',
    'Krisenintervention',
    'Psychotherapie',
    'Medikamentenanpassung',
    'Andere',
  ];
  let sessionTypeOptions = $derived(
    editedSessionType && !SESSION_TYPES.includes(editedSessionType)
      ? [editedSessionType, ...SESSION_TYPES]
      : SESSION_TYPES
  );
  let scoreFormOpen = $derived(showAddForm || editingScore !== null);
  let deletingScore = $state<OutcomeScore | null>(null);
  let showDeleteScore = $state(false);

  function askDeleteScore(score: OutcomeScore) {
    deletingScore = score;
    showDeleteScore = true;
  }

  async function confirmDeleteScore() {
    if (deletingScore) await handleDeleteScore(deletingScore.id);
  }

  function cancelEdit() {
    isEditing = false;
    showSummaryEditor = false;
    editedNotes = session?.notes || '';
    editedDuration = session?.duration_minutes || null;
    editedSessionType = session?.session_type || '';
    editedSessionDate = session?.session_date || '';
    editableSummary = session?.clinical_summary || '';
  }
</script>

<PageShell>
  <div>
    {#if isLoading}
      <div class="flex justify-center py-12"><Spinner label={$t('common.loading')} /></div>
    {:else if !session}
      <EmptyState title={$t('common.notFound')} />
    {:else}
      <Button size="sm" variant="ghost" href={`/patients/${patientId}/sessions`} class="-ml-2 mb-2">
        <ChevronLeft size={14} aria-hidden="true" />
        {$t('sessions.title')}
      </Button>

      <SectionHeader
        title={$sessionTypeLabel(session.session_type)}
        description={[
          $fmt.date(session.session_date),
          session.duration_minutes ? $fmt.duration(session.duration_minutes) : '',
        ]
          .filter(Boolean)
          .join(' · ')}
      >
        {#snippet actions()}
          {#if !isEditing}
            <Button size="sm" onclick={() => (isEditing = true)}>
              <Pencil size={14} aria-hidden="true" />
              {$t('common.edit')}
            </Button>
          {/if}
          <Menu
            size="sm"
            items={[
              {
                label: $t('sessions.deleteSession'),
                icon: Trash2,
                danger: true,
                disabled: isDeleting,
                onSelect: () => (showDeleteConfirm = true),
              },
            ]}
          />
        {/snippet}
      </SectionHeader>

      {#if error}
        <div class="mb-4">
          <ErrorDisplay {error} />
        </div>
      {/if}

      {#if isEditing}
        <div class="mb-4 grid grid-cols-1 gap-4 sm:grid-cols-3">
          <Field label={$t('sessions.sessionType')} for="session-type" class="sm:col-span-3">
            <Select id="session-type" bind:value={editedSessionType}>
              {#each sessionTypeOptions as type (type)}
                <option value={type}>{$sessionTypeLabel(type)}</option>
              {/each}
            </Select>
          </Field>
          <Field label={$t('sessions.date')} for="session-date">
            <Input id="session-date" type="date" bind:value={editedSessionDate} />
          </Field>
          <Field label={$t('sessions.duration')} for="session-duration">
            <Input
              id="session-duration"
              type="number"
              min="0"
              step="5"
              bind:value={editedDuration}
            />
          </Field>
        </div>
      {/if}

      <!-- Notes -->
      <section class="mb-6" aria-labelledby="session-notes-title">
        <div class="mb-2 flex items-center justify-between gap-2">
          {#if isEditing}
            <label id="session-notes-title" for="session-notes" class="text-heading text-fg">
              {$t('sessions.notes')}
            </label>
            <DictationButton
              onTranscript={(text) => (editedNotes = insertDictation(notesEl, editedNotes, text))}
            />
          {:else}
            <h3 id="session-notes-title" class="text-heading text-fg">{$t('sessions.notes')}</h3>
          {/if}
        </div>
        {#if isEditing}
          <!-- Raw textarea on purpose: dictation inserts at the caret, so it needs
               the element (bind:this), which the Textarea primitive does not expose. -->
          <textarea
            id="session-notes"
            bind:this={notesEl}
            bind:value={editedNotes}
            rows="12"
            class="w-full resize-y rounded-control border border-line bg-surface-raised px-2.5 py-2 text-body text-fg transition-colors duration-150 ease-standard focus:border-focus focus:outline-none focus:ring-2 focus:ring-focus/25 focus-visible:outline-none"
            placeholder={$t('sessions.notesPlaceholder')}></textarea>
        {:else if session.notes}
          <p class="whitespace-pre-wrap text-body text-fg">{session.notes}</p>
        {:else}
          <p class="text-body text-fg-subtle">{$t('sessions.noNotes')}</p>
        {/if}
      </section>

      <!-- Clinical summary -->
      <section class="mb-6 border-t border-line-subtle pt-4">
        <SectionHeader title={$t('sessions.clinicalSummary')} level={3} id="summary-title">
          {#snippet actions()}
            {#if !isGenerating && llmStatus?.is_loaded}
              <ThinkingEffortSelect />
              <Button
                size="sm"
                onclick={generateSummary}
                disabled={isGenerating || !editedNotes || editedNotes.trim().length === 0}
              >
                {$t('sessions.generateSummary')}
              </Button>
            {/if}
          {/snippet}
        </SectionHeader>

        {#if isGenerating}
          <ReportStream content={generatedSummary} isStreaming={isGenerating} />
        {:else if isEditing || showSummaryEditor}
          <Textarea
            bind:value={editableSummary}
            rows={15}
            aria-labelledby="summary-title"
            placeholder={$t('sessions.clinicalSummaryPlaceholder')}
          />
        {:else if session.clinical_summary}
          <Markdown source={session.clinical_summary} class="text-body text-fg" />
        {:else}
          <p class="text-body text-fg-subtle">{$t('sessions.noSummary')}</p>
        {/if}
      </section>

      {#if isEditing || showSummaryEditor}
        <div class="mb-6 flex justify-end gap-2 border-t border-line-subtle pt-4">
          <Button onclick={cancelEdit} disabled={isSaving}>{$t('common.cancel')}</Button>
          <Button variant="primary" onclick={handleUpdate} loading={isSaving}>
            {$t('common.save')}
          </Button>
        </div>
      {/if}

      <p class="mb-6 text-caption text-fg-subtle" data-numeric>
        {$t('common.createdAt')}: {$fmt.dateTime(session.created_at)}
        <span aria-hidden="true" class="px-1">·</span>
        {$t('common.updatedAt')}: {$fmt.dateTime(session.updated_at)}
      </p>

      <!-- Outcome scores -->
      <section class="border-t border-line-subtle pt-4">
        <SectionHeader title={$t('outcomeScores.title')} level={3}>
          {#snippet actions()}
            <Button
              size="sm"
              onclick={() => {
                editingScore = null;
                showAddForm = true;
              }}
            >
              <Plus size={14} aria-hidden="true" />
              {$t('outcomeScores.newScore')}
            </Button>
          {/snippet}
        </SectionHeader>

        {#if loadingScores}
          <Spinner label={$t('common.loading')} />
        {:else if scores.length === 0}
          <p class="text-body text-fg-muted">{$t('outcomeScores.noScores')}</p>
        {:else}
          <div class="grid gap-2">
            {#each scores as score (score.id)}
              <OutcomeScoreCard
                outcomeScore={score}
                onEdit={() => handleEditScore(score)}
                onDelete={() => askDeleteScore(score)}
              />
            {/each}
          </div>
        {/if}
      </section>
    {/if}
  </div>
</PageShell>

<Dialog
  open={scoreFormOpen}
  title={editingScore ? $t('outcomeScores.editScore') : $t('outcomeScores.newScore')}
  onClose={handleCancelEditScore}
>
  <!-- Keyed so switching to a different score remounts the form: it
       snapshots its props and would otherwise keep the previous values. -->
  {#key editingScore?.id ?? 'new'}
    {#if editingScore}
      <OutcomeScoreForm
        outcomeScore={editingScore}
        onSave={handleSaveScore}
        onCancel={handleCancelEditScore}
      />
    {:else}
      <OutcomeScoreForm {sessionId} onSave={handleSaveScore} onCancel={handleCancelEditScore} />
    {/if}
  {/key}
</Dialog>

<ConfirmDialog
  bind:open={showDeleteConfirm}
  tone="danger"
  title={$t('sessions.confirmDelete')}
  message={session
    ? $t('sessions.confirmDeleteNamed')
        .replace('{type}', $sessionTypeLabel(session.session_type))
        .replace('{date}', $fmt.date(session.session_date))
    : $t('sessions.confirmDeleteMessage')}
  onConfirm={handleDelete}
/>

<ConfirmDialog
  bind:open={showDeleteScore}
  tone="danger"
  title={$t('outcomeScores.deleteScore')}
  message={deletingScore
    ? $t('outcomeScores.confirmDeleteNamed')
        .replace('{scale}', deletingScore.scale_type)
        .replace('{date}', $fmt.date(deletingScore.administered_at))
    : ''}
  onConfirm={confirmDeleteScore}
/>
