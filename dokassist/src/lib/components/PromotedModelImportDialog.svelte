<script lang="ts">
  import { errorText } from '$lib/translations/labels';
  import { get } from 'svelte/store';
  import { listen, type UnlistenFn } from '@tauri-apps/api/event';
  import { open as openFileDialog } from '@tauri-apps/plugin-dialog';
  import { Alert, Button, Dialog } from '$lib/components/ui';
  import { inspectPromotedModel, importPromotedModel, type PromotedModelPreview } from '$lib/api';
  import { t } from '$lib/translations';

  let {
    open = $bindable(false),
    onImported,
  }: {
    open?: boolean;
    onImported?: () => void | Promise<void>;
  } = $props();

  let promotionPath = $state<string | null>(null);
  let artifactPath = $state<string | null>(null);
  let preview = $state<PromotedModelPreview | null>(null);
  let error = $state('');
  let importing = $state(false);
  let progress = $state<number | null>(null);
  let success = $state('');
  let progressUnlisten: UnlistenFn | null = null;

  function reset() {
    promotionPath = null;
    artifactPath = null;
    preview = null;
    error = '';
    importing = false;
    progress = null;
    success = '';
    progressUnlisten?.();
    progressUnlisten = null;
  }

  function dismiss() {
    open = false;
    reset();
  }

  function formatBytes(bytes: number): string {
    if (bytes >= 1024 ** 3) return `${(bytes / 1024 ** 3).toFixed(1)} GB`;
    if (bytes >= 1024 ** 2) return `${(bytes / 1024 ** 2).toFixed(1)} MB`;
    return `${Math.max(1, Math.round(bytes / 1024))} KB`;
  }

  async function pickPath(kind: 'json' | 'gguf'): Promise<string | null> {
    const selected = await openFileDialog({
      title: get(t)(
        kind === 'json' ? 'settings.selectPromotionDialogTitle' : 'settings.selectGgufDialogTitle'
      ),
      filters: [
        kind === 'json'
          ? { name: get(t)('settings.promotionImportFilterJson'), extensions: ['json'] }
          : { name: get(t)('settings.promotionImportFilterGguf'), extensions: ['gguf'] },
      ],
      multiple: false,
    });
    if (!selected || Array.isArray(selected)) return null;
    return selected;
  }

  async function loadPreview(nextPromotion: string, nextArtifact: string | null) {
    error = '';
    success = '';
    preview = await inspectPromotedModel(nextPromotion, nextArtifact);
    promotionPath = nextPromotion;
    artifactPath = preview.artifact_path ?? nextArtifact;
  }

  async function handleChooseRecord() {
    try {
      const selected = await pickPath('json');
      if (!selected) return;
      artifactPath = null;
      await loadPreview(selected, null);
    } catch (e) {
      preview = null;
      promotionPath = null;
      error = $errorText(e);
    }
  }

  async function handleChooseGguf() {
    if (!promotionPath) return;
    try {
      const selected = await pickPath('gguf');
      if (!selected) return;
      await loadPreview(promotionPath, selected);
    } catch (e) {
      error = $errorText(e);
    }
  }

  let canImport = $derived(
    Boolean(
      preview?.artifact_found && preview?.artifact_size_matches && promotionPath && !success
    ) && !importing
  );

  async function handleImport() {
    if (!promotionPath || !preview) return;
    importing = true;
    error = '';
    success = '';
    progress = 0;
    try {
      progressUnlisten = await listen<number>('promoted-model-import-progress', (event) => {
        progress = Math.round(event.payload * 100);
      });
      const imported = await importPromotedModel(promotionPath, artifactPath);
      await onImported?.();
      success = get(t)('settings.promotionImportSuccess').replace('{name}', imported.name);
      progress = 100;
    } catch (e) {
      error = $errorText(e);
    } finally {
      progressUnlisten?.();
      progressUnlisten = null;
      importing = false;
    }
  }
</script>

<Dialog
  bind:open
  title={$t('settings.promotionImportDialogTitle')}
  description={$t('settings.promotionImportDialogDesc')}
  onClose={reset}
>
  <div class="space-y-3">
    <Alert tone="info">{$t('settings.promotionDisclaimer')}</Alert>

    {#if preview}
      <div class="rounded-card border border-line bg-surface-hover p-3">
        <p class="text-body font-medium text-fg">{preview.display_name}</p>
        <p class="text-caption text-fg-muted mt-1">
          {$t('settings.promotionQuantization').replace('{quantization}', preview.quantization)}
        </p>
        <p class="text-caption text-fg-muted">
          {$t('settings.promotionStudy').replace('{study}', preview.study_id)}
        </p>
        <p class="text-caption text-fg-muted">
          {$t('settings.promotionComparedWith').replace(
            '{baselines}',
            preview.baseline_artifacts.join(', ')
          )}
        </p>
        {#if preview.artifact_found && preview.artifact_size_matches}
          <p class="text-caption text-success-fg mt-2">
            {$t('settings.promotionArtifactFound')
              .replace('{filename}', preview.filename)
              .replace('{size}', formatBytes(preview.size_bytes))}
          </p>
        {:else if preview.artifact_found}
          <p class="text-caption text-warning-fg mt-2">
            {$t('settings.promotionArtifactSizeMismatch')
              .replace('{actual}', formatBytes(preview.artifact_bytes ?? 0))
              .replace('{expected}', formatBytes(preview.size_bytes))}
          </p>
        {:else}
          <p class="text-caption text-warning-fg mt-2">
            {$t('settings.promotionArtifactMissing').replace('{filename}', preview.filename)}
          </p>
        {/if}
      </div>
      <div class="flex flex-wrap gap-2">
        <Button size="sm" onclick={handleChooseRecord} disabled={importing}>
          {$t('settings.promotionChangeRecord')}
        </Button>
        {#if !preview.artifact_found || !preview.artifact_size_matches}
          <Button size="sm" variant="primary" onclick={handleChooseGguf} disabled={importing}>
            {$t('settings.promotionChooseGguf')}
          </Button>
        {/if}
      </div>
    {:else}
      <Button onclick={handleChooseRecord} disabled={importing}>
        {$t('settings.promotionChooseRecord')}
      </Button>
    {/if}

    {#if importing || (progress !== null && !success)}
      <div>
        <div class="flex justify-between text-caption text-fg-muted mb-1">
          <span>
            {$t('settings.promotionImportProgress').replace('{percent}', String(progress ?? 0))}
          </span>
        </div>
        <div
          class="w-full bg-surface-selected rounded-full h-2"
          role="progressbar"
          aria-valuemin={0}
          aria-valuemax={100}
          aria-valuenow={progress ?? 0}
        >
          <div class="bg-accent h-2 rounded-full" style="width: {progress ?? 0}%"></div>
        </div>
      </div>
    {/if}

    {#if error}
      <Alert tone="danger">{error}</Alert>
    {/if}
    {#if success}
      <Alert tone="success">{success}</Alert>
    {/if}
  </div>

  {#snippet footer()}
    <Button onclick={dismiss}>{$t('common.cancel')}</Button>
    <Button variant="primary" loading={importing} disabled={!canImport} onclick={handleImport}>
      {$t('settings.promotionImportConfirm')}
    </Button>
  {/snippet}
</Dialog>
