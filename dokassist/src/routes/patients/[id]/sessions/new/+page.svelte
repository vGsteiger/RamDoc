<script lang="ts">
  import { sessionTypeLabel } from '$lib/translations/labels';
  import { page } from '$app/stores';
  import { goto } from '$app/navigation';
  import { createSession, type CreateSession } from '$lib/api';
  import { addToast } from '$lib/stores/toast';
  import { AMDP_CATEGORIES, serializeAMDP, type AMDPCategory } from '$lib/amdp';
  import AMDPForm from '$lib/components/AMDPForm.svelte';
  import { get } from 'svelte/store';
  import { t } from '$lib/translations';

  const patientId = $derived($page.params.id);

  const prefilledDate = $page.url.searchParams.get('date');
  const prefilledTime = $page.url.searchParams.get('time');

  let sessionType = $state('Erstgespräch');
  let sessionDate = $state(prefilledDate ?? new Date().toISOString().split('T')[0]);
  let sessionTime = $state(prefilledTime ?? '');
  let durationMinutes = $state(50);
  let notes = $state('');
  let amdpCategories = $state<AMDPCategory[]>(JSON.parse(JSON.stringify(AMDP_CATEGORIES)));

  let saving = $state(false);
  let error = $state<string | null>(null);
  let showAMDP = $state(false);

  const sessionTypes = [
    'Erstgespräch',
    'Verlaufskontrolle',
    'Krisenintervention',
    'Psychotherapie',
    'Medikamentenanpassung',
    'Andere',
  ];

  function handleAMDPScoreChange(code: string, score: 0 | 1 | 2 | 3) {
    // Find and update the score for the specific item
    amdpCategories = amdpCategories.map((category) => ({
      ...category,
      items: category.items.map((item) => (item.code === code ? { ...item, score } : item)),
    }));
  }

  async function handleSubmit(event: Event) {
    event.preventDefault();

    if (!sessionType.trim() || !notes.trim()) {
      error = get(t)('sessions.requiredFields');
      return;
    }

    try {
      saving = true;
      error = null;

      const input: CreateSession = {
        patient_id: patientId!,
        session_date: sessionDate,
        session_type: sessionType,
        duration_minutes: durationMinutes || undefined,
        scheduled_time: sessionTime ? `${sessionDate}T${sessionTime}:00` : undefined,
        notes,
        amdp_data: showAMDP ? serializeAMDP(amdpCategories) : undefined,
      };

      await createSession(input);
      addToast(get(t)('sessions.saved'));
      goto(`/patients/${patientId}/sessions`);
    } catch (err) {
      error =
        get(t)('common.saveFailed') + ': ' + (err instanceof Error ? err.message : String(err));
      console.error('Failed to create session:', err);
    } finally {
      saving = false;
    }
  }

  function handleCancel() {
    goto(`/patients/${patientId}/sessions`);
  }
</script>

<div class="p-4 sm:p-6 lg:p-8 max-w-4xl mx-auto">
  <h1 class="text-display font-semibold text-fg mb-6">{$t('sessions.newSessionTitle')}</h1>

  {#if error}
    <div class="bg-danger-subtle border border-danger-line text-danger-fg p-4 rounded-card mb-6">
      {error}
    </div>
  {/if}

  <form onsubmit={handleSubmit} class="space-y-6">
    <div class="grid grid-cols-1 gap-4 sm:grid-cols-2 lg:grid-cols-3">
      <div class="col-span-2">
        <label for="session-type" class="block text-body font-medium text-fg-muted mb-1">
          {$t('sessions.sessionType')} *
        </label>
        <select
          id="session-type"
          bind:value={sessionType}
          required
          class="w-full px-3 py-2 bg-surface-raised border border-line rounded-control text-fg focus:outline-none focus:ring-2 focus:ring-accent/30"
        >
          {#each sessionTypes as type}
            <option value={type}>{$sessionTypeLabel(type)}</option>
          {/each}
        </select>
      </div>

      <div>
        <label for="duration" class="block text-body font-medium text-fg-muted mb-1">
          {$t('sessions.duration')}
        </label>
        <input
          id="duration"
          type="number"
          bind:value={durationMinutes}
          min="0"
          step="5"
          placeholder="50"
          class="w-full px-3 py-2 bg-surface-raised border border-line rounded-control text-fg focus:outline-none focus:ring-2 focus:ring-accent/30"
        />
      </div>
    </div>

    <div>
      <label for="session-date" class="block text-body font-medium text-fg-muted mb-1">
        {$t('sessions.date')} *
      </label>
      <input
        id="session-date"
        type="date"
        bind:value={sessionDate}
        required
        class="w-full px-3 py-2 bg-surface-raised border border-line rounded-control text-fg focus:outline-none focus:ring-2 focus:ring-accent/30"
      />
    </div>

    <div>
      <label for="session-time" class="block text-body font-medium text-fg-muted mb-1">
        {$t('sessions.timeOptional')}
      </label>
      <input
        id="session-time"
        type="time"
        bind:value={sessionTime}
        class="w-full px-3 py-2 bg-surface-raised border border-line rounded-control text-fg focus:outline-none focus:ring-2 focus:ring-accent/30"
      />
    </div>

    <div>
      <label for="notes" class="block text-body font-medium text-fg-muted mb-1">
        {$t('common.notes')} *
      </label>
      <textarea
        id="notes"
        bind:value={notes}
        required
        rows="8"
        placeholder={$t('sessions.notesPlaceholderDetailed')}
        class="w-full px-3 py-2 bg-surface-raised border border-line rounded-control text-fg focus:outline-none focus:ring-2 focus:ring-accent/30 resize-none"
      ></textarea>
    </div>

    <div>
      <label class="flex items-center gap-2 cursor-pointer">
        <input
          type="checkbox"
          bind:checked={showAMDP}
          class="w-4 h-4 bg-surface-selected border-line rounded-control text-accent-fg focus:ring-2 focus:ring-accent/30"
        />
        <span class="text-body font-medium text-fg-muted">{$t('sessions.amdpToggle')}</span>
      </label>
    </div>

    {#if showAMDP}
      <div class="border border-line rounded-card p-4">
        <h2 class="text-heading font-semibold text-fg mb-4">{$t('sessions.amdpTitle')}</h2>
        <AMDPForm categories={amdpCategories} onScoreChange={handleAMDPScoreChange} />
      </div>
    {/if}

    <div class="flex justify-end gap-3 pt-4 border-t border-line">
      <button
        type="button"
        onclick={handleCancel}
        class="h-8 px-3 bg-surface-hover text-fg-muted rounded-control hover:bg-surface-selected transition-colors"
        disabled={saving}
      >
        {$t('common.cancel')}
      </button>
      <button
        type="submit"
        class="h-8 px-3 bg-accent text-on-accent rounded-control hover:bg-accent-hover transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
        disabled={saving}
      >
        {saving ? $t('common.saving') : $t('sessions.saveSession')}
      </button>
    </div>
  </form>
</div>
