<script lang="ts">
  import { errorText } from '$lib/translations/labels';
  import { get } from 'svelte/store';
  import { t } from '$lib/translations';
  import { goto } from '$app/navigation';
  import { createPatient, parseError, type CreatePatient, type UpdatePatient } from '$lib/api';
  import PatientForm from '$lib/components/PatientForm.svelte';
  import { addToast } from '$lib/stores/toast';
  import { Alert, Card, PageShell } from '$lib/components/ui';

  let isSubmitting = $state(false);
  let error = $state('');

  async function handleSubmit(
    event: CustomEvent<CreatePatient | { id: string; data: UpdatePatient }>
  ) {
    if ('id' in event.detail) return;
    try {
      isSubmitting = true;
      error = '';
      const patient = await createPatient(event.detail);
      addToast(get(t)('patients.created'));
      goto(`/patients/${patient.id}`);
    } catch (e) {
      const { code } = parseError(e);
      if (code === 'DB_UNIQUE_CONSTRAINT') {
        error = get(t)('patients.duplicateAhv');
      } else {
        error = $errorText(e, get(t)('patients.createFailed'));
      }
      console.error('Error creating patient:', e);
      isSubmitting = false;
    }
  }

  function handleCancel() {
    goto('/patients');
  }
</script>

<PageShell title={$t('patients.newPatient')} width="form">
  {#if error}
    <Alert tone="danger" class="mb-4">{error}</Alert>
  {/if}

  <Card>
    <PatientForm on:submit={handleSubmit} on:cancel={handleCancel} {isSubmitting} />
  </Card>
</PageShell>
