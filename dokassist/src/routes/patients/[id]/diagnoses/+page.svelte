<script lang="ts">
  import { onMount } from 'svelte';
  import { page } from '$app/stores';
  import {
    listDiagnosesForPatient,
    createDiagnosis,
    updateDiagnosis,
    deleteDiagnosis,
    type Diagnosis,
    type CreateDiagnosis,
    type UpdateDiagnosis,
  } from '$lib/api';
  import DiagnosisCard from '$lib/components/DiagnosisCard.svelte';
  import IcdSearch from '$lib/components/IcdSearch.svelte';
  import { get } from 'svelte/store';
  import { t } from '$lib/translations';

  const patientId = $derived($page.params.id);

  let diagnoses = $state<Diagnosis[]>([]);
  let loading = $state(true);
  let error = $state<string | null>(null);
  let showAddForm = $state(false);

  // Form state
  let selectedCode = $state('');
  let selectedDescription = $state('');
  let diagnosedDate = $state(new Date().toISOString().split('T')[0]);
  let status = $state('active');
  let resolvedDate = $state('');
  let notes = $state('');
  let saving = $state(false);
  let editingId = $state<string | null>(null);

  const statusOptions = [
    { value: 'active', label: 'Aktiv' },
    { value: 'remission', label: 'Remission' },
    { value: 'resolved', label: 'Aufgelöst' },
  ];

  onMount(async () => {
    await loadDiagnoses();
  });

  async function loadDiagnoses() {
    try {
      loading = true;
      error = null;
      diagnoses = await listDiagnosesForPatient(patientId);
    } catch (err) {
      error =
        'Fehler beim Laden der Diagnosen: ' + (err instanceof Error ? err.message : String(err));
      console.error('Failed to load diagnoses:', err);
    } finally {
      loading = false;
    }
  }

  function handleIcdSelect(code: string, description: string) {
    selectedCode = code;
    selectedDescription = description;
  }

  function handleEdit(diagnosis: Diagnosis) {
    editingId = diagnosis.id;
    selectedCode = diagnosis.icd10_code;
    selectedDescription = diagnosis.description;
    diagnosedDate = diagnosis.diagnosed_date;
    status = diagnosis.status;
    resolvedDate = diagnosis.resolved_date || '';
    notes = diagnosis.notes || '';
    showAddForm = true;
  }

  async function handleDelete(diagnosisId: string) {
    if (!confirm(get(t)('diagnoses.confirmDelete'))) {
      return;
    }

    try {
      await deleteDiagnosis(diagnosisId);
      await loadDiagnoses();
    } catch (err) {
      error = 'Fehler beim Löschen: ' + (err instanceof Error ? err.message : String(err));
      console.error('Failed to delete diagnosis:', err);
    }
  }

  async function handleSubmit(event: Event) {
    event.preventDefault();

    if (!selectedCode || !selectedDescription) {
      error = get(t)('diagnoses.selectRequired');
      return;
    }

    try {
      saving = true;
      error = null;

      if (editingId) {
        // Update existing diagnosis
        const update: UpdateDiagnosis = {
          icd10_code: selectedCode,
          description: selectedDescription,
          status,
          diagnosed_date: diagnosedDate,
          resolved_date: resolvedDate || undefined,
          notes: notes || undefined,
        };
        await updateDiagnosis(editingId, update);
      } else {
        // Create new diagnosis
        const input: CreateDiagnosis = {
          patient_id: patientId,
          icd10_code: selectedCode,
          description: selectedDescription,
          status,
          diagnosed_date: diagnosedDate,
          resolved_date: resolvedDate || undefined,
          notes: notes || undefined,
        };
        await createDiagnosis(input);
      }

      // Reset form
      resetForm();
      await loadDiagnoses();
    } catch (err) {
      error = 'Fehler beim Speichern: ' + (err instanceof Error ? err.message : String(err));
      console.error('Failed to save diagnosis:', err);
    } finally {
      saving = false;
    }
  }

  function resetForm() {
    showAddForm = false;
    editingId = null;
    selectedCode = '';
    selectedDescription = '';
    diagnosedDate = new Date().toISOString().split('T')[0];
    status = 'active';
    resolvedDate = '';
    notes = '';
  }
</script>

<div class="p-8 max-w-4xl mx-auto">
  <div class="flex justify-between items-center mb-6">
    <h1 class="text-2xl font-bold text-gray-900 dark:text-gray-100">Diagnosen</h1>
    <button
      class="px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 transition-colors"
      onclick={() => {
        if (showAddForm) {
          resetForm();
        } else {
          showAddForm = true;
        }
      }}
    >
      {showAddForm ? 'Abbrechen' : '+ Neue Diagnose'}
    </button>
  </div>

  {#if error}
    <div class="bg-red-500/10 border border-red-500/30 text-red-400 p-4 rounded-lg mb-6">
      {error}
    </div>
  {/if}

  {#if showAddForm}
    <div
      class="bg-white dark:bg-gray-800 border border-gray-200 dark:border-gray-700 rounded-lg p-6 mb-6"
    >
      <h2 class="text-lg font-semibold text-gray-900 dark:text-gray-100 mb-4">
        {editingId ? 'Diagnose bearbeiten' : 'Neue Diagnose hinzufügen'}
      </h2>
      <form onsubmit={handleSubmit} class="space-y-4">
        <div>
          <p class="block text-sm font-medium text-gray-600 dark:text-gray-300 mb-1">
            ICD-10 Code *
          </p>
          <IcdSearch onSelect={handleIcdSelect} />
          {#if selectedCode}
            <div class="mt-2 p-3 bg-gray-700 rounded border border-gray-600">
              <span class="font-mono text-sm text-blue-500 dark:text-blue-400">{selectedCode}</span>
              <span class="text-sm text-gray-600 dark:text-gray-300 ml-2"
                >— {selectedDescription}</span
              >
            </div>
          {/if}
        </div>

        <div class="grid grid-cols-2 gap-4">
          <div>
            <label
              for="diagnosed-date"
              class="block text-sm font-medium text-gray-600 dark:text-gray-300 mb-1"
            >
              Diagnosedatum *
            </label>
            <input
              id="diagnosed-date"
              type="date"
              bind:value={diagnosedDate}
              required
              class="w-full px-3 py-2 bg-white dark:bg-gray-700 border border-gray-300 dark:border-gray-600 rounded-lg text-gray-900 dark:text-gray-100 focus:outline-none focus:ring-2 focus:ring-blue-500"
            />
          </div>

          <div>
            <label
              for="status"
              class="block text-sm font-medium text-gray-600 dark:text-gray-300 mb-1"
            >
              Status *
            </label>
            <select
              id="status"
              bind:value={status}
              required
              class="w-full px-3 py-2 bg-white dark:bg-gray-700 border border-gray-300 dark:border-gray-600 rounded-lg text-gray-900 dark:text-gray-100 focus:outline-none focus:ring-2 focus:ring-blue-500"
            >
              {#each statusOptions as option}
                <option value={option.value}>{option.label}</option>
              {/each}
            </select>
          </div>
        </div>

        {#if status === 'resolved'}
          <div>
            <label
              for="resolved-date"
              class="block text-sm font-medium text-gray-600 dark:text-gray-300 mb-1"
            >
              Auflösungsdatum
            </label>
            <input
              id="resolved-date"
              type="date"
              bind:value={resolvedDate}
              class="w-full px-3 py-2 bg-white dark:bg-gray-700 border border-gray-300 dark:border-gray-600 rounded-lg text-gray-900 dark:text-gray-100 focus:outline-none focus:ring-2 focus:ring-blue-500"
            />
          </div>
        {/if}

        <div>
          <label
            for="notes"
            class="block text-sm font-medium text-gray-600 dark:text-gray-300 mb-1"
          >
            Notizen
          </label>
          <textarea
            id="notes"
            bind:value={notes}
            rows="3"
            placeholder="Zusätzliche Informationen..."
            class="w-full px-3 py-2 bg-gray-700 border border-gray-600 rounded-lg text-gray-100 placeholder-gray-400 focus:outline-none focus:ring-2 focus:ring-blue-500 resize-none"
          ></textarea>
        </div>

        <div class="flex justify-end gap-3 pt-4">
          <button
            type="button"
            onclick={resetForm}
            class="px-4 py-2 bg-gray-100 dark:bg-gray-700 text-gray-600 dark:text-gray-300 rounded-lg hover:bg-gray-200 dark:hover:bg-gray-600 transition-colors"
            disabled={saving}
          >
            Abbrechen
          </button>
          <button
            type="submit"
            class="px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
            disabled={saving}
          >
            {saving ? 'Speichert...' : editingId ? 'Aktualisieren' : 'Hinzufügen'}
          </button>
        </div>
      </form>
    </div>
  {/if}

  {#if loading}
    <div class="flex justify-center items-center py-12">
      <div class="text-gray-500 dark:text-gray-400">Lädt...</div>
    </div>
  {:else if diagnoses.length === 0}
    <div class="text-center py-12">
      <p class="text-gray-500 dark:text-gray-400 mb-4">Noch keine Diagnosen erfasst</p>
      {#if !showAddForm}
        <button
          class="px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 transition-colors"
          onclick={() => (showAddForm = true)}
        >
          Erste Diagnose erfassen
        </button>
      {/if}
    </div>
  {:else}
    <div class="grid gap-4">
      {#each diagnoses as diagnosis (diagnosis.id)}
        <DiagnosisCard
          {diagnosis}
          onEdit={() => handleEdit(diagnosis)}
          onDelete={() => handleDelete(diagnosis.id)}
        />
      {/each}
    </div>
  {/if}
</div>
