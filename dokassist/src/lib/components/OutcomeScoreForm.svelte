<script lang="ts">
  import type { CreateOutcomeScore, UpdateOutcomeScore, OutcomeScore } from '$lib/api';

  interface Props {
    outcomeScore?: OutcomeScore;
    sessionId?: string;
    onSave: (input: CreateOutcomeScore | { id: string; update: UpdateOutcomeScore }) => void;
    onCancel: () => void;
  }

  let { outcomeScore, sessionId, onSave, onCancel }: Props = $props();

  let scaleType = $state(outcomeScore?.scale_type || 'PHQ-9');
  let score = $state(outcomeScore?.score?.toString() || '');
  let administeredAt = $state(outcomeScore?.administered_at || new Date().toISOString().split('T')[0]);
  let notes = $state(outcomeScore?.notes || '');

  const scaleOptions = [
    { value: 'PHQ-9', label: 'PHQ-9 (Depression)', max: 27 },
    { value: 'GAD-7', label: 'GAD-7 (Anxiety)', max: 21 },
    { value: 'BDI-II', label: 'BDI-II (Depression)', max: 63 }
  ];

  let maxScore = $derived(scaleOptions.find(s => s.value === scaleType)?.max || 27);

  function handleSubmit(event: Event) {
    event.preventDefault();

    const scoreValue = parseInt(score, 10);
    if (isNaN(scoreValue) || scoreValue < 0 || scoreValue > maxScore) {
      return;
    }

    if (outcomeScore) {
      // Update existing score
      const update: UpdateOutcomeScore = {
        scale_type: scaleType !== outcomeScore.scale_type ? scaleType : undefined,
        score: scoreValue !== outcomeScore.score ? scoreValue : undefined,
        administered_at: administeredAt !== outcomeScore.administered_at ? administeredAt : undefined,
        notes: notes !== (outcomeScore.notes || '') ? (notes || undefined) : undefined
      };
      onSave({ id: outcomeScore.id, update });
    } else if (sessionId) {
      // Create new score
      const input: CreateOutcomeScore = {
        session_id: sessionId,
        scale_type: scaleType,
        score: scoreValue,
        administered_at: administeredAt,
        notes: notes || undefined
      };
      onSave(input);
    }
  }
</script>

<form onsubmit={handleSubmit} class="space-y-4">
  <div>
    <label for="scale-type" class="block text-sm font-medium text-gray-900 dark:text-gray-100 mb-1">
      Fragebogen *
    </label>
    <select
      id="scale-type"
      bind:value={scaleType}
      required
      class="w-full px-3 py-2 bg-white dark:bg-gray-700 border border-gray-300 dark:border-gray-600 rounded-lg text-gray-900 dark:text-gray-100 focus:outline-none focus:ring-2 focus:ring-blue-500"
    >
      {#each scaleOptions as option}
        <option value={option.value}>{option.label}</option>
      {/each}
    </select>
  </div>

  <div>
    <label for="score" class="block text-sm font-medium text-gray-900 dark:text-gray-100 mb-1">
      Gesamtpunktzahl * (0-{maxScore})
    </label>
    <input
      id="score"
      type="number"
      bind:value={score}
      required
      min="0"
      max={maxScore}
      placeholder="z.B. 12"
      class="w-full px-3 py-2 bg-white dark:bg-gray-700 border border-gray-300 dark:border-gray-600 rounded-lg text-gray-900 dark:text-gray-100 placeholder-gray-400 focus:outline-none focus:ring-2 focus:ring-blue-500"
    />
  </div>

  <div>
    <label for="administered-at" class="block text-sm font-medium text-gray-900 dark:text-gray-100 mb-1">
      Durchführungsdatum *
    </label>
    <input
      id="administered-at"
      type="date"
      bind:value={administeredAt}
      required
      class="w-full px-3 py-2 bg-white dark:bg-gray-700 border border-gray-300 dark:border-gray-600 rounded-lg text-gray-900 dark:text-gray-100 focus:outline-none focus:ring-2 focus:ring-blue-500"
    />
  </div>

  <div>
    <label for="notes" class="block text-sm font-medium text-gray-900 dark:text-gray-100 mb-1">
      Notizen
    </label>
    <textarea
      id="notes"
      bind:value={notes}
      rows="3"
      placeholder="Zusätzliche Beobachtungen..."
      class="w-full px-3 py-2 bg-white dark:bg-gray-700 border border-gray-300 dark:border-gray-600 rounded-lg text-gray-900 dark:text-gray-100 placeholder-gray-400 focus:outline-none focus:ring-2 focus:ring-blue-500 resize-none"
    />
  </div>

  <div class="flex justify-end gap-3 pt-4">
    <button
      type="button"
      onclick={onCancel}
      class="px-4 py-2 bg-gray-200 dark:bg-gray-700 text-gray-900 dark:text-gray-300 rounded-lg hover:bg-gray-300 dark:hover:bg-gray-600 transition-colors"
    >
      Abbrechen
    </button>
    <button
      type="submit"
      class="px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 transition-colors"
    >
      {outcomeScore ? 'Aktualisieren' : 'Hinzufügen'}
    </button>
  </div>
</form>
