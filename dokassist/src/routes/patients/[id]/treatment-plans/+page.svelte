<script lang="ts">
  import { get } from 'svelte/store';
  import { onMount } from 'svelte';
  import { page } from '$app/state';
  import { ClipboardList, Plus } from 'lucide-svelte';
  import { deleteTreatmentPlan, listTreatmentPlansForPatient, type TreatmentPlan } from '$lib/api';
  import {
    Alert,
    Button,
    ConfirmDialog,
    EmptyState,
    ListSkeleton,
    PageShell,
    SectionHeader,
  } from '$lib/components/ui';
  import TreatmentPlanCard from '$lib/components/treatment-plans/TreatmentPlanCard.svelte';
  import TreatmentPlanForm from '$lib/components/treatment-plans/TreatmentPlanForm.svelte';
  import { errorMessage } from '$lib/components/treatment-plans/options';
  import { t } from '$lib/translations';

  const patientId = $derived(page.params.id!);

  let plans = $state<TreatmentPlan[]>([]);
  let loading = $state(true);
  let error = $state<string | null>(null);
  let expandedId = $state<string | null>(null);

  let formOpen = $state(false);
  let editing = $state<TreatmentPlan | null>(null);
  let deleting = $state<TreatmentPlan | null>(null);
  let confirmOpen = $state(false);

  onMount(loadPlans);

  async function loadPlans() {
    try {
      loading = plans.length === 0;
      error = null;
      plans = await listTreatmentPlansForPatient(patientId);
    } catch (err) {
      error = errorMessage(get(t)('common.loadFailed'), err);
      console.error('Failed to load treatment plans:', err);
    } finally {
      loading = false;
    }
  }

  function openForm(plan: TreatmentPlan | null) {
    editing = plan;
    formOpen = true;
  }

  function askDelete(plan: TreatmentPlan) {
    deleting = plan;
    confirmOpen = true;
  }

  async function confirmDelete() {
    if (!deleting) return;
    const id = deleting.id;
    try {
      await deleteTreatmentPlan(id);
      if (expandedId === id) expandedId = null;
      await loadPlans();
    } catch (err) {
      error = errorMessage(get(t)('common.deleteFailed'), err);
      console.error('Failed to delete plan:', err);
    }
  }
</script>

<PageShell>
  <SectionHeader title={$t('treatmentPlans.title')}>
    {#snippet actions()}
      <Button size="sm" onclick={() => openForm(null)}>
        <Plus size={14} aria-hidden="true" />
        {$t('treatmentPlans.newPlan')}
      </Button>
    {/snippet}
  </SectionHeader>

  {#if error}
    <Alert tone="danger" class="mb-4">{error}</Alert>
  {/if}

  {#if loading}
    <ListSkeleton variant="file" count={2} />
  {:else if plans.length === 0}
    <EmptyState icon={ClipboardList} title={$t('treatmentPlans.noPlans')}>
      {#snippet action()}
        <Button onclick={() => openForm(null)}>{$t('treatmentPlans.addFirst')}</Button>
      {/snippet}
    </EmptyState>
  {:else}
    <div class="grid gap-3">
      {#each plans as plan (plan.id)}
        <TreatmentPlanCard
          {plan}
          expanded={expandedId === plan.id}
          onToggle={() => (expandedId = expandedId === plan.id ? null : plan.id)}
          onEdit={() => openForm(plan)}
          onDelete={() => askDelete(plan)}
          onError={(message) => (error = message)}
        />
      {/each}
    </div>
  {/if}
</PageShell>

<TreatmentPlanForm bind:open={formOpen} {patientId} plan={editing} onSaved={loadPlans} />

<ConfirmDialog
  bind:open={confirmOpen}
  tone="danger"
  title={$t('treatmentPlans.deletePlan')}
  message={$t('treatmentPlans.confirmDeletePlanNamed').replace('{name}', deleting?.title ?? '')}
  onConfirm={confirmDelete}
/>
