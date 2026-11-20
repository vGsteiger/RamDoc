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
  import {
    Alert,
    Badge,
    Button,
    ConfirmDialog,
    Dialog,
    EmptyState,
    ListSkeleton,
    Menu,
    PageShell,
    SectionHeader,
  } from '$lib/components/ui';
  import { usePatientWorkspace } from '$lib/components/patient/state.svelte';
  import { isMedicationActive } from '$lib/components/patient/workspace';
  import { fmt } from '$lib/format';
  import { Pencil, Pill, Plus, Trash2 } from 'lucide-svelte';
  import { get } from 'svelte/store';
  import { t } from '$lib/translations';

  const patientId = $derived(page.params.id!);

  let medications = $state<Medication[]>([]);
  let loading = $state(true);
  let error = $state<string | null>(null);
  let showAddForm = $state(false);
  let editingMedication = $state<Medication | null>(null);
  const workspace = usePatientWorkspace();
  let deleting = $state<Medication | null>(null);
  let confirmOpen = $state(false);
  // Get active medications (those without end_date or with end_date in the future)
  const activeMedications = $derived(medications.filter((m) => isMedicationActive(m)));

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

  function askDelete(medication: Medication) {
    deleting = medication;
    confirmOpen = true;
  }

  async function confirmDelete() {
    if (!deleting) return;
    try {
      await deleteMedication(deleting.id);
      await loadMedications();
      void workspace()?.refreshRecords();
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
      void workspace()?.refreshRecords();
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

  function openNew() {
    editingMedication = null;
    showAddForm = true;
  }
</script>

<PageShell>
  <SectionHeader title={$t('medications.title')}>
    {#snippet actions()}
      <Button size="sm" onclick={openNew}>
        <Plus size={14} aria-hidden="true" />
        {$t('medications.newMedication')}
      </Button>
    {/snippet}
  </SectionHeader>

  {#if error}
    <Alert tone="danger" class="mb-4">{error}</Alert>
  {/if}

  {#if loading}
    <ListSkeleton variant="file" count={4} />
  {:else if medications.length === 0}
    <EmptyState icon={Pill} title={$t('medications.noMedications')}>
      {#snippet action()}
        <Button onclick={openNew}>{$t('medications.addFirst')}</Button>
      {/snippet}
    </EmptyState>
  {:else}
    <ul
      class="rounded-card border border-line bg-surface-raised"
      aria-label={$t('medications.title')}
    >
      {#each medications as medication (medication.id)}
        {@const active = isMedicationActive(medication)}
        <li class="flex items-start gap-3 border-b border-line-subtle px-3 py-2.5 last:border-b-0">
          <div class="min-w-0 flex-1">
            <div class="flex flex-wrap items-center gap-x-2 gap-y-1">
              <h3 class="min-w-0 text-body font-medium text-fg [overflow-wrap:anywhere]">
                {medication.substance}
              </h3>
              <span class="text-label text-fg-muted">
                {[medication.dosage, medication.frequency].filter(Boolean).join(' · ')}
              </span>
              <Badge tone={active ? 'success' : 'neutral'}>
                {active ? $t('medications.active') : $t('medications.ended')}
              </Badge>
            </div>
            <p class="mt-0.5 text-caption text-fg-muted" data-numeric>
              {$t('medications.dateRangeFrom').replace('{date}', $fmt.date(medication.start_date))}
              {#if medication.end_date}
                {$t('medications.dateRangeTo').replace('{date}', $fmt.date(medication.end_date))}
              {/if}
            </p>
            {#if medication.notes}
              <p class="mt-1 text-label text-fg-muted">{medication.notes}</p>
            {/if}
          </div>
          <Menu
            size="sm"
            label={$t('common.moreActions')}
            items={[
              {
                label: $t('medications.editMedication'),
                icon: Pencil,
                onSelect: () => handleEdit(medication),
              },
              {
                label: $t('medications.deleteMedication'),
                icon: Trash2,
                danger: true,
                onSelect: () => askDelete(medication),
              },
            ]}
          />
        </li>
      {/each}
    </ul>
  {/if}
</PageShell>

<Dialog
  bind:open={showAddForm}
  size="lg"
  title={editingMedication ? $t('medications.editMedication') : $t('medications.addMedication')}
  onClose={handleCancel}
>
  {#if error}
    <Alert tone="danger" class="mb-4">{error}</Alert>
  {/if}
  {#key editingMedication?.id ?? 'new'}
    <MedicationForm
      medication={editingMedication || undefined}
      {patientId}
      {activeMedications}
      onSave={handleSave}
      onCancel={handleCancel}
    />
  {/key}
</Dialog>

<ConfirmDialog
  bind:open={confirmOpen}
  tone="danger"
  title={$t('medications.deleteMedication')}
  message={$t('medications.confirmDeleteNamed').replace(
    '{name}',
    deleting ? [deleting.substance, deleting.dosage].filter(Boolean).join(' ') : ''
  )}
  onConfirm={confirmDelete}
/>
