<script lang="ts">
  import { get } from 'svelte/store';
  import { Pencil, Plus, Trash2 } from 'lucide-svelte';
  import {
    createTreatmentGoal,
    deleteTreatmentGoal,
    updateTreatmentGoal,
    type TreatmentGoal,
  } from '$lib/api';
  import {
    Alert,
    Badge,
    Button,
    ConfirmDialog,
    Dialog,
    Field,
    Input,
    Menu,
    SectionHeader,
    Select,
    Textarea,
  } from '$lib/components/ui';
  import { fmt } from '$lib/format';
  import { t } from '$lib/translations';
  import { goalStatusLabel } from '$lib/translations/labels';
  import { GOAL_STATUSES, errorMessage, statusTone } from './options';

  let {
    planId,
    goals,
    onChange,
    onError,
  }: {
    planId: string;
    goals: TreatmentGoal[];
    /** Reload goals after a write. */
    onChange: () => void | Promise<void>;
    onError: (message: string) => void;
  } = $props();

  let formOpen = $state(false);
  let editing = $state<TreatmentGoal | null>(null);
  let description = $state('');
  let targetDate = $state('');
  let status = $state('in_progress');
  let sortOrder = $state(0);
  let saving = $state(false);
  let formError = $state<string | null>(null);

  let deleting = $state<TreatmentGoal | null>(null);
  let confirmOpen = $state(false);

  function openForm(goal: TreatmentGoal | null) {
    editing = goal;
    description = goal?.description ?? '';
    targetDate = goal?.target_date ?? '';
    status = goal?.status ?? 'in_progress';
    sortOrder =
      goal?.sort_order ?? (goals.length > 0 ? Math.max(...goals.map((g) => g.sort_order)) + 1 : 0);
    formError = null;
    formOpen = true;
  }

  async function handleSubmit(event: Event) {
    event.preventDefault();
    if (!description) {
      formError = get(t)('treatmentPlans.goalDescriptionRequired');
      return;
    }
    try {
      saving = true;
      formError = null;
      if (editing) {
        await updateTreatmentGoal(editing.id, {
          description,
          target_date: targetDate || undefined,
          status,
          sort_order: sortOrder,
        });
      } else {
        await createTreatmentGoal({
          treatment_plan_id: planId,
          description,
          target_date: targetDate || undefined,
          status,
          sort_order: sortOrder,
        });
      }
      formOpen = false;
      await onChange();
    } catch (err) {
      formError = errorMessage(get(t)('common.saveFailed'), err);
      console.error('Failed to save goal:', err);
    } finally {
      saving = false;
    }
  }

  function askDelete(goal: TreatmentGoal) {
    deleting = goal;
    confirmOpen = true;
  }

  async function confirmDelete() {
    if (!deleting) return;
    try {
      await deleteTreatmentGoal(deleting.id);
      await onChange();
    } catch (err) {
      onError(errorMessage(get(t)('common.deleteFailed'), err));
      console.error('Failed to delete goal:', err);
    }
  }
</script>

<section>
  <SectionHeader title={$t('treatmentPlans.goals')} level={4}>
    {#snippet actions()}
      <Button size="sm" variant="ghost" onclick={() => openForm(null)}>
        <Plus size={14} aria-hidden="true" />
        {$t('treatmentPlans.addGoal')}
      </Button>
    {/snippet}
  </SectionHeader>

  {#if goals.length === 0}
    <p class="text-label text-fg-muted">{$t('treatmentPlans.noGoals')}</p>
  {:else}
    <ul class="divide-y divide-line-subtle rounded-card border border-line-subtle">
      {#each goals as goal (goal.id)}
        <li class="flex items-start gap-3 px-3 py-2">
          <div class="min-w-0 flex-1">
            <p class="text-body text-fg [overflow-wrap:anywhere]">{goal.description}</p>
            <div class="mt-1 flex flex-wrap items-center gap-2">
              <Badge tone={statusTone(goal.status)}>{$goalStatusLabel(goal.status)}</Badge>
              {#if goal.target_date}
                <span class="text-caption text-fg-muted" data-numeric>
                  {$t('treatmentPlans.targetDateValue').replace(
                    '{date}',
                    $fmt.date(goal.target_date)
                  )}
                </span>
              {/if}
            </div>
          </div>
          <Menu
            size="sm"
            label={$t('common.moreActions')}
            items={[
              {
                label: $t('treatmentPlans.editGoal'),
                icon: Pencil,
                onSelect: () => openForm(goal),
              },
              {
                label: $t('treatmentPlans.deleteGoal'),
                icon: Trash2,
                danger: true,
                onSelect: () => askDelete(goal),
              },
            ]}
          />
        </li>
      {/each}
    </ul>
  {/if}
</section>

<Dialog
  bind:open={formOpen}
  title={editing ? $t('treatmentPlans.editGoal') : $t('treatmentPlans.addGoal')}
>
  <form id="goal-form-{planId}" onsubmit={handleSubmit} class="space-y-4">
    {#if formError}
      <Alert tone="danger">{formError}</Alert>
    {/if}
    <Field label={$t('treatmentPlans.description')} for="goal-description" required>
      <Textarea id="goal-description" bind:value={description} rows={2} required />
    </Field>
    <div class="grid grid-cols-1 gap-4 sm:grid-cols-3">
      <Field label={$t('treatmentPlans.targetDate')} for="goal-target-date">
        <Input id="goal-target-date" type="date" bind:value={targetDate} />
      </Field>
      <Field label={$t('treatmentPlans.statusLabel')} for="goal-status">
        <Select id="goal-status" bind:value={status}>
          {#each GOAL_STATUSES as option (option)}
            <option value={option}>{$goalStatusLabel(option)}</option>
          {/each}
        </Select>
      </Field>
      <Field label={$t('treatmentPlans.priority')} for="goal-priority">
        <Input id="goal-priority" type="number" bind:value={sortOrder} min="0" />
      </Field>
    </div>
  </form>
  {#snippet footer()}
    <Button onclick={() => (formOpen = false)} disabled={saving}>{$t('common.cancel')}</Button>
    <Button type="submit" variant="primary" form="goal-form-{planId}" loading={saving}>
      {editing ? $t('common.update') : $t('common.add')}
    </Button>
  {/snippet}
</Dialog>

<ConfirmDialog
  bind:open={confirmOpen}
  tone="danger"
  title={$t('treatmentPlans.deleteGoal')}
  message={$t('treatmentPlans.confirmDeleteGoalNamed').replace(
    '{name}',
    deleting?.description ?? ''
  )}
  onConfirm={confirmDelete}
/>
