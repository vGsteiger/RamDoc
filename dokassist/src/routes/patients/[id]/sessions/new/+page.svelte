<script lang="ts">
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
        patient_id: patientId,
        session_date: sessionDate,
        session_type: sessionType,
        duration_minutes: durationMinutes || undefined,
        scheduled_time: sessionTime ? `${sessionDate}T${sessionTime}:00` : undefined,
        notes,
        amdp_data: showAMDP ? serializeAMDP(amdpCategories) : undefined,
      };

      await createSession(input);
      addToast('Session saved');
      goto(`/patients/${patientId}/sessions`);
    } catch (err) {
      error = 'Fehler beim Speichern: ' + (err instanceof Error ? err.message : String(err));
      console.error('Failed to create session:', err);
    } finally {
      saving = false;
    }
  }

  function handleCancel() {
    goto(`/patients/${patientId}/sessions`);
  }
</script>

<div class="p-8 max-w-4xl mx-auto">
  <h1 class="text-2xl font-bold text-gray-900 dark:text-gray-100 mb-6">Neue Sitzung erfassen</h1>

  {#if error}
    <div class="bg-red-500/10 border border-red-500/30 text-red-400 p-4 rounded-lg mb-6">
      {error}
    </div>
  {/if}

  <form onsubmit={handleSubmit} class="space-y-6">
    <div class="grid grid-cols-3 gap-4">
      <div class="col-span-2">
        <label for="session-type" class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
          Sitzungstyp *
        </label>
        <select
          id="session-type"
          bind:value={sessionType}
          required
          class="w-full px-3 py-2 bg-white dark:bg-gray-700 border border-gray-300 dark:border-gray-600 rounded-lg text-gray-900 dark:text-gray-100 focus:outline-none focus:ring-2 focus:ring-blue-500"
        >
          {#each sessionTypes as type}
            <option value={type}>{type}</option>
          {/each}
        </select>
      </div>

      <div>
        <label for="duration" class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
          Dauer (Min.)
        </label>
        <input
          id="duration"
          type="number"
          bind:value={durationMinutes}
          min="0"
          step="5"
          placeholder="50"
          class="w-full px-3 py-2 bg-white dark:bg-gray-700 border border-gray-300 dark:border-gray-600 rounded-lg text-gray-900 dark:text-gray-100 focus:outline-none focus:ring-2 focus:ring-blue-500"
        />
      </div>
    </div>

    <div>
      <label for="session-date" class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
        Datum *
      </label>
      <input
        id="session-date"
        type="date"
        bind:value={sessionDate}
        required
        class="w-full px-3 py-2 bg-white dark:bg-gray-700 border border-gray-300 dark:border-gray-600 rounded-lg text-gray-900 dark:text-gray-100 focus:outline-none focus:ring-2 focus:ring-blue-500"
      />
    </div>

    <div>
      <label for="session-time" class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
        Uhrzeit (optional)
      </label>
      <input
        id="session-time"
        type="time"
        bind:value={sessionTime}
        class="w-full px-3 py-2 bg-white dark:bg-gray-700 border border-gray-300 dark:border-gray-600 rounded-lg text-gray-900 dark:text-gray-100 focus:outline-none focus:ring-2 focus:ring-blue-500"
      />
    </div>

    <div>
      <label for="notes" class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1"> Notizen * </label>
      <textarea
        id="notes"
        bind:value={notes}
        required
        rows="8"
        placeholder="Gesprächsnotizen, Beobachtungen, Interventionen..."
        class="w-full px-3 py-2 bg-white dark:bg-gray-700 border border-gray-300 dark:border-gray-600 rounded-lg text-gray-900 dark:text-gray-100 placeholder-gray-400 focus:outline-none focus:ring-2 focus:ring-blue-500 resize-none"
      ></textarea>
    </div>

    <div>
      <label class="flex items-center gap-2 cursor-pointer">
        <input
          type="checkbox"
          bind:checked={showAMDP}
          class="w-4 h-4 bg-gray-700 border-gray-600 rounded text-blue-600 focus:ring-2 focus:ring-blue-500"
        />
        <span class="text-sm font-medium text-gray-300"
          >AMDP psychopathologische Befunde erfassen</span
        >
      </label>
    </div>

    {#if showAMDP}
      <div class="border border-gray-700 rounded-lg p-4">
        <h2 class="text-lg font-semibold text-gray-100 mb-4">AMDP Befunderhebung</h2>
        <AMDPForm categories={amdpCategories} onScoreChange={handleAMDPScoreChange} />
      </div>
    {/if}

    <div class="flex justify-end gap-3 pt-4 border-t border-gray-700">
      <button
        type="button"
        onclick={handleCancel}
        class="px-6 py-2 bg-gray-100 dark:bg-gray-700 text-gray-700 dark:text-gray-300 rounded-lg hover:bg-gray-200 dark:hover:bg-gray-600 transition-colors"
        disabled={saving}
      >
        Abbrechen
      </button>
      <button
        type="submit"
        class="px-6 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
        disabled={saving}
      >
        {saving ? 'Speichert...' : 'Sitzung speichern'}
      </button>
    </div>
  </form>
</div>
