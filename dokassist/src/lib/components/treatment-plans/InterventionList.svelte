<script lang="ts">
  import { get } from 'svelte/store';
  import { Pencil, Plus, Trash2 } from 'lucide-svelte';
  import {
    createTreatmentIntervention,
    deleteTreatmentIntervention,
    updateTreatmentIntervention,
    type TreatmentIntervention,
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
  import { t } from '$lib/translations';
  import { interventionTypeLabel } from '$lib/translations/labels';
  import { INTERVENTION_TYPES, errorMessage } from './options';

  let {
    planId,
    interventions,
    onChange,
    onError,
  }: {
    planId: string;
    interventions: TreatmentIntervention[];
    onChange: () => void | Promise<void>;
    onError: (message: string) => void;
  } = $props();

  let formOpen = $state(false);
  let editing = $state<TreatmentIntervention | null>(null);
  let type = $state('psychotherapy');
  let description = $state('');
  let frequency = $state('');
  let saving = $state(false);
  let formError = $state<string | null>(null);

  let deleting = $state<TreatmentIntervention | null>(null);
  let confirmOpen = $state(false);

  function openForm(intervention: TreatmentIntervention | null) {
    editing = intervention;
    type = intervention?.type ?? 'psychotherapy';
    description = intervention?.description ?? '';
    frequency = intervention?.frequency ?? '';
    formError = null;
    formOpen = true;
  }

  async function handleSubmit(event: Event) {
    event.preventDefault();
    if (!description) {
      formError = get(t)('treatmentPlans.interventionDescriptionRequired');
      return;
    }
    try {
      saving = true;
      formError = null;
      if (editing) {
        await updateTreatmentIntervention(editing.id, {
          type,
          description,
          frequency: frequency || undefined,
        });
      } else {
        await createTreatmentIntervention({
          treatment_plan_id: planId,
          type,
          description,
          frequency: frequency || undefined,
        });
      }
      formOpen = false;
      await onChange();
    } catch (err) {
      formError = errorMessage(get(t)('common.saveFailed'), err);
      console.error('Failed to save intervention:', err);
    } finally {
      saving = false;
    }
  }

  function askDelete(intervention: TreatmentIntervention) {
    deleting = intervention;
    confirmOpen = true;
  }

  async function confirmDelete() {
    if (!deleting) return;
    try {
      await deleteTreatmentIntervention(deleting.id);
      await onChange();
    } catch (err) {
      onError(errorMessage(get(t)('common.deleteFailed'), err));
      console.error('Failed to delete intervention:', err);
    }
  }
</script>

<section>
  <SectionHeader title={$t('treatmentPlans.interventions')} level={4}>
    {#snippet actions()}
      <Button size="sm" variant="ghost" onclick={() => openForm(null)}>
        <Plus size={14} aria-hidden="true" />
        {$t('treatmentPlans.addIntervention')}
      </Button>
    {/snippet}
  </SectionHeader>

  {#if interventions.length === 0}
    <p class="text-label text-fg-muted">{$t('treatmentPlans.noInterventions')}</p>
  {:else}
    <ul class="divide-y divide-line-subtle rounded-card border border-line-subtle">
      {#each interventions as intervention (intervention.id)}
        <li class="flex items-start gap-3 px-3 py-2">
          <div class="min-w-0 flex-1">
            <p class="text-body text-fg [overflow-wrap:anywhere]">{intervention.description}</p>
            <div class="mt-1 flex flex-wrap items-center gap-2">
              <Badge>{$interventionTypeLabel(intervention.type)}</Badge>
              {#if intervention.frequency}
                <span class="text-caption text-fg-muted">
                  {$t('treatmentPlans.frequencyValue').replace('{value}', intervention.frequency)}
                </span>
              {/if}
            </div>
          </div>
          <Menu
            size="sm"
            label={$t('common.moreActions')}
            items={[
              {
                label: $t('treatmentPlans.editIntervention'),
                icon: Pencil,
                onSelect: () => openForm(intervention),
              },
              {
                label: $t('treatmentPlans.deleteIntervention'),
                icon: Trash2,
                danger: true,
                onSelect: () => askDelete(intervention),
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
  title={editing ? $t('treatmentPlans.editIntervention') : $t('treatmentPlans.addIntervention')}
>
  <form id="intervention-form-{planId}" onsubmit={handleSubmit} class="space-y-4">
    {#if formError}
      <Alert tone="danger">{formError}</Alert>
    {/if}
    <Field label={$t('treatmentPlans.interventionTypeLabel')} for="intervention-type" required>
      <Select id="intervention-type" bind:value={type} required>
        {#each INTERVENTION_TYPES as option (option)}
          <option value={option}>{$interventionTypeLabel(option)}</option>
        {/each}
      </Select>
    </Field>
    <Field label={$t('treatmentPlans.description')} for="intervention-description" required>
      <Textarea id="intervention-description" bind:value={description} rows={2} required />
    </Field>
    <Field label={$t('treatmentPlans.frequency')} for="intervention-frequency">
      <Input
        id="intervention-frequency"
        bind:value={frequency}
        placeholder={$t('treatmentPlans.frequencyPlaceholder')}
      />
    </Field>
  </form>
  {#snippet footer()}
    <Button onclick={() => (formOpen = false)} disabled={saving}>{$t('common.cancel')}</Button>
    <Button type="submit" variant="primary" form="intervention-form-{planId}" loading={saving}>
      {editing ? $t('common.update') : $t('common.add')}
    </Button>
  {/snippet}
</Dialog>

<ConfirmDialog
  bind:open={confirmOpen}
  tone="danger"
  title={$t('treatmentPlans.deleteIntervention')}
  message={$t('treatmentPlans.confirmDeleteInterventionNamed').replace(
    '{name}',
    deleting?.description ?? ''
  )}
  onConfirm={confirmDelete}
/>
