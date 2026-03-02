<script lang="ts">
  import { onMount } from 'svelte';
  import { page } from '$app/stores';
  import {
    listMedicationsForPatient,
    createMedication,
    updateMedication,
    deleteMedication,
    type Medication,
    type CreateMedication,
    type UpdateMedication
  } from '$lib/api';
  import MedicationForm from '$lib/components/MedicationForm.svelte';

  const patientId = $derived($page.params.id);

  let medications = $state<Medication[]>([]);
  let loading = $state(true);
  let error = $state<string | null>(null);
  let showAddForm = $state(false);
  let editingMedication = $state<Medication | null>(null);

  onMount(async () => {
    await loadMedications();
  });

  async function loadMedications() {
    try {
      loading = true;
      error = null;
      medications = await listMedicationsForPatient(patientId);
    } catch (err) {
      error = 'Fehler beim Laden der Medikamente: ' + (err instanceof Error ? err.message : String(err));
      console.error('Failed to load medications:', err);
    } finally {
      loading = false;
    }
  }

  function handleEdit(medication: Medication) {
    editingMedication = medication;
    showAddForm = true;
  }

  async function handleDelete(medicationId: string) {
    if (!confirm('Möchten Sie dieses Medikament wirklich löschen?')) {
      return;
    }

    try {
      await deleteMedication(medicationId);
      await loadMedications();
    } catch (err) {
      error = 'Fehler beim Löschen: ' + (err instanceof Error ? err.message : String(err));
      console.error('Failed to delete medication:', err);
    }
  }

  async function handleSave(
    input: CreateMedication | { id: string; update: UpdateMedication }
  ) {
    try {
      error = null;

      if ('id' in input) {
        // Update existing medication
        await updateMedication(input.id, input.update);
      } else {
        // Create new medication
        await createMedication(input);
      }

      // Reset form
      showAddForm = false;
      editingMedication = null;
      await loadMedications();
    } catch (err) {
      error = 'Fehler beim Speichern: ' + (err instanceof Error ? err.message : String(err));
      console.error('Failed to save medication:', err);
    }
  }

  function handleCancel() {
    showAddForm = false;
    editingMedication = null;
  }

  function formatDate(dateStr: string | null): string {
    if (!dateStr) return '—';
    try {
      const date = new Date(dateStr);
      return date.toLocaleDateString('de-CH', {
        year: 'numeric',
        month: '2-digit',
        day: '2-digit'
      });
    } catch {
      return dateStr;
    }
  }

  function isActive(medication: Medication): boolean {
    if (!medication.end_date) return true;
    const endDate = new Date(medication.end_date);
    return endDate >= new Date();
  }
</script>

<div class="p-8 max-w-4xl mx-auto">
  <div class="flex justify-between items-center mb-6">
    <h1 class="text-2xl font-bold text-gray-100">Medikation</h1>
    <button
      class="px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 transition-colors"
      onclick={() => {
        showAddForm = !showAddForm;
        editingMedication = null;
      }}
    >
      {showAddForm ? 'Abbrechen' : '+ Neues Medikament'}
    </button>
  </div>

  {#if error}
    <div class="bg-red-500/10 border border-red-500/30 text-red-400 p-4 rounded-lg mb-6">
      {error}
    </div>
  {/if}

  {#if showAddForm}
    <div class="bg-gray-800 border border-gray-700 rounded-lg p-6 mb-6">
      <h2 class="text-lg font-semibold text-gray-100 mb-4">
        {editingMedication ? 'Medikament bearbeiten' : 'Neues Medikament hinzufügen'}
      </h2>
      <MedicationForm
        medication={editingMedication || undefined}
        {patientId}
        onSave={handleSave}
        onCancel={handleCancel}
      />
    </div>
  {/if}

  {#if loading}
    <div class="flex justify-center items-center py-12">
      <div class="text-gray-400">Lädt...</div>
    </div>
  {:else if medications.length === 0}
    <div class="text-center py-12">
      <p class="text-gray-400 mb-4">Noch keine Medikamente erfasst</p>
      {#if !showAddForm}
        <button
          class="px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 transition-colors"
          onclick={() => (showAddForm = true)}
        >
          Erstes Medikament erfassen
        </button>
      {/if}
    </div>
  {:else}
    <div class="grid gap-4">
      {#each medications as medication (medication.id)}
        <div class="p-4 bg-gray-800 rounded-lg border border-gray-700">
          <div class="flex justify-between items-start mb-2">
            <div class="flex-1">
              <div class="flex items-center gap-2 mb-1">
                <h3 class="text-lg font-semibold text-gray-100">{medication.substance}</h3>
                {#if isActive(medication)}
                  <span class="px-2 py-0.5 rounded-full text-xs bg-green-500/20 text-green-400 border border-green-500/30">
                    Aktiv
                  </span>
                {:else}
                  <span class="px-2 py-0.5 rounded-full text-xs bg-gray-500/20 text-gray-400 border border-gray-500/30">
                    Beendet
                  </span>
                {/if}
              </div>
              <p class="text-sm text-gray-300">
                {medication.dosage} • {medication.frequency}
              </p>
              <p class="text-sm text-gray-400 mt-1">
                Von {formatDate(medication.start_date)}
                {#if medication.end_date}
                  bis {formatDate(medication.end_date)}
                {/if}
              </p>
              {#if medication.notes}
                <p class="text-sm text-gray-300 mt-2">{medication.notes}</p>
              {/if}
            </div>
            <div class="flex gap-2 ml-2">
              <button
                type="button"
                class="p-2 text-gray-400 hover:text-blue-400 hover:bg-gray-700 rounded transition-colors"
                onclick={() => handleEdit(medication)}
                title="Bearbeiten"
              >
                <svg
                  class="w-4 h-4"
                  fill="none"
                  stroke="currentColor"
                  viewBox="0 0 24 24"
                >
                  <path
                    stroke-linecap="round"
                    stroke-linejoin="round"
                    stroke-width="2"
                    d="M11 5H6a2 2 0 00-2 2v11a2 2 0 002 2h11a2 2 0 002-2v-5m-1.414-9.414a2 2 0 112.828 2.828L11.828 15H9v-2.828l8.586-8.586z"
                  />
                </svg>
              </button>
              <button
                type="button"
                class="p-2 text-gray-400 hover:text-red-400 hover:bg-gray-700 rounded transition-colors"
                onclick={() => handleDelete(medication.id)}
                title="Löschen"
              >
                <svg
                  class="w-4 h-4"
                  fill="none"
                  stroke="currentColor"
                  viewBox="0 0 24 24"
                >
                  <path
                    stroke-linecap="round"
                    stroke-linejoin="round"
                    stroke-width="2"
                    d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16"
                  />
                </svg>
              </button>
            </div>
          </div>
        </div>
      {/each}
    </div>
  {/if}
</div>
