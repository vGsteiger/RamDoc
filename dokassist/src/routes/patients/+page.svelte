<script lang="ts">
  import { errorText } from '$lib/translations/labels';
  import { get } from 'svelte/store';
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { listPatients, globalSearch, type Patient } from '$lib/api';
  import PatientCard from '$lib/components/PatientCard.svelte';
  import {
    Alert,
    Button,
    EmptyState,
    Input,
    ListSkeleton,
    PageHeader,
    Select,
  } from '$lib/components/ui';
  import { Plus, Users } from 'lucide-svelte';
  import { t } from '$lib/translations';

  let patients = $state<Patient[]>([]);
  let filteredPatients = $state<Patient[]>([]);
  let searchQuery = $state('');
  let isLoading = $state(true);
  let error = $state('');
  let sortBy = $state<'name' | 'created'>('name');

  // Debounced search
  let searchTimeout: ReturnType<typeof setTimeout> | null = null;

  onMount(async () => {
    await loadPatients();
  });

  async function loadPatients() {
    try {
      isLoading = true;
      error = '';
      patients = await listPatients(100, 0);
      filteredPatients = patients;
    } catch (e) {
      error = $errorText(e, get(t)('patients.loadFailed'));
      console.error('Error loading patients:', e);
    } finally {
      isLoading = false;
    }
  }

  async function handleSearch(query: string) {
    searchQuery = query;

    // Clear existing timeout
    if (searchTimeout) {
      clearTimeout(searchTimeout);
    }

    // Debounce search by 300ms
    searchTimeout = setTimeout(async () => {
      if (!query.trim()) {
        filteredPatients = patients;
        return;
      }

      try {
        const results = await globalSearch(query, 50);
        // Filter to only patient results
        const patientResults = results.filter((r) => r.result_type === 'patient');

        // Get patient IDs from search results
        const patientIds = new Set(patientResults.map((r) => r.entity_id));

        // Filter patients by search results
        filteredPatients = patients.filter((p) => patientIds.has(p.id));
      } catch (e) {
        console.error('Search error:', e);
        // On search error, fall back to client-side filtering
        const lowerQuery = query.toLowerCase();
        filteredPatients = patients.filter(
          (p) =>
            p.first_name.toLowerCase().includes(lowerQuery) ||
            p.last_name.toLowerCase().includes(lowerQuery) ||
            (p.ahv_number?.includes(query) ?? false)
        );
      }
    }, 300);
  }

  function sortPatients(pats: Patient[]): Patient[] {
    if (sortBy === 'name') {
      return [...pats].sort((a, b) => {
        const nameA = `${a.last_name} ${a.first_name}`.toLowerCase();
        const nameB = `${b.last_name} ${b.first_name}`.toLowerCase();
        return nameA.localeCompare(nameB);
      });
    } else {
      return [...pats].sort((a, b) => {
        return new Date(b.created_at).getTime() - new Date(a.created_at).getTime();
      });
    }
  }

  let sortedPatients = $derived(sortPatients(filteredPatients));

  function handlePatientClick(patientId: string) {
    goto(`/patients/${patientId}`);
  }

  function handleNewPatient() {
    goto('/patients/new');
  }
</script>

<div class="p-8">
  <div class="max-w-7xl mx-auto">
    <PageHeader title={$t('patients.title')}>
      {#snippet actions()}
        <Button variant="primary" onclick={handleNewPatient}>
          <Plus size={14} />
          {$t('patients.newPatient')}
        </Button>
      {/snippet}
    </PageHeader>

    <div class="flex gap-2 mb-4">
      <Input
        type="search"
        class="flex-1"
        placeholder={$t('patients.search')}
        bind:value={searchQuery}
        oninput={(e: Event) => handleSearch((e.currentTarget as HTMLInputElement).value)}
      />
      <Select bind:value={sortBy} class="w-48">
        <option value="name">{$t('patients.sortByName')}</option>
        <option value="created">{$t('patients.sortByCreated')}</option>
      </Select>
    </div>

    {#if !isLoading}
      <div class="mb-4 text-caption text-fg-subtle">
        {sortedPatients.length}
        {sortedPatients.length === 1 ? $t('patients.patient') : $t('patients.patients')}
        {#if searchQuery}
          {$t('patients.matching')} "{searchQuery}"
        {/if}
      </div>
    {/if}

    {#if isLoading}
      <ListSkeleton count={5} />
    {:else if error}
      <Alert tone="danger">{error}</Alert>
    {:else if sortedPatients.length === 0}
      <EmptyState
        icon={Users}
        title={searchQuery ? $t('patients.noSearchResults') : $t('patients.noPatients')}
      >
        {#snippet action()}
          {#if !searchQuery}
            <Button variant="primary" onclick={handleNewPatient}>
              {$t('patients.createFirst')}
            </Button>
          {/if}
        {/snippet}
      </EmptyState>
    {:else}
      <div class="grid gap-2">
        {#each sortedPatients as patient (patient.id)}
          <PatientCard {patient} onclick={() => handlePatientClick(patient.id)} />
        {/each}
      </div>
    {/if}
  </div>
</div>
