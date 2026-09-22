<script lang="ts">
  import { untrack } from 'svelte';
  import type {
    CreateMedication,
    UpdateMedication,
    Medication,
    SubstanceSummary,
    SubstanceDetail,
  } from '$lib/api';
  import { getMedicationReferenceDetail, searchMedicationReference } from '$lib/api';
  import { t } from '$lib/translations';
  import { Button, Field, Input, Select, Textarea } from '$lib/components/ui';
  import MedicationAutocomplete from './MedicationAutocomplete.svelte';
  import MedicationInfoPanel from './MedicationInfoPanel.svelte';
  import MedicationChangeAssistant from './MedicationChangeAssistant.svelte';

  interface Props {
    medication?: Medication;
    patientId?: string;
    activeMedications?: Medication[];
    onSave: (
      input: CreateMedication | { id: string; update: UpdateMedication },
      replacingMedicationId?: string | null
    ) => void;
    onCancel: () => void;
  }

  let { medication, patientId, activeMedications = [], onSave, onCancel }: Props = $props();

  let substance = $state(untrack(() => medication?.substance || ''));
  let dosage = $state(untrack(() => medication?.dosage || ''));
  let frequency = $state(untrack(() => medication?.frequency || ''));
  let startDate = $state(
    untrack(() => medication?.start_date || new Date().toISOString().split('T')[0])
  );
  let endDate = $state(untrack(() => medication?.end_date || ''));
  let notes = $state(untrack(() => medication?.notes || ''));
  let selectedSubstanceId = $state<string | null>(null);
  let selectedSubstanceDetail = $state<SubstanceDetail | null>(null);

  // Replacement medication state
  let isReplacement = $state(false);
  let replacingMedicationId = $state<string | null>(null);
  let replacingMedication = $state<Medication | null>(null);
  let replacingSubstanceDetail = $state<SubstanceDetail | null>(null);
  let showComparisonAssistant = $state(false);

  $effect(() => {
    if (medication) {
      substance = medication.substance || '';
      dosage = medication.dosage || '';
      frequency = medication.frequency || '';
      startDate = medication.start_date || new Date().toISOString().split('T')[0];
      endDate = medication.end_date || '';
      notes = medication.notes || '';
      // Clear any reference panel when editing an existing record
      selectedSubstanceId = null;
      selectedSubstanceDetail = null;
      isReplacement = false;
      replacingMedicationId = null;
    }
  });

  // Load substance detail when selected
  $effect(() => {
    if (selectedSubstanceId) {
      const currentId = selectedSubstanceId;
      getMedicationReferenceDetail(currentId)
        .then((detail) => {
          if (selectedSubstanceId === currentId) {
            selectedSubstanceDetail = detail;
          }
        })
        .catch((err) => {
          console.error('Failed to load substance detail:', err);
          if (selectedSubstanceId === currentId) {
            selectedSubstanceDetail = null;
          }
        });
    } else {
      selectedSubstanceDetail = null;
    }
  });

  // Load replacing medication substance detail
  $effect(() => {
    if (replacingMedicationId) {
      const currentReplacingId = replacingMedicationId;
      const med = activeMedications.find((m) => m.id === currentReplacingId);
      replacingMedication = med ?? null;
      replacingSubstanceDetail = null;
      showComparisonAssistant = false;
      if (med) {
        searchMedicationReference(med.substance)
          .then((results) => {
            if (results.length > 0) {
              return getMedicationReferenceDetail(results[0].id);
            }
            return null;
          })
          .then((detail) => {
            if (replacingMedicationId === currentReplacingId) {
              replacingSubstanceDetail = detail;
            }
          })
          .catch((err) => {
            console.error('Failed to load replacing substance detail:', err);
          });
      }
    } else {
      replacingMedication = null;
      replacingSubstanceDetail = null;
    }
  });

  function handleSubstanceSelect(summary: SubstanceSummary) {
    substance = summary.name_de;
    selectedSubstanceId = summary.id;
  }

  function handleReplacementToggle() {
    isReplacement = !isReplacement;
    if (!isReplacement) {
      replacingMedicationId = null;
      showComparisonAssistant = false;
    }
  }

  function handleShowComparison() {
    if (selectedSubstanceDetail && replacingSubstanceDetail) {
      showComparisonAssistant = true;
    }
  }

  function handleSubmit(event: Event) {
    event.preventDefault();

    if (!substance.trim() || !dosage.trim() || !frequency.trim()) {
      return;
    }

    if (medication) {
      const update: UpdateMedication = {
        substance: substance !== medication.substance ? substance : undefined,
        dosage: dosage !== medication.dosage ? dosage : undefined,
        frequency: frequency !== medication.frequency ? frequency : undefined,
        start_date: startDate !== medication.start_date ? startDate : undefined,
        end_date: endDate !== (medication.end_date || '') ? endDate || undefined : undefined,
        notes: notes !== (medication.notes || '') ? notes || undefined : undefined,
      };
      onSave({ id: medication.id, update });
    } else if (patientId) {
      const input: CreateMedication = {
        patient_id: patientId,
        substance,
        dosage,
        frequency,
        start_date: startDate,
        end_date: endDate || undefined,
        notes: notes || undefined,
      };
      onSave(input, isReplacement ? replacingMedicationId : null);
    }
  }
</script>

<form onsubmit={handleSubmit} class="space-y-4">
  <!-- Replacement Option -->
  {#if !medication && activeMedications.length > 0}
    <div class="bg-surface-sunken border border-line rounded-card p-4">
      <label class="flex items-center gap-2 cursor-pointer">
        <input
          type="checkbox"
          checked={isReplacement}
          onchange={handleReplacementToggle}
          class="w-4 h-4 text-accent-fg border-line rounded-control focus:ring-accent/30"
        />
        <span class="text-body font-medium text-fg-muted">
          {$t('medications.replacesExisting')}
        </span>
      </label>

      {#if isReplacement}
        <Field
          label={$t('medications.medicationToReplace')}
          for="replacing-medication"
          required
          class="mt-3"
        >
          <Select
            id="replacing-medication"
            bind:value={replacingMedicationId}
            required={isReplacement}
          >
            <option value={null}>{$t('common.pleaseSelect')}</option>
            {#each activeMedications as med (med.id)}
              <option value={med.id}>
                {med.substance} ({med.dosage}, {med.frequency})
              </option>
            {/each}
          </Select>

          {#if replacingMedicationId && selectedSubstanceDetail && replacingSubstanceDetail && patientId}
            <Button onclick={handleShowComparison} full class="mt-2">
              {$t('medications.showComparison')}
            </Button>
          {/if}
        </Field>
      {/if}
    </div>
  {/if}

  <!-- Show Comparison Assistant -->
  {#if showComparisonAssistant && selectedSubstanceDetail && replacingSubstanceDetail && patientId}
    <div class="border-t border-line pt-4">
      <MedicationChangeAssistant
        {patientId}
        currentSubstance={replacingSubstanceDetail}
        replacementSubstance={selectedSubstanceDetail}
      />
    </div>
  {/if}

  <Field label={$t('medications.substance')} for="substance" required>
    <MedicationAutocomplete
      id="substance"
      value={substance}
      onInput={(v) => {
        substance = v;
        selectedSubstanceId = null;
      }}
      onSelect={handleSubstanceSelect}
      required
      placeholder={$t('medications.substancePlaceholder')}
    />
    <MedicationInfoPanel substanceId={selectedSubstanceId} />
  </Field>

  <Field label={$t('medications.dosage')} for="dosage" required>
    <Input
      id="dosage"
      bind:value={dosage}
      required
      placeholder={$t('medications.dosagePlaceholder')}
    />
  </Field>

  <Field label={$t('medications.frequency')} for="frequency" required>
    <Input
      id="frequency"
      bind:value={frequency}
      required
      placeholder={$t('medications.frequencyPlaceholder')}
    />
  </Field>

  <div class="grid grid-cols-1 gap-4 sm:grid-cols-2">
    <Field label={$t('medications.startDate')} for="start-date" required>
      <Input id="start-date" type="date" bind:value={startDate} required />
    </Field>

    <Field label={$t('medications.endDate')} for="end-date">
      <Input id="end-date" type="date" bind:value={endDate} />
    </Field>
  </div>

  <Field label={$t('medications.notes')} for="notes">
    <Textarea
      id="notes"
      bind:value={notes}
      rows={3}
      placeholder={$t('medications.notesPlaceholder')}
      class="resize-none"
    />
  </Field>

  <div class="flex justify-end gap-3 pt-4">
    <Button onclick={onCancel}>{$t('common.cancel')}</Button>
    <Button type="submit" variant="primary">
      {medication ? $t('common.update') : $t('common.add')}
    </Button>
  </div>
</form>
