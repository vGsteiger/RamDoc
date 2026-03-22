<script lang="ts">
  import { goto } from '$app/navigation';
  import { createPatient, parseError, type CreatePatient } from '$lib/api';
  import PatientForm from '$lib/components/PatientForm.svelte';
  import { addToast } from '$lib/stores/toast';

  let isSubmitting = $state(false);
  let error = $state('');

  async function handleSubmit(event: CustomEvent<CreatePatient>) {
    try {
      isSubmitting = true;
      error = '';
      const patient = await createPatient(event.detail);
      addToast('Patient created');
      goto(`/patients/${patient.id}`);
    } catch (e) {
      const { code } = parseError(e);
      if (code === 'DB_UNIQUE_CONSTRAINT') {
        error = 'A patient with this AHV number already exists.';
      } else {
        error = e instanceof Error ? e.message : 'Failed to create patient';
      }
      console.error('Error creating patient:', e);
      isSubmitting = false;
    }
  }

  function handleCancel() {
    goto('/patients');
  }
</script>

<div class="p-8">
  <div class="max-w-3xl mx-auto">
    <h1 class="text-3xl font-bold text-gray-900 dark:text-gray-100 mb-6">New Patient</h1>

    {#if error}
      <div
        class="mb-6 bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 rounded-lg p-4 text-red-600 dark:text-red-400"
      >
        {error}
      </div>
    {/if}

    <div class="bg-white dark:bg-gray-800 rounded-lg p-6">
      <PatientForm on:submit={handleSubmit} on:cancel={handleCancel} {isSubmitting} />
    </div>
  </div>
</div>
