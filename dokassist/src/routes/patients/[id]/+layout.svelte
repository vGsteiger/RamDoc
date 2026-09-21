<script lang="ts">
  import { onMount } from 'svelte';
  import { page } from '$app/stores';
  import { getPatient, type Patient } from '$lib/api';
  import { Hourglass } from 'lucide-svelte';
  import type { Snippet } from 'svelte';
  import { t } from '$lib/translations';

  let { children }: { children: Snippet } = $props();

  let patientId = $derived($page.params.id!);
  let patient = $state<Patient | null>(null);
  let isLoading = $state(true);
  let errorMessage = $state('');

  let currentPath = $derived($page.url.pathname);

  let tabs = $derived([
    { path: `/patients/${patientId}`, labelKey: 'patients.overview', exact: true },
    { path: `/patients/${patientId}/sessions`, labelKey: 'patients.sessions', exact: false },
    { path: `/patients/${patientId}/medications`, labelKey: 'patients.medications', exact: false },
    { path: `/patients/${patientId}/diagnoses`, labelKey: 'patients.diagnoses', exact: false },
    {
      path: `/patients/${patientId}/treatment-plans`,
      labelKey: 'treatmentPlans.title',
      exact: false,
    },
    { path: `/patients/${patientId}/files`, labelKey: 'patients.files', exact: false },
    { path: `/patients/${patientId}/reports`, labelKey: 'patients.reports', exact: false },
    { path: `/patients/${patientId}/email`, labelKey: 'patients.email', exact: false },
    { path: `/patients/${patientId}/chat`, labelKey: 'patients.chat', exact: false },
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
        <div class="mb-4 flex justify-center text-fg-subtle">
          <Hourglass size={48} />
        </div>
        <p class="text-fg-muted">{$t('patients.loadingPatient')}</p>
      </div>
    </div>
  {:else if errorMessage}
    <div class="flex-1 flex items-center justify-center p-8">
      <div class="bg-danger-subtle border border-danger-line rounded-card p-6 max-w-md">
        <p class="text-danger-fg">{$t('patients.loadError')}</p>
      </div>
    </div>
  {:else if patient}
    <div class="bg-surface-sunken border-b border-line-subtle px-4 py-4 sm:px-6 sm:py-5">
      <h1 class="text-display font-semibold text-fg mb-1">
        {patient.first_name}
        {patient.last_name}
      </h1>
      {#if patient.date_of_birth}
        <p class="text-fg-muted">
          {$t('patients.bornOn')}
          {patient.date_of_birth}
        </p>
      {/if}
    </div>

    <div class="overflow-x-auto bg-surface-sunken border-b border-line-subtle">
      <nav class="flex min-w-max gap-1 px-3 sm:px-6" aria-label="Patient workspace">
        {#each tabs as tab}
          <a
            href={tab.path}
            aria-current={(
              tab.exact
                ? currentPath === tab.path
                : currentPath === tab.path || currentPath.startsWith(tab.path + '/')
            )
              ? 'page'
              : undefined}
            class="shrink-0 px-3 py-3 text-body font-medium transition-colors sm:px-4 {(
              tab.exact
                ? currentPath === tab.path
                : currentPath === tab.path || currentPath.startsWith(tab.path + '/')
            )
              ? 'text-accent-fg border-b-2 border-accent'
              : 'text-fg-muted hover:text-fg'}"
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
