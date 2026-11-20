<script lang="ts">
  import { get } from 'svelte/store';
  import { goto } from '$app/navigation';
  import { updatePatient, type CreatePatient, type UpdatePatient } from '$lib/api';
  import PatientForm from '$lib/components/PatientForm.svelte';
  import { Alert, Card, PageShell, SectionHeader } from '$lib/components/ui';
  import { getPatientWorkspace } from '$lib/components/patient/state.svelte';
  import { patientPath } from '$lib/components/patient/workspace';
  import { addToast } from '$lib/stores/toast';
  import { t } from '$lib/translations';
  import { errorText } from '$lib/translations/labels';

  /* Edit the patient's master data; reached from the header's ⋯ menu. */
  const workspace = getPatientWorkspace();

  let isSubmitting = $state(false);
  let error = $state('');

  async function handleSubmit(
    event: CustomEvent<CreatePatient | { id: string; data: UpdatePatient }>
  ) {
    if (!('id' in event.detail)) return;
    try {
      isSubmitting = true;
      error = '';
      const updated = await updatePatient(event.detail.id, event.detail.data);
      workspace?.setPatient(updated);
      addToast(get(t)('patients.updated'));
      await goto(patientPath(updated.id, 'overview'));
    } catch (e) {
      error = get(errorText)(e, get(t)('patients.updateFailed'));
      console.error('Error updating patient:', e);
    } finally {
      isSubmitting = false;
    }
  }

  function handleCancel() {
    if (workspace) void goto(patientPath(workspace.id, 'overview'));
  }
</script>

<PageShell>
  <div>
    <SectionHeader title={$t('patients.editPatient')} />
    {#if error}
      <Alert tone="danger" class="mb-4">{error}</Alert>
    {/if}
    {#if workspace?.patient}
      <Card>
        <PatientForm
          patient={workspace.patient}
          on:submit={handleSubmit}
          on:cancel={handleCancel}
          {isSubmitting}
        />
      </Card>
    {/if}
  </div>
</PageShell>
