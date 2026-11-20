<script lang="ts">
  import type { ModelInfo } from '$lib/api';
  import { Badge, Button, SettingsRow } from '$lib/components/ui';
  import { fmt } from '$lib/format';
  import { t } from '$lib/translations';
  import { formatGigabytes } from './format';

  /* One installed writing model: state badges, verification details, and its actions. */
  let {
    model,
    busy = false,
    onLoad,
    onSetDefault,
    onDelete,
  }: {
    model: ModelInfo;
    /** A download or load is in progress: don't start another load. */
    busy?: boolean;
    onLoad: (model: ModelInfo) => void;
    onSetDefault: (model: ModelInfo) => void;
    onDelete: (model: ModelInfo) => void;
  } = $props();
</script>

<SettingsRow
  label={model.name}
  description={`${model.filename} · ${formatGigabytes(model.size_bytes)}`}
>
  {#snippet control()}
    {#if model.exists_on_disk && !model.is_loaded}
      <Button size="sm" onclick={() => onLoad(model)} disabled={busy}>
        {$t('settings.load')}
      </Button>
    {/if}
    {#if model.exists_on_disk && !model.is_default}
      <Button size="sm" onclick={() => onSetDefault(model)}>{$t('settings.setDefault')}</Button>
    {/if}
    {#if !model.is_loaded}
      <Button size="sm" variant="danger" onclick={() => onDelete(model)}>
        {$t('settings.delete')}
      </Button>
    {/if}
  {/snippet}

  {#if model.is_default || model.is_loaded || model.quantization_promotion || !model.exists_on_disk || model.last_used}
    <div class="mt-1.5 flex flex-wrap items-center gap-1.5">
      {#if model.is_loaded}
        <Badge tone="success">{$t('settings.loadedBadge')}</Badge>
      {/if}
      {#if model.is_default}
        <Badge>{$t('settings.defaultBadge')}</Badge>
      {/if}
      {#if model.quantization_promotion}
        <Badge tone="info">{$t('settings.clinicalGateBadge')}</Badge>
      {/if}
      {#if !model.exists_on_disk}
        <Badge tone="danger">{$t('settings.modelMissingOnDisk')}</Badge>
      {/if}
      {#if model.last_used}
        <span class="text-caption text-fg-subtle">
          {$t('settings.lastUsed').replace('{date}', $fmt.date(model.last_used))}
        </span>
      {/if}
    </div>
  {/if}

  {#if model.quantization_promotion}
    {@const promotion = model.quantization_promotion}
    <details class="mt-2 rounded-control border border-line-subtle px-2.5 py-1.5">
      <summary class="cursor-pointer text-caption text-fg">
        {$t('settings.clinicalGateDetails')}
      </summary>
      <div class="mt-2 space-y-1 pb-1">
        <p class="text-caption text-fg">
          {$t('settings.clinicalGateSummary')
            .replace('{quantization}', promotion.quantization)
            .replace('{baselines}', promotion.baseline_artifacts.join(', '))}
        </p>
        <p class="text-caption text-fg-muted">
          {$t('settings.clinicalGateStudy').replace('{study}', promotion.study_id)}
        </p>
        <p class="text-caption text-fg-muted">
          {$t('settings.clinicalGateEvidence').replace(
            '{regression}',
            (promotion.worst_category_regression * 100).toFixed(2)
          )}
        </p>
        <p class="text-caption text-fg-subtle">
          {$t('settings.clinicalGateHeldOut').replace(
            '{hash}',
            promotion.held_out_results_sha256.slice(0, 12)
          )}
        </p>
        <p class="mt-1 text-caption text-fg-muted">{$t('settings.promotionDisclaimer')}</p>
      </div>
    </details>
  {/if}
</SettingsRow>
