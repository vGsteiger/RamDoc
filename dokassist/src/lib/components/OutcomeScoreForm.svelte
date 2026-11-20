<script lang="ts">
  import { t } from '$lib/translations';
  import { untrack } from 'svelte';
  import type { CreateOutcomeScore, UpdateOutcomeScore, OutcomeScore } from '$lib/api';
  import { Button, Field, Input, Select, Textarea } from '$lib/components/ui';

  interface Props {
    outcomeScore?: OutcomeScore;
    sessionId?: string;
    onSave: (input: CreateOutcomeScore | { id: string; update: UpdateOutcomeScore }) => void;
    onCancel: () => void;
  }

  let { outcomeScore, sessionId, onSave, onCancel }: Props = $props();

  // The form intentionally snapshots the score it was opened with; a different
  // score is edited by remounting the form, not by swapping the prop.
  const initial = untrack(() => ({
    scaleType: outcomeScore?.scale_type || 'PHQ-9',
    score: outcomeScore?.score?.toString() || '',
    administeredAt: outcomeScore?.administered_at || new Date().toISOString().split('T')[0],
    notes: outcomeScore?.notes || '',
  }));

  let scaleType = $state(initial.scaleType);
  let score = $state(initial.score);
  let administeredAt = $state(initial.administeredAt);
  let notes = $state(initial.notes);

  // `value` is persisted; the label is translated.
  const scaleOptions = [
    { value: 'PHQ-9', labelKey: 'outcomeScores.scales.phq9', max: 27 },
    { value: 'GAD-7', labelKey: 'outcomeScores.scales.gad7', max: 21 },
    { value: 'BDI-II', labelKey: 'outcomeScores.scales.bdiii', max: 63 },
  ];

  let maxScore = $derived(scaleOptions.find((s) => s.value === scaleType)?.max || 27);

  function handleSubmit(event: Event) {
    event.preventDefault();

    const scoreValue = parseInt(score, 10);
    if (isNaN(scoreValue) || scoreValue < 0 || scoreValue > maxScore) {
      return;
    }

    if (outcomeScore) {
      // Update existing score
      const update: UpdateOutcomeScore = {
        scale_type: scaleType !== outcomeScore.scale_type ? scaleType : undefined,
        score: scoreValue !== outcomeScore.score ? scoreValue : undefined,
        administered_at:
          administeredAt !== outcomeScore.administered_at ? administeredAt : undefined,
        notes: notes !== (outcomeScore.notes || '') ? notes || undefined : undefined,
      };
      onSave({ id: outcomeScore.id, update });
    } else if (sessionId) {
      // Create new score
      const input: CreateOutcomeScore = {
        session_id: sessionId,
        scale_type: scaleType,
        score: scoreValue,
        administered_at: administeredAt,
        notes: notes || undefined,
      };
      onSave(input);
    }
  }
</script>

<form onsubmit={handleSubmit} class="space-y-4">
  <Field label={$t('outcomeScores.questionnaire')} for="scale-type" required>
    <Select id="scale-type" bind:value={scaleType} required>
      {#each scaleOptions as option (option.value)}
        <option value={option.value}>{$t(option.labelKey)}</option>
      {/each}
    </Select>
  </Field>

  <div class="grid grid-cols-1 gap-4 sm:grid-cols-2">
    <Field
      label={$t('outcomeScores.totalScoreLabel')}
      for="score"
      required
      hint={$t('outcomeScores.scoreRange').replace('{max}', String(maxScore))}
    >
      <Input
        id="score"
        type="number"
        bind:value={score}
        required
        min="0"
        max={maxScore}
        placeholder={$t('outcomeScores.scorePlaceholder')}
      />
    </Field>

    <Field label={$t('outcomeScores.administeredOnLabel')} for="administered-at" required>
      <Input id="administered-at" type="date" bind:value={administeredAt} required />
    </Field>
  </div>

  <Field label={$t('common.notes')} for="score-notes">
    <Textarea
      id="score-notes"
      bind:value={notes}
      rows={3}
      placeholder={$t('outcomeScores.notesPlaceholder')}
    />
  </Field>

  <div class="flex justify-end gap-2 pt-2">
    <Button onclick={onCancel}>{$t('common.cancel')}</Button>
    <Button type="submit" variant="primary">
      {outcomeScore ? $t('common.update') : $t('common.add')}
    </Button>
  </div>
</form>
