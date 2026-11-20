<script lang="ts">
  import { onMount } from 'svelte';
  import { page } from '$app/state';
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
  import {
    Alert,
    Button,
    ConfirmDialog,
    Dialog,
    EmptyState,
    Field,
    Input,
    ListSkeleton,
    PageShell,
    SectionHeader,
    Select,
    Textarea,
  } from '$lib/components/ui';
  import { usePatientWorkspace } from '$lib/components/patient/state.svelte';
  import { Plus, Stethoscope } from 'lucide-svelte';
  import { get } from 'svelte/store';
  import { t } from '$lib/translations';

  const patientId = $derived(page.params.id!);

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

  // Stored values; only the labels are translated.
  const statusOptions = ['active', 'remission', 'resolved'];
  const workspace = usePatientWorkspace();

  let formError = $state<string | null>(null);
  let deleting = $state<Diagnosis | null>(null);
  let confirmOpen = $state(false);

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

  function askDelete(diagnosis: Diagnosis) {
    deleting = diagnosis;
    confirmOpen = true;
  }

  async function confirmDelete() {
    if (!deleting) return;
    try {
      await deleteDiagnosis(deleting.id);
      await loadDiagnoses();
      void workspace()?.refreshRecords();
    } catch (err) {
      error =
        get(t)('common.deleteFailed') + ': ' + (err instanceof Error ? err.message : String(err));
      console.error('Failed to delete diagnosis:', err);
    }
  }

  async function handleSubmit(event: Event) {
    event.preventDefault();

    if (!selectedCode || !selectedDescription) {
      formError = get(t)('diagnoses.selectRequired');
      return;
    }

    try {
      saving = true;
      formError = null;

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
      void workspace()?.refreshRecords();
    } catch (err) {
      formError =
        get(t)('common.saveFailed') + ': ' + (err instanceof Error ? err.message : String(err));
      console.error('Failed to save diagnosis:', err);
    } finally {
      saving = false;
    }
  }

  function openNew() {
    resetForm();
    showAddForm = true;
  }

  function resetForm() {
    showAddForm = false;
    formError = null;
    editingId = null;
    selectedCode = '';
    selectedDescription = '';
    diagnosedDate = new Date().toISOString().split('T')[0];
    status = 'active';
    resolvedDate = '';
    notes = '';
  }
</script>

<PageShell>
  <SectionHeader title={$t('diagnoses.title')}>
    {#snippet actions()}
      <Button size="sm" onclick={openNew}>
        <Plus size={14} aria-hidden="true" />
        {$t('diagnoses.newDiagnosis')}
      </Button>
    {/snippet}
  </SectionHeader>

  {#if error}
    <Alert tone="danger" class="mb-4">{error}</Alert>
  {/if}

  {#if loading}
    <ListSkeleton variant="file" count={4} />
  {:else if diagnoses.length === 0}
    <EmptyState icon={Stethoscope} title={$t('diagnoses.noDiagnoses')}>
      {#snippet action()}
        <Button onclick={openNew}>{$t('diagnoses.addFirst')}</Button>
      {/snippet}
    </EmptyState>
  {:else}
    <div class="rounded-card border border-line bg-surface-raised">
      {#each diagnoses as diagnosis (diagnosis.id)}
        <DiagnosisCard
          {diagnosis}
          onEdit={() => handleEdit(diagnosis)}
          onDelete={() => askDelete(diagnosis)}
        />
      {/each}
    </div>
  {/if}
</PageShell>

<Dialog
  bind:open={showAddForm}
  size="lg"
  title={editingId ? $t('diagnoses.editDiagnosis') : $t('diagnoses.addDiagnosis')}
  onClose={resetForm}
>
  <form id="diagnosis-form" onsubmit={handleSubmit} class="space-y-4">
    {#if formError}
      <Alert tone="danger">{formError}</Alert>
    {/if}
    <Field label={$t('diagnoses.icdCode')} for="icd-search" required>
      <IcdSearch id="icd-search" onSelect={handleIcdSelect} />
      {#if selectedCode}
        <p
          class="mt-2 flex min-w-0 items-baseline gap-2 rounded-control border border-line-subtle bg-surface-sunken px-2.5 py-1.5"
        >
          <span class="shrink-0 font-mono text-label text-fg">{selectedCode}</span>
          <span class="min-w-0 text-label text-fg-muted">{selectedDescription}</span>
        </p>
      {/if}
    </Field>

    <div class="grid grid-cols-1 gap-4 sm:grid-cols-2">
      <Field label={$t('diagnoses.diagnosedOn')} for="diagnosed-date" required>
        <Input id="diagnosed-date" type="date" bind:value={diagnosedDate} required />
      </Field>
      <Field label={$t('diagnoses.status')} for="status" required>
        <Select id="status" bind:value={status} required>
          {#each statusOptions as option (option)}
            <option value={option}>{$t(`diagnoses.${option}`)}</option>
          {/each}
        </Select>
      </Field>
    </div>

    {#if status === 'resolved'}
      <Field label={$t('diagnoses.resolvedOn')} for="resolved-date">
        <Input id="resolved-date" type="date" bind:value={resolvedDate} />
      </Field>
    {/if}

    <Field label={$t('common.notes')} for="notes">
      <Textarea
        id="notes"
        bind:value={notes}
        rows={3}
        placeholder={$t('diagnoses.notesPlaceholder')}
      />
    </Field>
  </form>

  {#snippet footer()}
    <Button onclick={resetForm} disabled={saving}>{$t('common.cancel')}</Button>
    <Button type="submit" variant="primary" form="diagnosis-form" loading={saving}>
      {editingId ? $t('common.update') : $t('common.add')}
    </Button>
  {/snippet}
</Dialog>

<ConfirmDialog
  bind:open={confirmOpen}
  tone="danger"
  title={$t('diagnoses.deleteDiagnosis')}
  message={$t('diagnoses.confirmDeleteNamed')
    .replace('{code}', deleting?.icd10_code ?? '')
    .replace('{name}', deleting?.description ?? '')}
  onConfirm={confirmDelete}
/>
