<script lang="ts">
  import { onMount } from 'svelte';
  import { page } from '$app/stores';
  import { goto } from '$app/navigation';
  import {
    getSession,
    listScoresForSession,
    createOutcomeScore,
    updateOutcomeScore,
    deleteOutcomeScore,
    type Session,
    type OutcomeScore,
    type CreateOutcomeScore,
    type UpdateOutcomeScore
  } from '$lib/api';
  import OutcomeScoreCard from '$lib/components/OutcomeScoreCard.svelte';
  import OutcomeScoreForm from '$lib/components/OutcomeScoreForm.svelte';
  import { t } from '$lib/translations';

  const patientId = $derived($page.params.id);
  const sessionId = $derived($page.params.sessionId);

  let session = $state<Session | null>(null);
  let scores = $state<OutcomeScore[]>([]);
  let loading = $state(true);
  let loadingScores = $state(false);
  let error = $state<string | null>(null);
  let showAddForm = $state(false);
  let editingScore = $state<OutcomeScore | null>(null);
  let saving = $state(false);

  onMount(async () => {
    await Promise.all([loadSession(), loadScores()]);
  });

  async function loadSession() {
    try {
      loading = true;
      error = null;
      session = await getSession(sessionId);
    } catch (err) {
      error = $t('common.error') + ': ' + (err instanceof Error ? err.message : String(err));
      console.error('Failed to load session:', err);
    } finally {
      loading = false;
    }
  }

  async function loadScores() {
    try {
      loadingScores = true;
      error = null;
      scores = await listScoresForSession(sessionId);
    } catch (err) {
      error = $t('common.error') + ': ' + (err instanceof Error ? err.message : String(err));
      console.error('Failed to load scores:', err);
    } finally {
      loadingScores = false;
    }
  }

  async function handleSave(input: CreateOutcomeScore | { id: string; update: UpdateOutcomeScore }) {
    try {
      saving = true;
      error = null;

      if ('id' in input) {
        // Update existing score
        await updateOutcomeScore(input.id, input.update);
      } else {
        // Create new score
        await createOutcomeScore(input);
      }

      await loadScores();
      showAddForm = false;
      editingScore = null;
    } catch (err) {
      error = $t('common.error') + ': ' + (err instanceof Error ? err.message : String(err));
      console.error('Failed to save score:', err);
    } finally {
      saving = false;
    }
  }

  async function handleDelete(id: string) {
    if (!confirm($t('common.confirm') + '?')) {
      return;
    }

    try {
      error = null;
      await deleteOutcomeScore(id);
      await loadScores();
    } catch (err) {
      error = $t('common.error') + ': ' + (err instanceof Error ? err.message : String(err));
      console.error('Failed to delete score:', err);
    }
  }

  function handleEdit(score: OutcomeScore) {
    editingScore = score;
    showAddForm = false;
  }

  function handleCancelEdit() {
    editingScore = null;
    showAddForm = false;
  }

  function formatDate(dateStr: string): string {
    try {
      const date = new Date(dateStr);
      return date.toLocaleDateString('de-CH', {
        year: 'numeric',
        month: 'long',
        day: 'numeric'
      });
    } catch {
      return dateStr;
    }
  }
</script>

<div class="p-8">
  {#if loading}
    <div class="flex justify-center items-center py-12">
      <div class="text-gray-500 dark:text-gray-400">{$t('common.loading')}</div>
    </div>
  {:else if error && !session}
    <div class="bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 text-red-600 dark:text-red-400 p-4 rounded-lg">
      {error}
    </div>
  {:else if session}
    <!-- Session Header -->
    <div class="mb-6">
      <button
        onclick={() => goto(`/patients/${patientId}/sessions`)}
        class="text-blue-600 dark:text-blue-400 hover:underline mb-2 flex items-center gap-1"
      >
        ← {$t('common.back')}
      </button>
      <h1 class="text-2xl font-bold text-gray-900 dark:text-gray-100">
        {session.session_type} - {formatDate(session.session_date)}
      </h1>
      {#if session.duration_minutes}
        <p class="text-gray-500 dark:text-gray-400">{session.duration_minutes} {$t('sessions.duration')}</p>
      {/if}
      {#if session.notes}
        <div class="mt-4 p-4 bg-gray-50 dark:bg-gray-800 rounded-lg border border-gray-200 dark:border-gray-700">
          <h3 class="text-sm font-medium text-gray-500 dark:text-gray-400 mb-2">{$t('sessions.notes')}</h3>
          <p class="text-gray-900 dark:text-gray-100 whitespace-pre-line">{session.notes}</p>
        </div>
      {/if}
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

      {#if error}
        <div class="bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 text-red-600 dark:text-red-400 p-4 rounded-lg mb-4">
          {error}
        </div>
      {/if}

      {#if showAddForm}
        <div class="mb-6 p-6 bg-white dark:bg-gray-800 rounded-lg border border-gray-200 dark:border-gray-700">
          <h3 class="text-lg font-medium text-gray-900 dark:text-gray-100 mb-4">{$t('outcomeScores.newScore')}</h3>
          <OutcomeScoreForm
            sessionId={sessionId}
            onSave={handleSave}
            onCancel={handleCancelEdit}
          />
        </div>
      {/if}

      {#if editingScore}
        <div class="mb-6 p-6 bg-white dark:bg-gray-800 rounded-lg border border-gray-200 dark:border-gray-700">
          <h3 class="text-lg font-medium text-gray-900 dark:text-gray-100 mb-4">{$t('common.edit')}</h3>
          <OutcomeScoreForm
            outcomeScore={editingScore}
            onSave={handleSave}
            onCancel={handleCancelEdit}
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
              onEdit={() => handleEdit(score)}
              onDelete={() => handleDelete(score.id)}
            />
          {/each}
        </div>
      {/if}
    </div>
  {/if}
</div>
