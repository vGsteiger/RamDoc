<script lang="ts">
  import { get } from 'svelte/store';
  import { ChevronDown, ChevronUp, Pencil, Trash2 } from 'lucide-svelte';
  import {
    listTreatmentGoalsForPlan,
    listTreatmentInterventionsForPlan,
    type TreatmentGoal,
    type TreatmentIntervention,
    type TreatmentPlan,
  } from '$lib/api';
  import { Badge, Card, IconButton, Menu, Spinner } from '$lib/components/ui';
  import { fmt } from '$lib/format';
  import { t } from '$lib/translations';
  import { planStatusLabel } from '$lib/translations/labels';
  import GoalList from './GoalList.svelte';
  import InterventionList from './InterventionList.svelte';
  import { errorMessage, statusTone } from './options';

  /*
   * One treatment plan. Expanding it loads its goals and interventions; only
   * one plan is expanded at a time (the page owns `expanded`).
   */
  let {
    plan,
    expanded = false,
    onToggle,
    onEdit,
    onDelete,
    onError,
  }: {
    plan: TreatmentPlan;
    expanded?: boolean;
    onToggle: () => void;
    onEdit: () => void;
    onDelete: () => void;
    onError: (message: string) => void;
  } = $props();

  let goals = $state<TreatmentGoal[]>([]);
  let interventions = $state<TreatmentIntervention[]>([]);
  let loading = $state(false);
  const detailsId = $props.id();

  async function loadDetails(showSpinner = false) {
    try {
      if (showSpinner) loading = true;
      const [loadedGoals, loadedInterventions] = await Promise.all([
        listTreatmentGoalsForPlan(plan.id),
        listTreatmentInterventionsForPlan(plan.id),
      ]);
      goals = loadedGoals;
      interventions = loadedInterventions;
    } catch (err) {
      onError(errorMessage(get(t)('common.loadFailed'), err));
      console.error('Failed to load plan details:', err);
    } finally {
      loading = false;
    }
  }

  $effect(() => {
    if (expanded) {
      goals = [];
      interventions = [];
      void loadDetails(true);
    }
  });
</script>

<Card padding="none">
  <div class="flex items-start gap-3 px-4 py-3">
    <div class="min-w-0 flex-1">
      <div class="flex flex-wrap items-center gap-2">
        <h3 class="min-w-0 text-heading text-fg [overflow-wrap:anywhere]">{plan.title}</h3>
        <Badge tone={statusTone(plan.status)}>{$planStatusLabel(plan.status)}</Badge>
      </div>
      {#if plan.description}
        <p class="mt-1 text-body text-fg-muted">{plan.description}</p>
      {/if}
      <p class="mt-1 text-caption text-fg-subtle" data-numeric>
        {plan.end_date
          ? $t('treatmentPlans.dateRange')
              .replace('{start}', $fmt.date(plan.start_date))
              .replace('{end}', $fmt.date(plan.end_date))
          : $fmt.date(plan.start_date)}
      </p>
    </div>
    <div class="flex shrink-0 items-center gap-1">
      <Menu
        size="sm"
        label={$t('common.moreActions')}
        items={[
          { label: $t('treatmentPlans.editPlan'), icon: Pencil, onSelect: onEdit },
          {
            label: $t('treatmentPlans.deletePlan'),
            icon: Trash2,
            danger: true,
            onSelect: onDelete,
          },
        ]}
      />
      <IconButton
        size="sm"
        label={expanded ? $t('treatmentPlans.collapse') : $t('treatmentPlans.viewDetails')}
        aria-expanded={expanded}
        aria-controls={detailsId}
        onclick={onToggle}
      >
        {#if expanded}
          <ChevronUp size={16} />
        {:else}
          <ChevronDown size={16} />
        {/if}
      </IconButton>
    </div>
  </div>

  {#if expanded}
    <div id={detailsId} class="space-y-5 border-t border-line-subtle px-4 py-4">
      {#if loading}
        <Spinner label={$t('treatmentPlans.loadingDetails')} />
      {:else}
        <GoalList planId={plan.id} {goals} onChange={() => loadDetails()} {onError} />
        <InterventionList
          planId={plan.id}
          {interventions}
          onChange={() => loadDetails()}
          {onError}
        />
      {/if}
    </div>
  {/if}
</Card>
