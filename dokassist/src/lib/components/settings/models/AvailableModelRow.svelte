<script lang="ts">
  import type { AvailableModel, ModelChoice } from '$lib/api';
  import { Badge, Button, ProgressBar, SettingsRow } from '$lib/components/ui';
  import { t } from '$lib/translations';
  import { catalogText, formatGigabytes, hasEnoughRam } from './format';

  /* One downloadable writing model from the catalogue. */
  let {
    model,
    totalRamBytes,
    recommended = false,
    downloading = false,
    progress = 0,
    disabled = false,
    onDownload,
  }: {
    model: AvailableModel;
    totalRamBytes: number | null | undefined;
    recommended?: boolean;
    /** This model is the one downloading right now. */
    downloading?: boolean;
    progress?: number | null;
    /** Another download or load is running. */
    disabled?: boolean;
    onDownload: (choice: ModelChoice) => void;
  } = $props();

  let description = $derived(
    catalogText($t, 'descriptions', model.filename, model.description)
  );
  let disclaimer = $derived(
    model.disclaimer ? catalogText($t, 'disclaimers', model.filename, model.disclaimer) : null
  );
  let canRun = $derived(hasEnoughRam(totalRamBytes, model.min_ram_gb));
  let ramLabel = $derived(totalRamBytes ? formatGigabytes(totalRamBytes) : '…');

  function download() {
    onDownload({
      name: model.name,
      filename: model.filename,
      size_bytes: model.size_bytes,
      reason: model.description,
    });
  }
</script>

<SettingsRow label={model.name} {description}>
  {#snippet control()}
    {#if !model.is_downloaded}
      <Button onclick={download} disabled={disabled || !canRun} loading={downloading}>
        {downloading ? $t('settings.downloadingEllipsis') : $t('settings.downloadModel')}
      </Button>
    {/if}
  {/snippet}

  <div class="mt-1.5 flex flex-wrap items-center gap-1.5">
    {#if model.is_downloaded}
      <Badge tone="success">{$t('settings.downloadedBadge')}</Badge>
    {/if}
    {#if recommended}
      <Badge tone="info">{$t('settings.models.recommendedBadge')}</Badge>
    {/if}
    {#if canRun}
      <Badge>{$t('settings.compatibleBadge')}</Badge>
    {:else}
      <Badge tone="warning">
        {$t('settings.needsRam').replace('{gb}', String(model.min_ram_gb))}
      </Badge>
    {/if}
  </div>
  {#if model.context_window_tokens}
    <p class="mt-1.5 text-caption text-fg-subtle">
      {$t('settings.modelSpecs')
        .replace('{size}', formatGigabytes(model.size_bytes))
        .replace('{ram}', String(model.min_ram_gb))
        .replace('{context}', String(Math.round(model.context_window_tokens / 1024)))}
      · {model.parameters} · {model.license}
    </p>
  {:else}
    <p class="mt-1.5 text-caption text-fg-subtle">
      {$t('settings.size').replace('{size}', formatGigabytes(model.size_bytes))}
    </p>
  {/if}
  {#if disclaimer}
    <p class="mt-1 text-caption text-warning-fg">{disclaimer}</p>
  {/if}
  {#if !model.is_downloaded && !canRun}
    <p class="mt-1 text-caption text-warning-fg">
      {$t('settings.insufficientRam')
        .replace('{required}', String(model.min_ram_gb))
        .replace('{available}', ramLabel)}
    </p>
  {/if}
  {#if downloading}
    <ProgressBar
      class="mt-3"
      value={progress ?? 0}
      label={$t('settings.downloadingEllipsis')}
      showValue
    />
  {/if}
</SettingsRow>
