<script lang="ts">
  import { onMount } from 'svelte';
  import { page } from '$app/stores';
  import { getPatient, type Patient } from '$lib/api';
  import { Hourglass } from 'lucide-svelte';
  import type { Snippet } from 'svelte';
  import { t } from '$lib/translations';

  let { children }: { children: Snippet } = $props();

  let patientId = $derived($page.params.id);
  let patient = $state<Patient | null>(null);
  let isLoading = $state(true);
  let errorMessage = $state('');

  let currentPath = $derived($page.url.pathname);

  let tabs = $derived([
    { path: `/patients/${patientId}`, labelKey: 'patients.overview' },
    { path: `/patients/${patientId}/files`, labelKey: 'patients.files' },
    { path: `/patients/${patientId}/reports`, labelKey: 'patients.reports' },
    { path: `/patients/${patientId}/email`, labelKey: 'patients.email' },
    { path: `/patients/${patientId}/chat`, labelKey: 'patients.chat' },
  ]);

  onMount(async () => {
    try {
      patient = await getPatient(patientId);
    } catch (error) {
      console.error('Failed to load patient:', error);
      errorMessage = 'error';
    } finally {
      isLoading = false;
    }
  });
</script>

<div class="h-full flex flex-col">
  {#if isLoading}
    <div class="flex-1 flex items-center justify-center">
      <div class="text-center">
        <div class="mb-4 flex justify-center text-gray-400 dark:text-gray-400">
          <Hourglass size={48} />
        </div>
        <p class="text-gray-500 dark:text-gray-400">{$t('patients.loadingPatient')}</p>
      </div>
    </div>
  {:else if errorMessage}
    <div class="flex-1 flex items-center justify-center p-8">
      <div
        class="bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 rounded-lg p-6 max-w-md"
      >
        <p class="text-red-600 dark:text-red-400">{$t('patients.loadError')}</p>
      </div>
    </div>
  {:else if patient}
    <div class="bg-gray-50 dark:bg-gray-900 border-b border-gray-200 dark:border-gray-800 p-6">
      <h1 class="text-2xl font-bold text-gray-900 dark:text-gray-100 mb-2">
        {patient.first_name}
        {patient.last_name}
      </h1>
      {#if patient.date_of_birth}
        <p class="text-gray-500 dark:text-gray-400">
          {$t('patients.bornOn')}
          {patient.date_of_birth}
        </p>
      {/if}
    </div>

    <div class="bg-gray-50 dark:bg-gray-900 border-b border-gray-200 dark:border-gray-800">
      <nav class="flex gap-1 px-6">
        {#each tabs as tab}
          <a
            href={tab.path}
            class="px-4 py-3 font-medium transition-colors {currentPath === tab.path
              ? 'text-blue-500 dark:text-blue-400 border-b-2 border-blue-500 dark:border-blue-400'
              : 'text-gray-500 dark:text-gray-400 hover:text-gray-700 dark:hover:text-gray-300'}"
          >
            {$t(tab.labelKey)}
          </a>
        {/each}
      </nav>
    </div>

    <div class="flex-1 overflow-auto">
      {@render children()}
    </div>
  {/if}
</div>
