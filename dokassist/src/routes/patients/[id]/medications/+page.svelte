<script lang="ts">
  import { onMount } from 'svelte';
  import { page } from '$app/state';
  import {
    listMedicationsForPatient,
    createMedication,
    updateMedication,
    deleteMedication,
    type Medication,
    type CreateMedication,
    type UpdateMedication,
  } from '$lib/api';
  import MedicationForm from '$lib/components/MedicationForm.svelte';
  import { get } from 'svelte/store';
  import { t } from '$lib/translations';

  const patientId = $derived(page.params.id!);

  let medications = $state<Medication[]>([]);
  let loading = $state(true);
  let error = $state<string | null>(null);
  let showAddForm = $state(false);
  let editingMedication = $state<Medication | null>(null);
  // Get active medications (those without end_date or with end_date in the future)
  const activeMedications = $derived(
    medications.filter((m) => {
      if (!m.end_date) return true;
      const endDate = new Date(m.end_date);
      const today = new Date();
      endDate.setHours(0, 0, 0, 0);
      today.setHours(0, 0, 0, 0);
      return endDate >= today;
    })
  );

  onMount(async () => {
    await loadMedications();
  });

  async function loadMedications() {
    try {
      loading = true;
      error = null;
      medications = await listMedicationsForPatient(patientId);
    } catch (err) {
      error =
        get(t)('common.loadFailed') + ': ' + (err instanceof Error ? err.message : String(err));
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
    if (!confirm(get(t)('medications.confirmDelete'))) {
      return;
    }

    try {
      await deleteMedication(medicationId);
      await loadMedications();
    } catch (err) {
      error =
        get(t)('common.deleteFailed') + ': ' + (err instanceof Error ? err.message : String(err));
      console.error('Failed to delete medication:', err);
    }
  }

  async function handleSave(
    input: CreateMedication | { id: string; update: UpdateMedication },
    replacingMedicationId?: string | null
  ) {
    try {
      error = null;

      if ('id' in input) {
        // Update existing medication
        await updateMedication(input.id, input.update);
      } else {
        // Create new medication
        await createMedication(input);

        // If replacing an existing medication, end-date it
        if (replacingMedicationId) {
          try {
            await updateMedication(replacingMedicationId, { end_date: input.start_date });
          } catch (updateErr) {
            error =
              get(t)('medications.replacementNotEnded') +
              (updateErr instanceof Error ? updateErr.message : String(updateErr));
            console.error('Failed to end-date replaced medication:', updateErr);
          }
        }
      }

      // Reset form
      showAddForm = false;
      editingMedication = null;
      await loadMedications();
    } catch (err) {
      error =
        get(t)('common.saveFailed') + ': ' + (err instanceof Error ? err.message : String(err));
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
        day: '2-digit',
      });
    } catch {
      return dateStr;
    }
  }

  function isActive(medication: Medication): boolean {
    if (!medication.end_date) return true;
    const endDate = new Date(medication.end_date);
    const today = new Date();
    endDate.setHours(0, 0, 0, 0);
    today.setHours(0, 0, 0, 0);
    return endDate >= today;
  }
</script>

<div class="p-8 max-w-4xl mx-auto">
  <div class="flex justify-between items-center mb-6">
    <h1 class="text-display font-semibold text-fg">{$t('medications.title')}</h1>
    <button
      class="h-8 px-3 bg-accent text-on-accent rounded-control hover:bg-accent-hover transition-colors"
      onclick={() => {
        showAddForm = !showAddForm;
        editingMedication = null;
      }}
    >
      {showAddForm ? $t('common.cancel') : `+ ${$t('medications.newMedication')}`}
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
        {editingMedication ? $t('medications.editMedication') : $t('medications.addMedication')}
      </h2>
      <MedicationForm
        medication={editingMedication || undefined}
        {patientId}
        {activeMedications}
        onSave={handleSave}
        onCancel={handleCancel}
      />
    </div>
  {/if}

  {#if loading}
    <div class="flex justify-center items-center py-12">
      <div class="text-fg-muted">{$t('common.loading')}</div>
    </div>
  {:else if medications.length === 0}
    <div class="text-center py-12">
      <p class="text-fg-muted mb-4">{$t('medications.noMedications')}</p>
      {#if !showAddForm}
        <button
          class="h-8 px-3 bg-accent text-on-accent rounded-control hover:bg-accent-hover transition-colors"
          onclick={() => (showAddForm = true)}
        >
          {$t('medications.addFirst')}
        </button>
      {/if}
    </div>
  {:else}
    <div class="grid gap-4">
      {#each medications as medication (medication.id)}
        <div class="p-4 bg-surface-raised rounded-card border border-line">
          <div class="flex justify-between items-start mb-2">
            <div class="flex-1">
              <div class="flex items-center gap-2 mb-1">
                <h3 class="text-heading font-semibold text-fg">
                  {medication.substance}
                </h3>
                {#if isActive(medication)}
                  <span
                    class="px-2 py-0.5 rounded-full text-caption bg-success-subtle text-success-fg border border-success-line"
                  >
                    {$t('medications.active')}
                  </span>
                {:else}
                  <span
                    class="px-2 py-0.5 rounded-full text-caption bg-surface-selected/20 text-fg-muted border border-line-strong/30"
                  >
                    {$t('medications.ended')}
                  </span>
                {/if}
              </div>
              <p class="text-body text-fg-muted">
                {medication.dosage} • {medication.frequency}
              </p>
              <p class="text-body text-fg-muted mt-1">
                {$t('medications.dateRangeFrom').replace(
                  '{date}',
                  formatDate(medication.start_date)
                )}
                {#if medication.end_date}
                  {$t('medications.dateRangeTo').replace('{date}', formatDate(medication.end_date))}
                {/if}
              </p>
              {#if medication.notes}
                <p class="text-body text-fg-muted mt-2">{medication.notes}</p>
              {/if}
            </div>
            <div class="flex gap-2 ml-2">
              <button
                type="button"
                class="p-2 text-fg-muted hover:text-accent-fg hover:bg-surface-hover rounded-control transition-colors"
                onclick={() => handleEdit(medication)}
                title={$t('common.edit')}
              >
                <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
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
                class="p-2 text-fg-muted hover:text-danger-fg hover:bg-surface-hover rounded-control transition-colors"
                onclick={() => handleDelete(medication.id)}
                title={$t('common.delete')}
              >
                <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
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
