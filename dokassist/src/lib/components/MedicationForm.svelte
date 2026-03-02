<script lang="ts">
  import type { CreateMedication, UpdateMedication, Medication } from '$lib/api';

  interface Props {
    medication?: Medication;
    patientId?: string;
    onSave: (input: CreateMedication | { id: string; update: UpdateMedication }) => void;
    onCancel: () => void;
  }

  let { medication, patientId, onSave, onCancel }: Props = $props();

  let substance = $state(medication?.substance || '');
  let dosage = $state(medication?.dosage || '');
  let frequency = $state(medication?.frequency || '');
  let startDate = $state(medication?.start_date || new Date().toISOString().split('T')[0]);
  let endDate = $state(medication?.end_date || '');
  let notes = $state(medication?.notes || '');

  function handleSubmit(event: Event) {
    event.preventDefault();

    if (!substance.trim() || !dosage.trim() || !frequency.trim()) {
      return;
    }

    if (medication) {
      // Update existing medication
      const update: UpdateMedication = {
        substance: substance !== medication.substance ? substance : undefined,
        dosage: dosage !== medication.dosage ? dosage : undefined,
        frequency: frequency !== medication.frequency ? frequency : undefined,
        start_date: startDate !== medication.start_date ? startDate : undefined,
        end_date: endDate !== (medication.end_date || '') ? (endDate || undefined) : undefined,
        notes: notes !== (medication.notes || '') ? (notes || undefined) : undefined
      };
      onSave({ id: medication.id, update });
    } else if (patientId) {
      // Create new medication
      const input: CreateMedication = {
        patient_id: patientId,
        substance,
        dosage,
        frequency,
        start_date: startDate,
        end_date: endDate || undefined,
        notes: notes || undefined
      };
      onSave(input);
    }
  }
</script>

<form onsubmit={handleSubmit} class="space-y-4">
  <div>
    <label for="substance" class="block text-sm font-medium text-gray-300 mb-1">
      Wirkstoff *
    </label>
    <input
      id="substance"
      type="text"
      bind:value={substance}
      required
      placeholder="z.B. Sertralin"
      class="w-full px-3 py-2 bg-gray-700 border border-gray-600 rounded-lg text-gray-100 placeholder-gray-400 focus:outline-none focus:ring-2 focus:ring-blue-500"
    />
  </div>

  <div>
    <label for="dosage" class="block text-sm font-medium text-gray-300 mb-1">
      Dosierung *
    </label>
    <input
      id="dosage"
      type="text"
      bind:value={dosage}
      required
      placeholder="z.B. 50 mg"
      class="w-full px-3 py-2 bg-gray-700 border border-gray-600 rounded-lg text-gray-100 placeholder-gray-400 focus:outline-none focus:ring-2 focus:ring-blue-500"
    />
  </div>

  <div>
    <label for="frequency" class="block text-sm font-medium text-gray-300 mb-1">
      Häufigkeit *
    </label>
    <input
      id="frequency"
      type="text"
      bind:value={frequency}
      required
      placeholder="z.B. 1x täglich"
      class="w-full px-3 py-2 bg-gray-700 border border-gray-600 rounded-lg text-gray-100 placeholder-gray-400 focus:outline-none focus:ring-2 focus:ring-blue-500"
    />
  </div>

  <div class="grid grid-cols-2 gap-4">
    <div>
      <label for="start-date" class="block text-sm font-medium text-gray-300 mb-1">
        Startdatum *
      </label>
      <input
        id="start-date"
        type="date"
        bind:value={startDate}
        required
        class="w-full px-3 py-2 bg-gray-700 border border-gray-600 rounded-lg text-gray-100 focus:outline-none focus:ring-2 focus:ring-blue-500"
      />
    </div>

    <div>
      <label for="end-date" class="block text-sm font-medium text-gray-300 mb-1">
        Enddatum
      </label>
      <input
        id="end-date"
        type="date"
        bind:value={endDate}
        class="w-full px-3 py-2 bg-gray-700 border border-gray-600 rounded-lg text-gray-100 focus:outline-none focus:ring-2 focus:ring-blue-500"
      />
    </div>
  </div>

  <div>
    <label for="notes" class="block text-sm font-medium text-gray-300 mb-1">
      Notizen
    </label>
    <textarea
      id="notes"
      bind:value={notes}
      rows="3"
      placeholder="Zusätzliche Informationen..."
      class="w-full px-3 py-2 bg-gray-700 border border-gray-600 rounded-lg text-gray-100 placeholder-gray-400 focus:outline-none focus:ring-2 focus:ring-blue-500 resize-none"
    />
  </div>

  <div class="flex justify-end gap-3 pt-4">
    <button
      type="button"
      onclick={onCancel}
      class="px-4 py-2 bg-gray-700 text-gray-300 rounded-lg hover:bg-gray-600 transition-colors"
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
