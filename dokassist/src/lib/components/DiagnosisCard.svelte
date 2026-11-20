<script lang="ts">
  import { Pencil, Trash2 } from 'lucide-svelte';
  import { t } from '$lib/translations';
  import type { Diagnosis } from '$lib/api';
  import { Badge, Menu, type MenuItem } from '$lib/components/ui';
  import { fmt } from '$lib/format';

  /* One diagnosis: ICD-10 code, status, description, dates; Edit and Delete in a ⋯ menu. */
  interface Props {
    diagnosis: Diagnosis;
    onEdit?: () => void;
    onDelete?: () => void;
  }

  let { diagnosis, onEdit, onDelete }: Props = $props();

  const tones: Record<string, 'success' | 'warning' | 'neutral'> = {
    active: 'success',
    remission: 'warning',
    resolved: 'neutral',
  };

  function statusLabel(status: string): string {
    return ['active', 'remission', 'resolved'].includes(status)
      ? $t(`diagnoses.${status}`)
      : status;
  }

  let items = $derived<MenuItem[]>([
    ...(onEdit ? [{ label: $t('diagnoses.editDiagnosis'), icon: Pencil, onSelect: onEdit }] : []),
    ...(onDelete
      ? [{ label: $t('diagnoses.deleteDiagnosis'), icon: Trash2, danger: true, onSelect: onDelete }]
      : []),
  ]);
</script>

<div class="flex items-start gap-3 border-b border-line-subtle px-3 py-2.5 last:border-b-0">
  <span class="w-16 shrink-0 pt-px font-mono text-label text-fg">{diagnosis.icd10_code}</span>
  <div class="min-w-0 flex-1">
    <div class="flex flex-wrap items-center gap-x-2 gap-y-1">
      <h3 class="min-w-0 text-body text-fg [overflow-wrap:anywhere]">{diagnosis.description}</h3>
      <Badge tone={tones[diagnosis.status] ?? 'neutral'}>{statusLabel(diagnosis.status)}</Badge>
    </div>
    <p class="mt-0.5 text-caption text-fg-muted" data-numeric>
      {$t('diagnoses.diagnosedOnValue').replace('{date}', $fmt.date(diagnosis.diagnosed_date))}
      {#if diagnosis.resolved_date}
        <span aria-hidden="true" class="px-1">·</span>
        {$t('diagnoses.resolvedOnValue').replace('{date}', $fmt.date(diagnosis.resolved_date))}
      {/if}
    </p>
    {#if diagnosis.notes}
      <p class="mt-1 text-label text-fg-muted">{diagnosis.notes}</p>
    {/if}
  </div>
  {#if items.length > 0}
    <Menu size="sm" {items} label={$t('common.moreActions')} />
  {/if}
</div>
