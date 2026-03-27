<script lang="ts">
  import { untrack } from 'svelte';
  import type { CreateMedication, UpdateMedication, Medication, SubstanceSummary } from '$lib/api';
  import MedicationAutocomplete from './MedicationAutocomplete.svelte';
  import MedicationInfoPanel from './MedicationInfoPanel.svelte';

  interface Props {
    medication?: Medication;
    patientId?: string;
    onSave: (input: CreateMedication | { id: string; update: UpdateMedication }) => void;
    onCancel: () => void;
  }

  let { medication, patientId, onSave, onCancel }: Props = $props();

  let substance = $state(untrack(() => medication?.substance || ''));
  let dosage = $state(untrack(() => medication?.dosage || ''));
  let frequency = $state(untrack(() => medication?.frequency || ''));
  let startDate = $state(
    untrack(() => medication?.start_date || new Date().toISOString().split('T')[0])
  );
  let endDate = $state(untrack(() => medication?.end_date || ''));
  let notes = $state(untrack(() => medication?.notes || ''));
  let selectedSubstanceId = $state<string | null>(null);

  $effect(() => {
    if (medication) {
      substance = medication.substance || '';
      dosage = medication.dosage || '';
      frequency = medication.frequency || '';
      startDate = medication.start_date || new Date().toISOString().split('T')[0];
      endDate = medication.end_date || '';
      notes = medication.notes || '';
      // Clear any reference panel when editing an existing record
      selectedSubstanceId = null;
    }
  });

  function handleSubstanceSelect(summary: SubstanceSummary) {
    substance = summary.name_de;
    selectedSubstanceId = summary.id;
  }

  function handleSubmit(event: Event) {
    event.preventDefault();

    if (!substance.trim() || !dosage.trim() || !frequency.trim()) {
      return;
    }

    if (medication) {
      const update: UpdateMedication = {
        substance: substance !== medication.substance ? substance : undefined,
        dosage: dosage !== medication.dosage ? dosage : undefined,
        frequency: frequency !== medication.frequency ? frequency : undefined,
        start_date: startDate !== medication.start_date ? startDate : undefined,
        end_date: endDate !== (medication.end_date || '') ? endDate || undefined : undefined,
        notes: notes !== (medication.notes || '') ? notes || undefined : undefined,
      };
      onSave({ id: medication.id, update });
    } else if (patientId) {
      const input: CreateMedication = {
        patient_id: patientId,
        substance,
        dosage,
        frequency,
        start_date: startDate,
        end_date: endDate || undefined,
        notes: notes || undefined,
      };
      onSave(input);
    }
  }
</script>

<form onsubmit={handleSubmit} class="space-y-4">
  <div>
    <label for="substance" class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
      Wirkstoff *
    </label>
    <MedicationAutocomplete
      id="substance"
      value={substance}
      onInput={(v) => {
        substance = v;
        selectedSubstanceId = null;
      }}
      onSelect={handleSubstanceSelect}
      required
      placeholder="z.B. Sertralin"
      class="w-full px-3 py-2 bg-white dark:bg-gray-700 border border-gray-300 dark:border-gray-600 rounded-lg text-gray-900 dark:text-gray-100 placeholder-gray-400 focus:outline-none focus:ring-2 focus:ring-blue-500"
    />
    <MedicationInfoPanel substanceId={selectedSubstanceId} />
  </div>

  <div>
    <label for="dosage" class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1"> Dosierung * </label>
    <input
      id="dosage"
      type="text"
      bind:value={dosage}
      required
      placeholder="z.B. 50 mg"
      class="w-full px-3 py-2 bg-white dark:bg-gray-700 border border-gray-300 dark:border-gray-600 rounded-lg text-gray-900 dark:text-gray-100 placeholder-gray-400 focus:outline-none focus:ring-2 focus:ring-blue-500"
    />
  </div>

  <div>
    <label for="frequency" class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
      Häufigkeit *
    </label>
    <input
      id="frequency"
      type="text"
      bind:value={frequency}
      required
      placeholder="z.B. 1x täglich"
      class="w-full px-3 py-2 bg-white dark:bg-gray-700 border border-gray-300 dark:border-gray-600 rounded-lg text-gray-900 dark:text-gray-100 placeholder-gray-400 focus:outline-none focus:ring-2 focus:ring-blue-500"
    />
  </div>

  <div class="grid grid-cols-2 gap-4">
    <div>
      <label for="start-date" class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
        Startdatum *
      </label>
      <input
        id="start-date"
        type="date"
        bind:value={startDate}
        required
        class="w-full px-3 py-2 bg-white dark:bg-gray-700 border border-gray-300 dark:border-gray-600 rounded-lg text-gray-900 dark:text-gray-100 focus:outline-none focus:ring-2 focus:ring-blue-500"
      />
    </div>

    <div>
      <label for="end-date" class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1"> Enddatum </label>
      <input
        id="end-date"
        type="date"
        bind:value={endDate}
        class="w-full px-3 py-2 bg-white dark:bg-gray-700 border border-gray-300 dark:border-gray-600 rounded-lg text-gray-900 dark:text-gray-100 focus:outline-none focus:ring-2 focus:ring-blue-500"
      />
    </div>
  </div>

  <div>
    <label for="notes" class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1"> Notizen </label>
    <textarea
      id="notes"
      bind:value={notes}
      rows="3"
      placeholder="Zusätzliche Informationen..."
      class="w-full px-3 py-2 bg-white dark:bg-gray-700 border border-gray-300 dark:border-gray-600 rounded-lg text-gray-900 dark:text-gray-100 placeholder-gray-400 focus:outline-none focus:ring-2 focus:ring-blue-500 resize-none"
    ></textarea>
  </div>

  <div class="flex justify-end gap-3 pt-4">
    <button
      type="button"
      onclick={onCancel}
      class="px-4 py-2 bg-gray-100 dark:bg-gray-700 text-gray-700 dark:text-gray-300 rounded-lg hover:bg-gray-200 dark:hover:bg-gray-600 transition-colors"
    >
      Abbrechen
    </button>
    <button
      type="submit"
      class="px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 transition-colors"
    >
      {medication ? 'Aktualisieren' : 'Hinzufügen'}
    </button>
  </div>
</form>
