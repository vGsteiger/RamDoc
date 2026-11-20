<script lang="ts">
  import { Pencil, Trash2 } from 'lucide-svelte';
  import { interpretationLabel } from '$lib/translations/labels';
  import { t } from '$lib/translations';
  import type { OutcomeScore } from '$lib/api';
  import { Badge, Menu, type MenuItem } from '$lib/components/ui';
  import { fmt } from '$lib/format';
  import { severityTone } from '$lib/components/patient/workspace';

  /* One questionnaire result: scale, score, interpretation and date; Edit/Delete in a ⋯ menu. */
  interface Props {
    outcomeScore: OutcomeScore;
    onEdit?: () => void;
    /** Ask for confirmation before calling the API; the caller owns the ConfirmDialog. */
    onDelete?: () => void;
  }

  let { outcomeScore, onEdit, onDelete }: Props = $props();

  let items = $derived<MenuItem[]>([
    ...(onEdit ? [{ label: $t('outcomeScores.editScore'), icon: Pencil, onSelect: onEdit }] : []),
    ...(onDelete
      ? [{ label: $t('outcomeScores.deleteScore'), icon: Trash2, danger: true, onSelect: onDelete }]
      : []),
  ]);
</script>

<div class="flex items-start gap-3 card-surface rounded-panel px-3 py-2.5">
  <div class="min-w-0 flex-1">
    <div class="flex flex-wrap items-baseline gap-x-3 gap-y-1">
      <span class="text-label font-medium text-fg">{outcomeScore.scale_type}</span>
      <span class="text-heading text-fg" data-numeric>
        {outcomeScore.score}
        <span class="text-label text-fg-muted">{$t('outcomeScores.points')}</span>
      </span>
      {#if outcomeScore.interpretation}
        <Badge tone={severityTone(outcomeScore.interpretation)}>
          {$interpretationLabel(outcomeScore.interpretation)}
        </Badge>
      {/if}
      <span class="text-caption text-fg-muted" data-numeric>
        {$fmt.date(outcomeScore.administered_at)}
      </span>
    </div>
    {#if outcomeScore.notes}
      <p class="mt-1 text-label text-fg-muted">{outcomeScore.notes}</p>
    {/if}
  </div>
  {#if items.length > 0}
    <Menu size="sm" {items} label={$t('common.moreActions')} />
  {/if}
</div>
