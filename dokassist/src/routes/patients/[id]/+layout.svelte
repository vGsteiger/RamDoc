<script lang="ts">
  import { onMount } from 'svelte';
  import { page } from '$app/stores';
  import { getPatient, type Patient } from '$lib/api';
  import PatientTabs from '$lib/components/PatientTabs.svelte';

  let patient = $state<Patient | null>(null);
  let isLoading = $state(true);
  let error = $state('');

  let patientId = $derived($page.params.id);

  onMount(async () => {
    await loadPatient();
  });

  async function loadPatient() {
    if (!patientId) {
      error = 'No patient ID provided';
      isLoading = false;
      return;
    }

    try {
      isLoading = true;
      error = '';
      patient = await getPatient(patientId);
    } catch (e) {
      error = e instanceof Error ? e.message : 'Failed to load patient';
      console.error('Error loading patient:', e);
    } finally {
      isLoading = false;
    }
  }

  // Expose patient to child routes via context if needed
  // For now, child routes can fetch their own data
</script>

<div class="flex flex-col h-full">
  {#if isLoading}
    <div class="flex justify-center items-center flex-1">
      <div class="text-gray-400">Loading patient...</div>
    </div>
  {:else if error}
    <div class="p-8">
      <div class="bg-red-900/20 border border-red-800 rounded-lg p-4 text-red-400">
        {error}
      </div>
    </div>
  {:else if patient}
    <!-- Patient Header -->
    <div class="bg-gray-900 border-b border-gray-800 p-6">
      <div class="max-w-7xl mx-auto">
        <div class="flex justify-between items-start">
          <div>
            <h1 class="text-3xl font-bold text-gray-100 mb-2">
              {patient.last_name}, {patient.first_name}
            </h1>
            <div class="flex gap-4 text-sm text-gray-400">
              <span>AHV: {patient.ahv_number}</span>
              <span>DOB: {patient.date_of_birth}</span>
              {#if patient.gender}
                <span>{patient.gender}</span>
              {/if}
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- Tab Navigation -->
    <PatientTabs {patientId} />

    <!-- Tab Content -->
    <div class="flex-1 overflow-auto">
      <slot />
    </div>
  {/if}
</div>
