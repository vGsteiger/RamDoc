<script lang="ts">
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
    type UpdateTreatmentIntervention
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

  const statusOptions = [
    { value: 'active', label: 'Active' },
    { value: 'completed', label: 'Completed' },
    { value: 'revised', label: 'Revised' },
    { value: 'discontinued', label: 'Discontinued' }
  ];

  const goalStatusOptions = [
    { value: 'pending', label: 'Pending' },
    { value: 'in_progress', label: 'In Progress' },
    { value: 'achieved', label: 'Achieved' },
    { value: 'revised', label: 'Revised' },
    { value: 'discontinued', label: 'Discontinued' }
  ];

  const interventionTypeOptions = [
    { value: 'psychotherapy', label: 'Psychotherapy' },
    { value: 'medication', label: 'Medication' },
    { value: 'referral', label: 'Referral' },
    { value: 'other', label: 'Other' }
  ];

  onMount(async () => {
    await loadPlans();
  });

  async function loadPlans() {
    try {
      loading = true;
      error = null;
      plans = await listTreatmentPlansForPatient(patientId);
    } catch (err) {
      error = 'Error loading treatment plans: ' + (err instanceof Error ? err.message : String(err));
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
        listTreatmentInterventionsForPlan(planId)
      ]);
      goals = loadedGoals;
      interventions = loadedInterventions;
    } catch (err) {
      error = 'Error loading plan details: ' + (err instanceof Error ? err.message : String(err));
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
    if (!confirm('Are you sure you want to delete this treatment plan? This will also delete all goals and interventions.')) {
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
      error = 'Error deleting plan: ' + (err instanceof Error ? err.message : String(err));
      console.error('Failed to delete plan:', err);
    }
  }

  async function handleSubmit(event: Event) {
    event.preventDefault();

    if (!title) {
      error = 'Please enter a title.';
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
          status
        };
        await updateTreatmentPlan(editingId, update);
      } else {
        const input: CreateTreatmentPlan = {
          patient_id: patientId,
          title,
          description: description || undefined,
          start_date: startDate,
          end_date: endDate || undefined,
          status
        };
        await createTreatmentPlan(input);
      }

      resetForm();
      await loadPlans();
    } catch (err) {
      error = 'Error saving plan: ' + (err instanceof Error ? err.message : String(err));
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
    if (!confirm('Are you sure you want to delete this goal?')) {
      return;
    }

    try {
      await deleteTreatmentGoal(goalId);
      if (selectedPlanId) {
        await loadPlanDetails(selectedPlanId);
      }
    } catch (err) {
      error = 'Error deleting goal: ' + (err instanceof Error ? err.message : String(err));
      console.error('Failed to delete goal:', err);
    }
  }

  async function handleSubmitGoal(event: Event) {
    event.preventDefault();

    if (!goalDescription || !selectedPlanId) {
      error = 'Please enter a goal description.';
      return;
    }

    try {
      error = null;

      if (editingGoalId) {
        const update: UpdateTreatmentGoal = {
          description: goalDescription,
          target_date: goalTargetDate || undefined,
          status: goalStatus,
          sort_order: goalSortOrder
        };
        await updateTreatmentGoal(editingGoalId, update);
      } else {
        const input: CreateTreatmentGoal = {
          treatment_plan_id: selectedPlanId,
          description: goalDescription,
          target_date: goalTargetDate || undefined,
          status: goalStatus,
          sort_order: goalSortOrder
        };
        await createTreatmentGoal(input);
      }

      resetGoalForm();
      await loadPlanDetails(selectedPlanId);
    } catch (err) {
      error = 'Error saving goal: ' + (err instanceof Error ? err.message : String(err));
      console.error('Failed to save goal:', err);
    }
  }

  function resetGoalForm() {
    showAddGoalForm = false;
    editingGoalId = null;
    goalDescription = '';
    goalTargetDate = '';
    goalStatus = 'in_progress';
    goalSortOrder = goals.length > 0 ? Math.max(...goals.map(g => g.sort_order)) + 1 : 0;
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
    if (!confirm('Are you sure you want to delete this intervention?')) {
      return;
    }

    try {
      await deleteTreatmentIntervention(interventionId);
      if (selectedPlanId) {
        await loadPlanDetails(selectedPlanId);
      }
    } catch (err) {
      error = 'Error deleting intervention: ' + (err instanceof Error ? err.message : String(err));
      console.error('Failed to delete intervention:', err);
    }
  }

  async function handleSubmitIntervention(event: Event) {
    event.preventDefault();

    if (!interventionDescription || !selectedPlanId) {
      error = 'Please enter an intervention description.';
      return;
    }

    try {
      error = null;

      if (editingInterventionId) {
        const update: UpdateTreatmentIntervention = {
          type: interventionType,
          description: interventionDescription,
          frequency: interventionFrequency || undefined
        };
        await updateTreatmentIntervention(editingInterventionId, update);
      } else {
        const input: CreateTreatmentIntervention = {
          treatment_plan_id: selectedPlanId,
          type: interventionType,
          description: interventionDescription,
          frequency: interventionFrequency || undefined
        };
        await createTreatmentIntervention(input);
      }

      resetInterventionForm();
      await loadPlanDetails(selectedPlanId);
    } catch (err) {
      error = 'Error saving intervention: ' + (err instanceof Error ? err.message : String(err));
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
        return 'text-green-600 dark:text-green-400 border-green-600 dark:border-green-400';
      case 'completed':
      case 'achieved':
        return 'text-blue-600 dark:text-blue-400 border-blue-600 dark:border-blue-400';
      case 'revised':
        return 'text-yellow-600 dark:text-yellow-400 border-yellow-600 dark:border-yellow-400';
      case 'discontinued':
      case 'pending':
        return 'text-gray-600 dark:text-gray-400 border-gray-600 dark:border-gray-400';
      default:
        return 'text-gray-600 dark:text-gray-400 border-gray-600 dark:border-gray-400';
    }
  }
</script>

<div class="p-8 max-w-6xl mx-auto">
  <div class="flex justify-between items-center mb-6">
    <h1 class="text-2xl font-bold text-gray-900 dark:text-gray-100">Treatment Plans</h1>
    <button
      class="px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 transition-colors"
      onclick={() => {
        if (showAddForm) {
          resetForm();
        } else {
          showAddForm = true;
        }
      }}
    >
      {showAddForm ? 'Cancel' : '+ New Treatment Plan'}
    </button>
  </div>

  {#if error}
    <div class="bg-red-500/10 border border-red-500/30 text-red-400 p-4 rounded-lg mb-6">
      {error}
    </div>
  {/if}

  {#if showAddForm}
    <div class="bg-white dark:bg-gray-800 border border-gray-200 dark:border-gray-700 rounded-lg p-6 mb-6">
      <h2 class="text-lg font-semibold text-gray-900 dark:text-gray-100 mb-4">
        {editingId ? 'Edit Treatment Plan' : 'Add New Treatment Plan'}
      </h2>
      <form onsubmit={handleSubmit} class="space-y-4">
        <div>
          <label for="title" class="block text-sm font-medium text-gray-600 dark:text-gray-300 mb-1">
            Title *
          </label>
          <input
            id="title"
            type="text"
            bind:value={title}
            required
            class="w-full px-3 py-2 bg-gray-50 dark:bg-gray-700 border border-gray-300 dark:border-gray-600 rounded-lg text-gray-900 dark:text-gray-100 focus:outline-none focus:ring-2 focus:ring-blue-500"
          />
        </div>

        <div>
          <label for="description" class="block text-sm font-medium text-gray-600 dark:text-gray-300 mb-1">
            Description
          </label>
          <textarea
            id="description"
            bind:value={description}
            rows="3"
            class="w-full px-3 py-2 bg-gray-50 dark:bg-gray-700 border border-gray-300 dark:border-gray-600 rounded-lg text-gray-900 dark:text-gray-100 focus:outline-none focus:ring-2 focus:ring-blue-500"
          ></textarea>
        </div>

        <div class="grid grid-cols-3 gap-4">
          <div>
            <label for="start-date" class="block text-sm font-medium text-gray-600 dark:text-gray-300 mb-1">
              Start Date *
            </label>
            <input
              id="start-date"
              type="date"
              bind:value={startDate}
              required
              class="w-full px-3 py-2 bg-gray-50 dark:bg-gray-700 border border-gray-300 dark:border-gray-600 rounded-lg text-gray-900 dark:text-gray-100 focus:outline-none focus:ring-2 focus:ring-blue-500"
            />
          </div>

          <div>
            <label for="end-date" class="block text-sm font-medium text-gray-600 dark:text-gray-300 mb-1">
              End Date
            </label>
            <input
              id="end-date"
              type="date"
              bind:value={endDate}
              class="w-full px-3 py-2 bg-gray-50 dark:bg-gray-700 border border-gray-300 dark:border-gray-600 rounded-lg text-gray-900 dark:text-gray-100 focus:outline-none focus:ring-2 focus:ring-blue-500"
            />
          </div>

          <div>
            <label for="status" class="block text-sm font-medium text-gray-600 dark:text-gray-300 mb-1">
              Status *
            </label>
            <select
              id="status"
              bind:value={status}
              required
              class="w-full px-3 py-2 bg-gray-50 dark:bg-gray-700 border border-gray-300 dark:border-gray-600 rounded-lg text-gray-900 dark:text-gray-100 focus:outline-none focus:ring-2 focus:ring-blue-500"
            >
              {#each statusOptions as option}
                <option value={option.value}>{option.label}</option>
              {/each}
            </select>
          </div>
        </div>

        <div class="flex justify-end gap-2 pt-2">
          <button
            type="button"
            onclick={resetForm}
            class="px-4 py-2 border border-gray-300 dark:border-gray-600 text-gray-700 dark:text-gray-300 rounded-lg hover:bg-gray-50 dark:hover:bg-gray-700 transition-colors"
          >
            Cancel
          </button>
          <button
            type="submit"
            disabled={saving}
            class="px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 transition-colors disabled:opacity-50"
          >
            {saving ? 'Saving...' : editingId ? 'Update' : 'Create'}
          </button>
        </div>
      </form>
    </div>
  {/if}

  {#if loading}
    <div class="flex justify-center py-12">
      <div class="text-gray-500 dark:text-gray-400">Loading treatment plans...</div>
    </div>
  {:else if plans.length === 0}
    <div class="text-center py-12">
      <p class="text-gray-500 dark:text-gray-400 mb-4">No treatment plans yet</p>
      <button
        class="px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 transition-colors"
        onclick={() => showAddForm = true}
      >
        Add First Treatment Plan
      </button>
    </div>
  {:else}
    <div class="grid gap-4">
      {#each plans as plan (plan.id)}
        <div class="bg-white dark:bg-gray-800 rounded-lg border border-gray-200 dark:border-gray-700">
          <div class="p-4">
            <div class="flex justify-between items-start mb-2">
              <div class="flex-1">
                <div class="flex items-center gap-2 mb-1">
                  <h3 class="text-lg font-semibold text-gray-900 dark:text-gray-100">{plan.title}</h3>
                  <span class="px-2 py-0.5 rounded-full text-xs border {getStatusColor(plan.status)}">
                    {plan.status}
                  </span>
                </div>
                {#if plan.description}
                  <p class="text-sm text-gray-600 dark:text-gray-300 mb-2">{plan.description}</p>
                {/if}
                <div class="text-sm text-gray-500 dark:text-gray-400">
                  <span>{plan.start_date}</span>
                  {#if plan.end_date}
                    <span> — {plan.end_date}</span>
                  {/if}
                </div>
              </div>
              <div class="flex gap-2 ml-2">
                <button
                  onclick={() => handleEdit(plan)}
                  class="p-2 text-gray-400 hover:text-blue-500 transition-colors"
                  title="Edit"
                >
                  <svg xmlns="http://www.w3.org/2000/svg" class="w-5 h-5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M11 5H6a2 2 0 00-2 2v11a2 2 0 002 2h11a2 2 0 002-2v-5m-1.414-9.414a2 2 0 112.828 2.828L11.828 15H9v-2.828l8.586-8.586z" />
                  </svg>
                </button>
                <button
                  onclick={() => handleDelete(plan.id)}
                  class="p-2 text-gray-400 hover:text-red-500 transition-colors"
                  title="Delete"
                >
                  <svg xmlns="http://www.w3.org/2000/svg" class="w-5 h-5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16" />
                  </svg>
                </button>
                <button
                  onclick={() => selectedPlanId === plan.id ? (selectedPlanId = null) : loadPlanDetails(plan.id)}
                  class="p-2 text-gray-400 hover:text-blue-500 transition-colors"
                  title={selectedPlanId === plan.id ? 'Collapse' : 'View Details'}
                >
                  <svg xmlns="http://www.w3.org/2000/svg" class="w-5 h-5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d={selectedPlanId === plan.id ? "M5 15l7-7 7 7" : "M19 9l-7 7-7-7"} />
                  </svg>
                </button>
              </div>
            </div>
          </div>

          {#if selectedPlanId === plan.id}
            <div class="border-t border-gray-200 dark:border-gray-700 p-4 bg-gray-50 dark:bg-gray-900/50">
              {#if loadingDetails}
                <div class="text-center py-4 text-gray-500 dark:text-gray-400">Loading details...</div>
              {:else}
                <!-- Goals Section -->
                <div class="mb-6">
                  <div class="flex justify-between items-center mb-3">
                    <h4 class="text-md font-semibold text-gray-900 dark:text-gray-100">Goals</h4>
                    <button
                      onclick={() => {
                        if (showAddGoalForm) {
                          resetGoalForm();
                        } else {
                          resetGoalForm();
                          showAddGoalForm = true;
                        }
                      }}
                      class="text-sm px-3 py-1 bg-blue-600 text-white rounded hover:bg-blue-700 transition-colors"
                    >
                      {showAddGoalForm ? 'Cancel' : '+ Add Goal'}
                    </button>
                  </div>

                  {#if showAddGoalForm}
                    <form onsubmit={handleSubmitGoal} class="bg-white dark:bg-gray-800 p-4 rounded border border-gray-200 dark:border-gray-700 mb-3 space-y-3">
                      <div>
                        <label class="block text-sm font-medium text-gray-600 dark:text-gray-300 mb-1">
                          Description *
                        </label>
                        <textarea
                          bind:value={goalDescription}
                          required
                          rows="2"
                          class="w-full px-3 py-2 bg-gray-50 dark:bg-gray-700 border border-gray-300 dark:border-gray-600 rounded text-gray-900 dark:text-gray-100 focus:outline-none focus:ring-2 focus:ring-blue-500"
                        ></textarea>
                      </div>
                      <div class="grid grid-cols-3 gap-3">
                        <div>
                          <label class="block text-sm font-medium text-gray-600 dark:text-gray-300 mb-1">
                            Target Date
                          </label>
                          <input
                            type="date"
                            bind:value={goalTargetDate}
                            class="w-full px-3 py-2 bg-gray-50 dark:bg-gray-700 border border-gray-300 dark:border-gray-600 rounded text-gray-900 dark:text-gray-100 focus:outline-none focus:ring-2 focus:ring-blue-500"
                          />
                        </div>
                        <div>
                          <label class="block text-sm font-medium text-gray-600 dark:text-gray-300 mb-1">
                            Status
                          </label>
                          <select
                            bind:value={goalStatus}
                            class="w-full px-3 py-2 bg-gray-50 dark:bg-gray-700 border border-gray-300 dark:border-gray-600 rounded text-gray-900 dark:text-gray-100 focus:outline-none focus:ring-2 focus:ring-blue-500"
                          >
                            {#each goalStatusOptions as option}
                              <option value={option.value}>{option.label}</option>
                            {/each}
                          </select>
                        </div>
                        <div>
                          <label class="block text-sm font-medium text-gray-600 dark:text-gray-300 mb-1">
                            Priority
                          </label>
                          <input
                            type="number"
                            bind:value={goalSortOrder}
                            min="0"
                            class="w-full px-3 py-2 bg-gray-50 dark:bg-gray-700 border border-gray-300 dark:border-gray-600 rounded text-gray-900 dark:text-gray-100 focus:outline-none focus:ring-2 focus:ring-blue-500"
                          />
                        </div>
                      </div>
                      <div class="flex justify-end gap-2">
                        <button
                          type="button"
                          onclick={resetGoalForm}
                          class="px-3 py-1 border border-gray-300 dark:border-gray-600 text-gray-700 dark:text-gray-300 rounded hover:bg-gray-50 dark:hover:bg-gray-700 transition-colors"
                        >
                          Cancel
                        </button>
                        <button
                          type="submit"
                          class="px-3 py-1 bg-blue-600 text-white rounded hover:bg-blue-700 transition-colors"
                        >
                          {editingGoalId ? 'Update' : 'Add'}
                        </button>
                      </div>
                    </form>
                  {/if}

                  {#if goals.length === 0}
                    <p class="text-sm text-gray-500 dark:text-gray-400">No goals yet</p>
                  {:else}
                    <div class="space-y-2">
                      {#each goals as goal (goal.id)}
                        <div class="bg-white dark:bg-gray-800 p-3 rounded border border-gray-200 dark:border-gray-700">
                          <div class="flex justify-between items-start">
                            <div class="flex-1">
                              <div class="flex items-center gap-2 mb-1">
                                <span class="px-2 py-0.5 rounded-full text-xs border {getStatusColor(goal.status)}">
                                  {goal.status}
                                </span>
                              </div>
                              <p class="text-sm text-gray-900 dark:text-gray-100">{goal.description}</p>
                              {#if goal.target_date}
                                <p class="text-xs text-gray-500 dark:text-gray-400 mt-1">Target: {goal.target_date}</p>
                              {/if}
                            </div>
                            <div class="flex gap-1 ml-2">
                              <button
                                onclick={() => handleEditGoal(goal)}
                                class="p-1 text-gray-400 hover:text-blue-500 transition-colors"
                              >
                                <svg xmlns="http://www.w3.org/2000/svg" class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M11 5H6a2 2 0 00-2 2v11a2 2 0 002 2h11a2 2 0 002-2v-5m-1.414-9.414a2 2 0 112.828 2.828L11.828 15H9v-2.828l8.586-8.586z" />
                                </svg>
                              </button>
                              <button
                                onclick={() => handleDeleteGoal(goal.id)}
                                class="p-1 text-gray-400 hover:text-red-500 transition-colors"
                              >
                                <svg xmlns="http://www.w3.org/2000/svg" class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16" />
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
                    <h4 class="text-md font-semibold text-gray-900 dark:text-gray-100">Interventions</h4>
                    <button
                      onclick={() => {
                        if (showAddInterventionForm) {
                          resetInterventionForm();
                        } else {
                          resetInterventionForm();
                          showAddInterventionForm = true;
                        }
                      }}
                      class="text-sm px-3 py-1 bg-blue-600 text-white rounded hover:bg-blue-700 transition-colors"
                    >
                      {showAddInterventionForm ? 'Cancel' : '+ Add Intervention'}
                    </button>
                  </div>

                  {#if showAddInterventionForm}
                    <form onsubmit={handleSubmitIntervention} class="bg-white dark:bg-gray-800 p-4 rounded border border-gray-200 dark:border-gray-700 mb-3 space-y-3">
                      <div>
                        <label class="block text-sm font-medium text-gray-600 dark:text-gray-300 mb-1">
                          Type *
                        </label>
                        <select
                          bind:value={interventionType}
                          required
                          class="w-full px-3 py-2 bg-gray-50 dark:bg-gray-700 border border-gray-300 dark:border-gray-600 rounded text-gray-900 dark:text-gray-100 focus:outline-none focus:ring-2 focus:ring-blue-500"
                        >
                          {#each interventionTypeOptions as option}
                            <option value={option.value}>{option.label}</option>
                          {/each}
                        </select>
                      </div>
                      <div>
                        <label class="block text-sm font-medium text-gray-600 dark:text-gray-300 mb-1">
                          Description *
                        </label>
                        <textarea
                          bind:value={interventionDescription}
                          required
                          rows="2"
                          class="w-full px-3 py-2 bg-gray-50 dark:bg-gray-700 border border-gray-300 dark:border-gray-600 rounded text-gray-900 dark:text-gray-100 focus:outline-none focus:ring-2 focus:ring-blue-500"
                        ></textarea>
                      </div>
                      <div>
                        <label class="block text-sm font-medium text-gray-600 dark:text-gray-300 mb-1">
                          Frequency
                        </label>
                        <input
                          type="text"
                          bind:value={interventionFrequency}
                          placeholder="e.g., Weekly, Twice per week"
                          class="w-full px-3 py-2 bg-gray-50 dark:bg-gray-700 border border-gray-300 dark:border-gray-600 rounded text-gray-900 dark:text-gray-100 focus:outline-none focus:ring-2 focus:ring-blue-500"
                        />
                      </div>
                      <div class="flex justify-end gap-2">
                        <button
                          type="button"
                          onclick={resetInterventionForm}
                          class="px-3 py-1 border border-gray-300 dark:border-gray-600 text-gray-700 dark:text-gray-300 rounded hover:bg-gray-50 dark:hover:bg-gray-700 transition-colors"
                        >
                          Cancel
                        </button>
                        <button
                          type="submit"
                          class="px-3 py-1 bg-blue-600 text-white rounded hover:bg-blue-700 transition-colors"
                        >
                          {editingInterventionId ? 'Update' : 'Add'}
                        </button>
                      </div>
                    </form>
                  {/if}

                  {#if interventions.length === 0}
                    <p class="text-sm text-gray-500 dark:text-gray-400">No interventions yet</p>
                  {:else}
                    <div class="space-y-2">
                      {#each interventions as intervention (intervention.id)}
                        <div class="bg-white dark:bg-gray-800 p-3 rounded border border-gray-200 dark:border-gray-700">
                          <div class="flex justify-between items-start">
                            <div class="flex-1">
                              <div class="flex items-center gap-2 mb-1">
                                <span class="px-2 py-0.5 rounded-full text-xs bg-purple-100 dark:bg-purple-900/30 text-purple-800 dark:text-purple-300">
                                  {intervention.type}
                                </span>
                              </div>
                              <p class="text-sm text-gray-900 dark:text-gray-100">{intervention.description}</p>
                              {#if intervention.frequency}
                                <p class="text-xs text-gray-500 dark:text-gray-400 mt-1">Frequency: {intervention.frequency}</p>
                              {/if}
                            </div>
                            <div class="flex gap-1 ml-2">
                              <button
                                onclick={() => handleEditIntervention(intervention)}
                                class="p-1 text-gray-400 hover:text-blue-500 transition-colors"
                              >
                                <svg xmlns="http://www.w3.org/2000/svg" class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M11 5H6a2 2 0 00-2 2v11a2 2 0 002 2h11a2 2 0 002-2v-5m-1.414-9.414a2 2 0 112.828 2.828L11.828 15H9v-2.828l8.586-8.586z" />
                                </svg>
                              </button>
                              <button
                                onclick={() => handleDeleteIntervention(intervention.id)}
                                class="p-1 text-gray-400 hover:text-red-500 transition-colors"
                              >
                                <svg xmlns="http://www.w3.org/2000/svg" class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16" />
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
