<script lang="ts">
  import { t } from '$lib/translations';
  import { get } from 'svelte/store';
  import {
    planStatusLabel,
    goalStatusLabel,
    interventionTypeLabel,
  } from '$lib/translations/labels';
  import { onMount } from 'svelte';
  import { page } from '$app/stores';
  import {
    listTreatmentPlansForPatient,
    createTreatmentPlan,
    updateTreatmentPlan,
    deleteTreatmentPlan,
    listTreatmentGoalsForPlan,
    createTreatmentGoal,
    updateTreatmentGoal,
    deleteTreatmentGoal,
    listTreatmentInterventionsForPlan,
    createTreatmentIntervention,
    updateTreatmentIntervention,
    deleteTreatmentIntervention,
    type TreatmentPlan,
    type CreateTreatmentPlan,
    type UpdateTreatmentPlan,
    type TreatmentGoal,
    type CreateTreatmentGoal,
    type UpdateTreatmentGoal,
    type TreatmentIntervention,
    type CreateTreatmentIntervention,
    type UpdateTreatmentIntervention,
  } from '$lib/api';

  const patientId = $derived($page.params.id);

  let plans = $state<TreatmentPlan[]>([]);
  let loading = $state(true);
  let error = $state<string | null>(null);
  let showAddForm = $state(false);

  // Selected plan for viewing goals/interventions
  let selectedPlanId = $state<string | null>(null);
  let goals = $state<TreatmentGoal[]>([]);
  let interventions = $state<TreatmentIntervention[]>([]);
  let loadingDetails = $state(false);

  // Form state for plans
  let title = $state('');
  let description = $state('');
  let startDate = $state(new Date().toISOString().split('T')[0]);
  let endDate = $state('');
  let status = $state('active');
  let saving = $state(false);
  let editingId = $state<string | null>(null);

  // Form state for goals
  let showAddGoalForm = $state(false);
  let goalDescription = $state('');
  let goalTargetDate = $state('');
  let goalStatus = $state('in_progress');
  let goalSortOrder = $state(0);
  let editingGoalId = $state<string | null>(null);

  // Form state for interventions
  let showAddInterventionForm = $state(false);
  let interventionType = $state('psychotherapy');
  let interventionDescription = $state('');
  let interventionFrequency = $state('');
  let editingInterventionId = $state<string | null>(null);

  const statusOptions = ['active', 'completed', 'revised', 'discontinued'];

  const goalStatusOptions = ['pending', 'in_progress', 'achieved', 'revised', 'discontinued'];

  const interventionTypeOptions = ['psychotherapy', 'medication', 'referral', 'other'];

  onMount(async () => {
    await loadPlans();
  });

  async function loadPlans() {
    try {
      loading = true;
      error = null;
      plans = await listTreatmentPlansForPatient(patientId!);
    } catch (err) {
      error =
        get(t)('common.loadFailed') + ': ' + (err instanceof Error ? err.message : String(err));
      console.error('Failed to load treatment plans:', err);
    } finally {
      loading = false;
    }
  }

  async function loadPlanDetails(planId: string) {
    try {
      loadingDetails = true;
      selectedPlanId = planId;
      // Clear stale data immediately
      goals = [];
      interventions = [];
      resetGoalForm();
      resetInterventionForm();

      const [loadedGoals, loadedInterventions] = await Promise.all([
        listTreatmentGoalsForPlan(planId),
        listTreatmentInterventionsForPlan(planId),
      ]);
      goals = loadedGoals;
      interventions = loadedInterventions;
    } catch (err) {
      error =
        get(t)('common.loadFailed') + ': ' + (err instanceof Error ? err.message : String(err));
      console.error('Failed to load plan details:', err);
    } finally {
      loadingDetails = false;
    }
  }

  function handleEdit(plan: TreatmentPlan) {
    editingId = plan.id;
    title = plan.title;
    description = plan.description || '';
    startDate = plan.start_date;
    endDate = plan.end_date || '';
    status = plan.status;
    showAddForm = true;
  }

  async function handleDelete(planId: string) {
    if (!confirm(get(t)('treatmentPlans.confirmDeletePlan'))) {
      return;
    }

    try {
      await deleteTreatmentPlan(planId);
      if (selectedPlanId === planId) {
        selectedPlanId = null;
        goals = [];
        interventions = [];
      }
      await loadPlans();
    } catch (err) {
      error =
        get(t)('common.deleteFailed') + ': ' + (err instanceof Error ? err.message : String(err));
      console.error('Failed to delete plan:', err);
    }
  }

  async function handleSubmit(event: Event) {
    event.preventDefault();

    if (!title) {
      error = get(t)('treatmentPlans.titleRequired');
      return;
    }

    try {
      saving = true;
      error = null;

      if (editingId) {
        const update: UpdateTreatmentPlan = {
          title,
          description: description || undefined,
          start_date: startDate,
          end_date: endDate || undefined,
          status,
        };
        await updateTreatmentPlan(editingId, update);
      } else {
        const input: CreateTreatmentPlan = {
          patient_id: patientId!,
          title,
          description: description || undefined,
          start_date: startDate,
          end_date: endDate || undefined,
          status,
        };
        await createTreatmentPlan(input);
      }

      resetForm();
      await loadPlans();
    } catch (err) {
      error =
        get(t)('common.saveFailed') + ': ' + (err instanceof Error ? err.message : String(err));
      console.error('Failed to save plan:', err);
    } finally {
      saving = false;
    }
  }

  function resetForm() {
    showAddForm = false;
    editingId = null;
    title = '';
    description = '';
    startDate = new Date().toISOString().split('T')[0];
    endDate = '';
    status = 'active';
  }

  // Goal handlers
  function handleEditGoal(goal: TreatmentGoal) {
    editingGoalId = goal.id;
    goalDescription = goal.description;
    goalTargetDate = goal.target_date || '';
    goalStatus = goal.status;
    goalSortOrder = goal.sort_order;
    showAddGoalForm = true;
  }

  async function handleDeleteGoal(goalId: string) {
    if (!confirm(get(t)('treatmentPlans.confirmDeleteGoal'))) {
      return;
    }

    try {
      await deleteTreatmentGoal(goalId);
      if (selectedPlanId) {
        await loadPlanDetails(selectedPlanId);
      }
    } catch (err) {
      error =
        get(t)('common.deleteFailed') + ': ' + (err instanceof Error ? err.message : String(err));
      console.error('Failed to delete goal:', err);
    }
  }

  async function handleSubmitGoal(event: Event) {
    event.preventDefault();

    if (!goalDescription || !selectedPlanId) {
      error = get(t)('treatmentPlans.goalDescriptionRequired');
      return;
    }

    try {
      error = null;

      if (editingGoalId) {
        const update: UpdateTreatmentGoal = {
          description: goalDescription,
          target_date: goalTargetDate || undefined,
          status: goalStatus,
          sort_order: goalSortOrder,
        };
        await updateTreatmentGoal(editingGoalId, update);
      } else {
        const input: CreateTreatmentGoal = {
          treatment_plan_id: selectedPlanId,
          description: goalDescription,
          target_date: goalTargetDate || undefined,
          status: goalStatus,
          sort_order: goalSortOrder,
        };
        await createTreatmentGoal(input);
      }

      resetGoalForm();
      await loadPlanDetails(selectedPlanId);
    } catch (err) {
      error =
        get(t)('common.saveFailed') + ': ' + (err instanceof Error ? err.message : String(err));
      console.error('Failed to save goal:', err);
    }
  }

  function resetGoalForm() {
    showAddGoalForm = false;
    editingGoalId = null;
    goalDescription = '';
    goalTargetDate = '';
    goalStatus = 'in_progress';
    goalSortOrder = goals.length > 0 ? Math.max(...goals.map((g) => g.sort_order)) + 1 : 0;
  }

  // Intervention handlers
  function handleEditIntervention(intervention: TreatmentIntervention) {
    editingInterventionId = intervention.id;
    interventionType = intervention.type;
    interventionDescription = intervention.description;
    interventionFrequency = intervention.frequency || '';
    showAddInterventionForm = true;
  }

  async function handleDeleteIntervention(interventionId: string) {
    if (!confirm(get(t)('treatmentPlans.confirmDeleteIntervention'))) {
      return;
    }

    try {
      await deleteTreatmentIntervention(interventionId);
      if (selectedPlanId) {
        await loadPlanDetails(selectedPlanId);
      }
    } catch (err) {
      error =
        get(t)('common.deleteFailed') + ': ' + (err instanceof Error ? err.message : String(err));
      console.error('Failed to delete intervention:', err);
    }
  }

  async function handleSubmitIntervention(event: Event) {
    event.preventDefault();

    if (!interventionDescription || !selectedPlanId) {
      error = get(t)('treatmentPlans.interventionDescriptionRequired');
      return;
    }

    try {
      error = null;

      if (editingInterventionId) {
        const update: UpdateTreatmentIntervention = {
          type: interventionType,
          description: interventionDescription,
          frequency: interventionFrequency || undefined,
        };
        await updateTreatmentIntervention(editingInterventionId, update);
      } else {
        const input: CreateTreatmentIntervention = {
          treatment_plan_id: selectedPlanId,
          type: interventionType,
          description: interventionDescription,
          frequency: interventionFrequency || undefined,
        };
        await createTreatmentIntervention(input);
      }

      resetInterventionForm();
      await loadPlanDetails(selectedPlanId);
    } catch (err) {
      error =
        get(t)('common.saveFailed') + ': ' + (err instanceof Error ? err.message : String(err));
      console.error('Failed to save intervention:', err);
    }
  }

  function resetInterventionForm() {
    showAddInterventionForm = false;
    editingInterventionId = null;
    interventionType = 'psychotherapy';
    interventionDescription = '';
    interventionFrequency = '';
  }

  function getStatusColor(status: string): string {
    switch (status) {
      case 'active':
      case 'in_progress':
        return 'text-success-fg  border-success ';
      case 'completed':
      case 'achieved':
        return 'text-accent-fg  border-accent ';
      case 'revised':
        return 'text-warning-fg  border-warning ';
      case 'discontinued':
      case 'pending':
        return 'text-fg-muted  border-line ';
      default:
        return 'text-fg-muted  border-line ';
    }
  }
</script>

<div class="p-4 sm:p-6 lg:p-8 max-w-6xl mx-auto">
  <div class="flex justify-between items-center mb-6">
    <h1 class="text-display font-semibold text-fg">{$t('treatmentPlans.title')}</h1>
    <button
      class="h-8 px-3 bg-accent text-on-accent rounded-control hover:bg-accent-hover transition-colors"
      onclick={() => {
        if (showAddForm) {
          resetForm();
        } else {
          showAddForm = true;
        }
      }}
    >
      {showAddForm ? $t('common.cancel') : `+ ${$t('treatmentPlans.newPlan')}`}
    </button>
  </div>

  {#if error}
    <div class="bg-danger-subtle border border-danger-line text-danger-fg p-4 rounded-card mb-6">
      {error}
    </div>
  {/if}

  {#if showAddForm}
    <div class="bg-surface-raised border border-line rounded-card p-6 mb-6">
      <h2 class="text-heading font-semibold text-fg mb-4">
        {editingId ? $t('treatmentPlans.editPlan') : $t('treatmentPlans.newPlan')}
      </h2>
      <form onsubmit={handleSubmit} class="space-y-4">
        <div>
          <label for="title" class="block text-body font-medium text-fg-muted mb-1">
            {$t('treatmentPlans.planTitle')} *
          </label>
          <input
            id="title"
            type="text"
            bind:value={title}
            required
            class="w-full px-3 py-2 bg-surface-sunken border border-line rounded-control text-fg focus:outline-none focus:ring-2 focus:ring-accent/30"
          />
        </div>

        <div>
          <label for="description" class="block text-body font-medium text-fg-muted mb-1">
            {$t('treatmentPlans.description')}
          </label>
          <textarea
            id="description"
            bind:value={description}
            rows="3"
            class="w-full px-3 py-2 bg-surface-sunken border border-line rounded-control text-fg focus:outline-none focus:ring-2 focus:ring-accent/30"
          ></textarea>
        </div>

        <div class="grid grid-cols-1 gap-4 sm:grid-cols-2 lg:grid-cols-3">
          <div>
            <label for="start-date" class="block text-body font-medium text-fg-muted mb-1">
              {$t('treatmentPlans.startDate')} *
            </label>
            <input
              id="start-date"
              type="date"
              bind:value={startDate}
              required
              class="w-full px-3 py-2 bg-surface-sunken border border-line rounded-control text-fg focus:outline-none focus:ring-2 focus:ring-accent/30"
            />
          </div>

          <div>
            <label for="end-date" class="block text-body font-medium text-fg-muted mb-1">
              {$t('treatmentPlans.endDate')}
            </label>
            <input
              id="end-date"
              type="date"
              bind:value={endDate}
              class="w-full px-3 py-2 bg-surface-sunken border border-line rounded-control text-fg focus:outline-none focus:ring-2 focus:ring-accent/30"
            />
          </div>

          <div>
            <label for="status" class="block text-body font-medium text-fg-muted mb-1">
              {$t('treatmentPlans.statusLabel')} *
            </label>
            <select
              id="status"
              bind:value={status}
              required
              class="w-full px-3 py-2 bg-surface-sunken border border-line rounded-control text-fg focus:outline-none focus:ring-2 focus:ring-accent/30"
            >
              {#each statusOptions as option}
                <option value={option}>{$planStatusLabel(option)}</option>
              {/each}
            </select>
          </div>
        </div>

        <div class="flex justify-end gap-2 pt-2">
          <button
            type="button"
            onclick={resetForm}
            class="h-8 px-3 border border-line text-fg-muted rounded-control hover:bg-surface-hover transition-colors"
          >
            {$t('common.cancel')}
          </button>
          <button
            type="submit"
            disabled={saving}
            class="h-8 px-3 bg-accent text-on-accent rounded-control hover:bg-accent-hover transition-colors disabled:opacity-50"
          >
            {saving ? $t('common.saving') : editingId ? $t('common.update') : $t('common.create')}
          </button>
        </div>
      </form>
    </div>
  {/if}

  {#if loading}
    <div class="flex justify-center py-12">
      <div class="text-fg-muted">{$t('treatmentPlans.loading')}</div>
    </div>
  {:else if plans.length === 0}
    <div class="text-center py-12">
      <p class="text-fg-muted mb-4">{$t('treatmentPlans.noPlans')}</p>
      <button
        class="h-8 px-3 bg-accent text-on-accent rounded-control hover:bg-accent-hover transition-colors"
        onclick={() => (showAddForm = true)}
      >
        {$t('treatmentPlans.addFirst')}
      </button>
    </div>
  {:else}
    <div class="grid gap-4">
      {#each plans as plan (plan.id)}
        <div class="bg-surface-raised rounded-card border border-line">
          <div class="p-4">
            <div class="flex justify-between items-start mb-2">
              <div class="flex-1">
                <div class="flex items-center gap-2 mb-1">
                  <h3 class="text-heading font-semibold text-fg">{plan.title}</h3>
                  <span
                    class="px-2 py-0.5 rounded-full text-caption border {getStatusColor(
                      plan.status
                    )}"
                  >
                    {$planStatusLabel(plan.status)}
                  </span>
                </div>
                {#if plan.description}
                  <p class="text-body text-fg-muted mb-2">{plan.description}</p>
                {/if}
                <div class="text-body text-fg-muted">
                  <span>{plan.start_date}</span>
                  {#if plan.end_date}
                    <span> — {plan.end_date}</span>
                  {/if}
                </div>
              </div>
              <div class="flex gap-2 ml-2">
                <button
                  onclick={() => handleEdit(plan)}
                  class="p-2 text-fg-muted hover:text-accent-fg transition-colors"
                  title={$t('common.edit')}
                >
                  <svg
                    xmlns="http://www.w3.org/2000/svg"
                    class="w-5 h-5"
                    fill="none"
                    viewBox="0 0 24 24"
                    stroke="currentColor"
                  >
                    <path
                      stroke-linecap="round"
                      stroke-linejoin="round"
                      stroke-width="2"
                      d="M11 5H6a2 2 0 00-2 2v11a2 2 0 002 2h11a2 2 0 002-2v-5m-1.414-9.414a2 2 0 112.828 2.828L11.828 15H9v-2.828l8.586-8.586z"
                    />
                  </svg>
                </button>
                <button
                  onclick={() => handleDelete(plan.id)}
                  class="p-2 text-fg-muted hover:text-danger-fg transition-colors"
                  title={$t('common.delete')}
                >
                  <svg
                    xmlns="http://www.w3.org/2000/svg"
                    class="w-5 h-5"
                    fill="none"
                    viewBox="0 0 24 24"
                    stroke="currentColor"
                  >
                    <path
                      stroke-linecap="round"
                      stroke-linejoin="round"
                      stroke-width="2"
                      d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16"
                    />
                  </svg>
                </button>
                <button
                  onclick={() =>
                    selectedPlanId === plan.id ? (selectedPlanId = null) : loadPlanDetails(plan.id)}
                  class="p-2 text-fg-muted hover:text-accent-fg transition-colors"
                  title={selectedPlanId === plan.id
                    ? $t('treatmentPlans.collapse')
                    : $t('treatmentPlans.viewDetails')}
                >
                  <svg
                    xmlns="http://www.w3.org/2000/svg"
                    class="w-5 h-5"
                    fill="none"
                    viewBox="0 0 24 24"
                    stroke="currentColor"
                  >
                    <path
                      stroke-linecap="round"
                      stroke-linejoin="round"
                      stroke-width="2"
                      d={selectedPlanId === plan.id ? 'M5 15l7-7 7 7' : 'M19 9l-7 7-7-7'}
                    />
                  </svg>
                </button>
              </div>
            </div>
          </div>

          {#if selectedPlanId === plan.id}
            <div class="border-t border-line p-4 bg-surface-sunken">
              {#if loadingDetails}
                <div class="text-center py-4 text-fg-muted">
                  {$t('treatmentPlans.loadingDetails')}
                </div>
              {:else}
                <!-- Goals Section -->
                <div class="mb-6">
                  <div class="flex justify-between items-center mb-3">
                    <h4 class="text-md font-semibold text-fg">{$t('treatmentPlans.goals')}</h4>
                    <button
                      onclick={() => {
                        if (showAddGoalForm) {
                          resetGoalForm();
                        } else {
                          resetGoalForm();
                          showAddGoalForm = true;
                        }
                      }}
                      class="text-body h-7 px-2.5 bg-accent text-on-accent rounded-control hover:bg-accent-hover transition-colors"
                    >
                      {showAddGoalForm ? $t('common.cancel') : `+ ${$t('treatmentPlans.addGoal')}`}
                    </button>
                  </div>

                  {#if showAddGoalForm}
                    <form
                      onsubmit={handleSubmitGoal}
                      class="bg-surface-raised p-4 rounded-card border border-line mb-3 space-y-3"
                    >
                      <div>
                        <label
                          for="goal-description"
                          class="block text-body font-medium text-fg-muted mb-1"
                        >
                          {$t('treatmentPlans.description')} *
                        </label>
                        <textarea
                          id="goal-description"
                          bind:value={goalDescription}
                          required
                          rows="2"
                          class="w-full px-3 py-2 bg-surface-sunken border border-line rounded-control text-fg focus:outline-none focus:ring-2 focus:ring-accent/30"
                        ></textarea>
                      </div>
                      <div class="grid grid-cols-1 gap-3 sm:grid-cols-3">
                        <div>
                          <label
                            for="goal-target-date"
                            class="block text-body font-medium text-fg-muted mb-1"
                          >
                            {$t('treatmentPlans.targetDate')}
                          </label>
                          <input
                            id="goal-target-date"
                            type="date"
                            bind:value={goalTargetDate}
                            class="w-full px-3 py-2 bg-surface-sunken border border-line rounded-control text-fg focus:outline-none focus:ring-2 focus:ring-accent/30"
                          />
                        </div>
                        <div>
                          <label
                            for="goal-status"
                            class="block text-body font-medium text-fg-muted mb-1"
                          >
                            {$t('treatmentPlans.statusLabel')}
                          </label>
                          <select
                            id="goal-status"
                            bind:value={goalStatus}
                            class="w-full px-3 py-2 bg-surface-sunken border border-line rounded-control text-fg focus:outline-none focus:ring-2 focus:ring-accent/30"
                          >
                            {#each goalStatusOptions as option}
                              <option value={option}>{$goalStatusLabel(option)}</option>
                            {/each}
                          </select>
                        </div>
                        <div>
                          <label
                            for="goal-priority"
                            class="block text-body font-medium text-fg-muted mb-1"
                          >
                            {$t('treatmentPlans.priority')}
                          </label>
                          <input
                            id="goal-priority"
                            type="number"
                            bind:value={goalSortOrder}
                            min="0"
                            class="w-full px-3 py-2 bg-surface-sunken border border-line rounded-control text-fg focus:outline-none focus:ring-2 focus:ring-accent/30"
                          />
                        </div>
                      </div>
                      <div class="flex justify-end gap-2">
                        <button
                          type="button"
                          onclick={resetGoalForm}
                          class="h-7 px-2.5 border border-line text-fg-muted rounded-control hover:bg-surface-hover transition-colors"
                        >
                          {$t('common.cancel')}
                        </button>
                        <button
                          type="submit"
                          class="h-7 px-2.5 bg-accent text-on-accent rounded-control hover:bg-accent-hover transition-colors"
                        >
                          {editingGoalId ? $t('common.update') : $t('common.add')}
                        </button>
                      </div>
                    </form>
                  {/if}

                  {#if goals.length === 0}
                    <p class="text-body text-fg-muted">{$t('treatmentPlans.noGoals')}</p>
                  {:else}
                    <div class="space-y-2">
                      {#each goals as goal (goal.id)}
                        <div class="bg-surface-raised p-3 rounded-card border border-line">
                          <div class="flex justify-between items-start">
                            <div class="flex-1">
                              <div class="flex items-center gap-2 mb-1">
                                <span
                                  class="px-2 py-0.5 rounded-full text-caption border {getStatusColor(
                                    goal.status
                                  )}"
                                >
                                  {$goalStatusLabel(goal.status)}
                                </span>
                              </div>
                              <p class="text-body text-fg">{goal.description}</p>
                              {#if goal.target_date}
                                <p class="text-caption text-fg-muted mt-1">
                                  {$t('treatmentPlans.targetDateValue').replace(
                                    '{date}',
                                    goal.target_date
                                  )}
                                </p>
                              {/if}
                            </div>
                            <div class="flex gap-1 ml-2">
                              <button
                                onclick={() => handleEditGoal(goal)}
                                class="p-1 text-fg-muted hover:text-accent-fg transition-colors"
                                aria-label={$t('treatmentPlans.editGoal')}
                              >
                                <svg
                                  xmlns="http://www.w3.org/2000/svg"
                                  class="w-4 h-4"
                                  fill="none"
                                  viewBox="0 0 24 24"
                                  stroke="currentColor"
                                >
                                  <path
                                    stroke-linecap="round"
                                    stroke-linejoin="round"
                                    stroke-width="2"
                                    d="M11 5H6a2 2 0 00-2 2v11a2 2 0 002 2h11a2 2 0 002-2v-5m-1.414-9.414a2 2 0 112.828 2.828L11.828 15H9v-2.828l8.586-8.586z"
                                  />
                                </svg>
                              </button>
                              <button
                                onclick={() => handleDeleteGoal(goal.id)}
                                class="p-1 text-fg-muted hover:text-danger-fg transition-colors"
                                aria-label={$t('treatmentPlans.deleteGoal')}
                              >
                                <svg
                                  xmlns="http://www.w3.org/2000/svg"
                                  class="w-4 h-4"
                                  fill="none"
                                  viewBox="0 0 24 24"
                                  stroke="currentColor"
                                >
                                  <path
                                    stroke-linecap="round"
                                    stroke-linejoin="round"
                                    stroke-width="2"
                                    d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16"
                                  />
                                </svg>
                              </button>
                            </div>
                          </div>
                        </div>
                      {/each}
                    </div>
                  {/if}
                </div>

                <!-- Interventions Section -->
                <div>
                  <div class="flex justify-between items-center mb-3">
                    <h4 class="text-md font-semibold text-fg">
                      {$t('treatmentPlans.interventions')}
                    </h4>
                    <button
                      onclick={() => {
                        if (showAddInterventionForm) {
                          resetInterventionForm();
                        } else {
                          resetInterventionForm();
                          showAddInterventionForm = true;
                        }
                      }}
                      class="text-body h-7 px-2.5 bg-accent text-on-accent rounded-control hover:bg-accent-hover transition-colors"
                    >
                      {showAddInterventionForm
                        ? $t('common.cancel')
                        : `+ ${$t('treatmentPlans.addIntervention')}`}
                    </button>
                  </div>

                  {#if showAddInterventionForm}
                    <form
                      onsubmit={handleSubmitIntervention}
                      class="bg-surface-raised p-4 rounded-card border border-line mb-3 space-y-3"
                    >
                      <div>
                        <label
                          for="intervention-type"
                          class="block text-body font-medium text-fg-muted mb-1"
                        >
                          {$t('treatmentPlans.interventionTypeLabel')} *
                        </label>
                        <select
                          id="intervention-type"
                          bind:value={interventionType}
                          required
                          class="w-full px-3 py-2 bg-surface-sunken border border-line rounded-control text-fg focus:outline-none focus:ring-2 focus:ring-accent/30"
                        >
                          {#each interventionTypeOptions as option}
                            <option value={option}>{$interventionTypeLabel(option)}</option>
                          {/each}
                        </select>
                      </div>
                      <div>
                        <label
                          for="intervention-description"
                          class="block text-body font-medium text-fg-muted mb-1"
                        >
                          {$t('treatmentPlans.description')} *
                        </label>
                        <textarea
                          id="intervention-description"
                          bind:value={interventionDescription}
                          required
                          rows="2"
                          class="w-full px-3 py-2 bg-surface-sunken border border-line rounded-control text-fg focus:outline-none focus:ring-2 focus:ring-accent/30"
                        ></textarea>
                      </div>
                      <div>
                        <label
                          for="intervention-frequency"
                          class="block text-body font-medium text-fg-muted mb-1"
                        >
                          {$t('treatmentPlans.frequency')}
                        </label>
                        <input
                          id="intervention-frequency"
                          type="text"
                          bind:value={interventionFrequency}
                          placeholder={$t('treatmentPlans.frequencyPlaceholder')}
                          class="w-full px-3 py-2 bg-surface-sunken border border-line rounded-control text-fg focus:outline-none focus:ring-2 focus:ring-accent/30"
                        />
                      </div>
                      <div class="flex justify-end gap-2">
                        <button
                          type="button"
                          onclick={resetInterventionForm}
                          class="h-7 px-2.5 border border-line text-fg-muted rounded-control hover:bg-surface-hover transition-colors"
                        >
                          {$t('common.cancel')}
                        </button>
                        <button
                          type="submit"
                          class="h-7 px-2.5 bg-accent text-on-accent rounded-control hover:bg-accent-hover transition-colors"
                        >
                          {editingInterventionId ? $t('common.update') : $t('common.add')}
                        </button>
                      </div>
                    </form>
                  {/if}

                  {#if interventions.length === 0}
                    <p class="text-body text-fg-muted">{$t('treatmentPlans.noInterventions')}</p>
                  {:else}
                    <div class="space-y-2">
                      {#each interventions as intervention (intervention.id)}
                        <div class="bg-surface-raised p-3 rounded-card border border-line">
                          <div class="flex justify-between items-start">
                            <div class="flex-1">
                              <div class="flex items-center gap-2 mb-1">
                                <span
                                  class="px-2 py-0.5 rounded-full text-caption bg-accent-subtle text-accent-fg"
                                >
                                  {$interventionTypeLabel(intervention.type)}
                                </span>
                              </div>
                              <p class="text-body text-fg">{intervention.description}</p>
                              {#if intervention.frequency}
                                <p class="text-caption text-fg-muted mt-1">
                                  {$t('treatmentPlans.frequencyValue').replace(
                                    '{value}',
                                    intervention.frequency
                                  )}
                                </p>
                              {/if}
                            </div>
                            <div class="flex gap-1 ml-2">
                              <button
                                onclick={() => handleEditIntervention(intervention)}
                                class="p-1 text-fg-muted hover:text-accent-fg transition-colors"
                                aria-label={$t('treatmentPlans.editIntervention')}
                              >
                                <svg
                                  xmlns="http://www.w3.org/2000/svg"
                                  class="w-4 h-4"
                                  fill="none"
                                  viewBox="0 0 24 24"
                                  stroke="currentColor"
                                >
                                  <path
                                    stroke-linecap="round"
                                    stroke-linejoin="round"
                                    stroke-width="2"
                                    d="M11 5H6a2 2 0 00-2 2v11a2 2 0 002 2h11a2 2 0 002-2v-5m-1.414-9.414a2 2 0 112.828 2.828L11.828 15H9v-2.828l8.586-8.586z"
                                  />
                                </svg>
                              </button>
                              <button
                                onclick={() => handleDeleteIntervention(intervention.id)}
                                class="p-1 text-fg-muted hover:text-danger-fg transition-colors"
                                aria-label={$t('treatmentPlans.deleteIntervention')}
                              >
                                <svg
                                  xmlns="http://www.w3.org/2000/svg"
                                  class="w-4 h-4"
                                  fill="none"
                                  viewBox="0 0 24 24"
                                  stroke="currentColor"
                                >
                                  <path
                                    stroke-linecap="round"
                                    stroke-linejoin="round"
                                    stroke-width="2"
                                    d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16"
                                  />
                                </svg>
                              </button>
                            </div>
                          </div>
                        </div>
                      {/each}
                    </div>
                  {/if}
                </div>
              {/if}
            </div>
          {/if}
        </div>
      {/each}
    </div>
  {/if}
</div>
