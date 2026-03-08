<script lang="ts">
  import { onMount } from 'svelte';
  import { page } from '$app/stores';
  import { getPatient, type Patient } from '$lib/api';
  import { Hourglass } from 'lucide-svelte';

  let patientId = $derived($page.params.id);
  let patient = $state<Patient | null>(null);
  let isLoading = $state(true);
  let errorMessage = $state('');

  let currentPath = $derived($page.url.pathname);

  let tabs = $derived([
    { path: `/patients/${patientId}`, label: 'Overview' },
    { path: `/patients/${patientId}/files`, label: 'Files' },
    { path: `/patients/${patientId}/reports`, label: 'Reports' },
    { path: `/patients/${patientId}/email`, label: 'Email' },
    { path: `/patients/${patientId}/chat`, label: 'Chat' },
  ]);

  onMount(async () => {
    try {
      patient = await getPatient(patientId);
    } catch (error) {
      console.error('Failed to load patient:', error);
      errorMessage = 'Failed to load patient information';
    } finally {
      isLoading = false;
    }
  });
</script>

<div class="h-full flex flex-col">
  {#if isLoading}
    <div class="flex-1 flex items-center justify-center">
      <div class="text-center">
        <div class="mb-4 flex justify-center text-gray-400">
          <Hourglass size={48} />
        </div>
        <p class="text-gray-400">Loading patient...</p>
      </div>
    </div>
  {:else if errorMessage}
    <div class="flex-1 flex items-center justify-center p-8">
      <div class="bg-red-900/20 border border-red-800 rounded-lg p-6 max-w-md">
        <p class="text-red-400">{errorMessage}</p>
      </div>
    </div>
  {:else if patient}
    <div class="bg-gray-900 border-b border-gray-800 p-6">
      <h1 class="text-2xl font-bold text-gray-100 mb-2">
        {patient.first_name} {patient.last_name}
      </h1>
      {#if patient.date_of_birth}
        <p class="text-gray-400">Born {patient.date_of_birth}</p>
      {/if}
    </div>

    <div class="bg-gray-900 border-b border-gray-800">
      <nav class="flex gap-1 px-6">
        {#each tabs as tab}
          <a
            href={tab.path}
            class="px-4 py-3 font-medium transition-colors {currentPath === tab.path
              ? 'text-blue-400 border-b-2 border-blue-400'
              : 'text-gray-400 hover:text-gray-300'}"
          >
            {tab.label}
          </a>
        {/each}
      </nav>
    </div>

    <div class="flex-1 overflow-auto">
      <slot />
    </div>
  {/if}
</div>
