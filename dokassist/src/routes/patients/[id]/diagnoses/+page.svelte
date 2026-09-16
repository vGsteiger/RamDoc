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
  import { ListSkeleton } from '$lib/components/ui';
  import { get } from 'svelte/store';
  import { t } from '$lib/translations';

  const patientId = $derived($page.params.id!);

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
    { value: 'active', label: get(t)('diagnoses.active') },
    { value: 'remission', label: get(t)('diagnoses.remission') },
    { value: 'resolved', label: get(t)('diagnoses.resolved') },
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
        get(t)('common.loadFailed') + ': ' + (err instanceof Error ? err.message : String(err));
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
      error =
        get(t)('common.deleteFailed') + ': ' + (err instanceof Error ? err.message : String(err));
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
      error =
        get(t)('common.saveFailed') + ': ' + (err instanceof Error ? err.message : String(err));
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
    <h1 class="text-display font-semibold text-fg">{$t('diagnoses.title')}</h1>
    <button
      class="h-8 px-3 bg-accent text-on-accent rounded-control hover:bg-accent-hover transition-colors"
      onclick={() => {
        if (showAddForm) {
          resetForm();
        } else {
          showAddForm = true;
        }
      }}
    >
      {showAddForm ? $t('common.cancel') : `+ ${$t('diagnoses.newDiagnosis')}`}
    </button>
  </div>

  {#if error}
    <div class="bg-danger-subtle border border-danger-line text-danger-fg p-4 rounded-card mb-6">
      {error}
    </div>
  {/if}

  {#if showAddForm}
    <div class="bg-surface-raised border border-line rounded-card p-6 mb-6">
      <h2 class="text-heading font-semibold text-fg mb-4">
        {editingId ? $t('diagnoses.editDiagnosis') : $t('diagnoses.addDiagnosis')}
      </h2>
      <form onsubmit={handleSubmit} class="space-y-4">
        <div>
          <p class="block text-body font-medium text-fg-muted mb-1">{$t('diagnoses.icdCode')} *</p>
          <IcdSearch onSelect={handleIcdSelect} />
          {#if selectedCode}
            <div class="mt-2 p-3 bg-surface-selected rounded-card border border-line">
              <span class="font-mono text-body text-accent-fg">{selectedCode}</span>
              <span class="text-body text-fg-muted ml-2">— {selectedDescription}</span>
            </div>
          {/if}
        </div>

        <div class="grid grid-cols-2 gap-4">
          <div>
            <label for="diagnosed-date" class="block text-body font-medium text-fg-muted mb-1">
              {$t('diagnoses.diagnosedOn')} *
            </label>
            <input
              id="diagnosed-date"
              type="date"
              bind:value={diagnosedDate}
              required
              class="w-full px-3 py-2 bg-surface-raised border border-line rounded-control text-fg focus:outline-none focus:ring-2 focus:ring-accent/30"
            />
          </div>

          <div>
            <label for="status" class="block text-body font-medium text-fg-muted mb-1">
              {$t('diagnoses.status')} *
            </label>
            <select
              id="status"
              bind:value={status}
              required
              class="w-full px-3 py-2 bg-surface-raised border border-line rounded-control text-fg focus:outline-none focus:ring-2 focus:ring-accent/30"
            >
              {#each statusOptions as option}
                <option value={option.value}>{option.label}</option>
              {/each}
            </select>
          </div>
        </div>

        {#if status === 'resolved'}
          <div>
            <label for="resolved-date" class="block text-body font-medium text-fg-muted mb-1">
              {$t('diagnoses.resolvedOn')}
            </label>
            <input
              id="resolved-date"
              type="date"
              bind:value={resolvedDate}
              class="w-full px-3 py-2 bg-surface-raised border border-line rounded-control text-fg focus:outline-none focus:ring-2 focus:ring-accent/30"
            />
          </div>
        {/if}

        <div>
          <label for="notes" class="block text-body font-medium text-fg-muted mb-1">
            {$t('common.notes')}
          </label>
          <textarea
            id="notes"
            bind:value={notes}
            rows="3"
            placeholder={$t('diagnoses.notesPlaceholder')}
            class="w-full px-3 py-2 bg-surface-raised border border-line rounded-control text-fg focus:outline-none focus:ring-2 focus:ring-accent/30 resize-none"
          ></textarea>
        </div>

        <div class="flex justify-end gap-3 pt-4">
          <button
            type="button"
            onclick={resetForm}
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
            {saving ? $t('common.saving') : editingId ? $t('common.update') : $t('common.add')}
          </button>
        </div>
      </form>
    </div>
  {/if}

  {#if loading}
    <ListSkeleton variant="file" count={4} />
  {:else if diagnoses.length === 0}
    <div class="text-center py-12">
      <p class="text-fg-muted mb-4">{$t('diagnoses.noDiagnoses')}</p>
      {#if !showAddForm}
        <button
          class="h-8 px-3 bg-accent text-on-accent rounded-control hover:bg-accent-hover transition-colors"
          onclick={() => (showAddForm = true)}
        >
          {$t('diagnoses.addFirst')}
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
