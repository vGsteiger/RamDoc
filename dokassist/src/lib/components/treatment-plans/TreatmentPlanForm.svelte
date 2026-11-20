<script lang="ts">
  import { get } from 'svelte/store';
  import {
    createTreatmentPlan,
    updateTreatmentPlan,
    type CreateTreatmentPlan,
    type TreatmentPlan,
    type UpdateTreatmentPlan,
  } from '$lib/api';
  import { Alert, Button, Dialog, Field, Input, Select, Textarea } from '$lib/components/ui';
  import { t } from '$lib/translations';
  import { planStatusLabel } from '$lib/translations/labels';
  import { PLAN_STATUSES, errorMessage, today } from './options';

  /** Create or edit a treatment plan in a dialog. Remount (or reopen) to edit another plan. */
  let {
    open = $bindable(false),
    patientId,
    plan = null,
    onSaved,
  }: {
    open?: boolean;
    patientId: string;
    plan?: TreatmentPlan | null;
    onSaved: () => void | Promise<void>;
  } = $props();

  let title = $state('');
  let description = $state('');
  let startDate = $state(today());
  let endDate = $state('');
  let status = $state('active');
  let saving = $state(false);
  let error = $state<string | null>(null);

  // Snapshot the plan each time the dialog opens.
  $effect(() => {
    if (!open) return;
    title = plan?.title ?? '';
    description = plan?.description ?? '';
    startDate = plan?.start_date ?? today();
    endDate = plan?.end_date ?? '';
    status = plan?.status ?? 'active';
    error = null;
  });

  async function handleSubmit(event: Event) {
    event.preventDefault();
    if (!title) {
      error = get(t)('treatmentPlans.titleRequired');
      return;
    }
    try {
      saving = true;
      error = null;
      if (plan) {
        const update: UpdateTreatmentPlan = {
          title,
          description: description || undefined,
          start_date: startDate,
          end_date: endDate || undefined,
          status,
        };
        await updateTreatmentPlan(plan.id, update);
      } else {
        const input: CreateTreatmentPlan = {
          patient_id: patientId,
          title,
          description: description || undefined,
          start_date: startDate,
          end_date: endDate || undefined,
          status,
        };
        await createTreatmentPlan(input);
      }
      open = false;
      await onSaved();
    } catch (err) {
      error = errorMessage(get(t)('common.saveFailed'), err);
      console.error('Failed to save plan:', err);
    } finally {
      saving = false;
    }
  }
</script>

<Dialog
  bind:open
  size="lg"
  title={plan ? $t('treatmentPlans.editPlan') : $t('treatmentPlans.newPlan')}
>
  <form id="treatment-plan-form" onsubmit={handleSubmit} class="space-y-4">
    {#if error}
      <Alert tone="danger">{error}</Alert>
    {/if}
    <Field label={$t('treatmentPlans.planTitle')} for="plan-title" required>
      <Input id="plan-title" bind:value={title} required />
    </Field>
    <Field label={$t('treatmentPlans.description')} for="plan-description">
      <Textarea id="plan-description" bind:value={description} rows={3} />
    </Field>
    <div class="grid grid-cols-1 gap-4 sm:grid-cols-3">
      <Field label={$t('treatmentPlans.startDate')} for="plan-start" required>
        <Input id="plan-start" type="date" bind:value={startDate} required />
      </Field>
      <Field label={$t('treatmentPlans.endDate')} for="plan-end">
        <Input id="plan-end" type="date" bind:value={endDate} />
      </Field>
      <Field label={$t('treatmentPlans.statusLabel')} for="plan-status" required>
        <Select id="plan-status" bind:value={status} required>
          {#each PLAN_STATUSES as option (option)}
            <option value={option}>{$planStatusLabel(option)}</option>
          {/each}
        </Select>
      </Field>
    </div>
  </form>

  {#snippet footer()}
    <Button onclick={() => (open = false)} disabled={saving}>{$t('common.cancel')}</Button>
    <Button type="submit" variant="primary" form="treatment-plan-form" loading={saving}>
      {plan ? $t('common.update') : $t('common.create')}
    </Button>
  {/snippet}
</Dialog>
