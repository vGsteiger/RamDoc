<script lang="ts">
  import { get } from 'svelte/store';
  import { onDestroy, onMount } from 'svelte';
  import { listen, type UnlistenFn } from '@tauri-apps/api/event';
  import {
    deleteModel,
    downloadAndRegisterModel,
    getEngineStatus,
    getRecommendedModel,
    listAvailableModels,
    listModels,
    setDefaultModel,
    type AvailableModel,
    type InferenceProfile,
    type LlmEngineStatus,
    type ModelChoice,
    type ModelInfo,
  } from '$lib/api';
  import {
    Alert,
    Button,
    ConfirmDialog,
    ListSkeleton,
    PageShell,
    SettingsRow,
    StatusIndicator,
    ThinkingIndicator,
  } from '$lib/components/ui';
  import PromotedModelImportDialog from '$lib/components/PromotedModelImportDialog.svelte';
  import SettingsGroup from './SettingsGroup.svelte';
  import AvailableModelRow from './models/AvailableModelRow.svelte';
  import EmbeddingRow from './models/EmbeddingRow.svelte';
  import InstalledModelRow from './models/InstalledModelRow.svelte';
  import ModelDiagnostics from './models/ModelDiagnostics.svelte';
  import ReportSampling from './models/ReportSampling.svelte';
  import RouterSettings from './models/RouterSettings.svelte';
  import { formatGigabytes } from './models/format';
  import { engine, loadEngineModel, loadingModelLabel } from '$lib/stores/engine';
  import { refreshRouterDiagnostics } from '$lib/stores/router';
  import { errorText } from '$lib/translations/labels';
  import { t } from '$lib/translations';

  let status = $state<LlmEngineStatus | null>(null);
  let recommended = $state<ModelChoice | null>(null);
  let downloadProgress = $state<number | null>(null);
  let phase = $state<'idle' | 'downloading' | 'loading' | 'done' | 'error'>('idle');
  let errorMsg = $state('');
  let activeDownloadFilename = $state<string | null>(null);
  let unlisten: UnlistenFn | null = null;
  let doneUnsubscribe: UnlistenFn | null = null;

  let installedModels = $state<ModelInfo[]>([]);
  let availableModels = $state<AvailableModel[]>([]);
  let modelManagementError = $state('');
  let loadingModels = $state(true);
  let importDialogOpen = $state(false);
  let inferenceProfile = $state<InferenceProfile>('conservative');

  let pendingDelete = $state<ModelInfo | null>(null);
  let deleteOpen = $state(false);

  let busy = $derived(phase === 'downloading' || phase === 'loading' || $engine.isLoading);
  let ramLabel = $derived(status?.total_ram_bytes ? formatGigabytes(status.total_ram_bytes) : '…');
  // The recommended model normally appears in the catalogue; only list it
  // separately when it does not.
  let recommendedOutsideCatalogue = $derived(
    recommended && !availableModels.some((m) => m.filename === recommended?.filename)
      ? recommended
      : null
  );

  onMount(async () => {
    [status, recommended] = await Promise.all([getEngineStatus(), getRecommendedModel()]);
    if (status.inference_config) inferenceProfile = status.inference_config.profile;
    if (status.is_loaded) phase = 'done';
    await loadInstalledModels();
    await refreshRouterDiagnostics();
  });

  onDestroy(() => {
    unlisten?.();
    doneUnsubscribe?.();
  });

  async function loadInstalledModels() {
    try {
      loadingModels = true;
      const [installed, available] = await Promise.allSettled([
        listModels(),
        listAvailableModels(),
      ]);
      if (installed.status === 'fulfilled') installedModels = installed.value;
      else console.error('Failed to load installed models:', installed.reason);
      if (available.status === 'fulfilled') availableModels = available.value;
      else console.error('Failed to load available models:', available.reason);
    } catch (e) {
      modelManagementError = $errorText(e);
    } finally {
      loadingModels = false;
    }
  }

  async function handleDownloadNewModel(model: ModelChoice) {
    phase = 'downloading';
    downloadProgress = 0;
    errorMsg = '';
    activeDownloadFilename = model.filename;

    unlisten = await listen<number>('model-download-progress', (e) => {
      downloadProgress = Math.round(e.payload * 100);
    });
    doneUnsubscribe = await listen('model-download-done', () => {
      // Cleanup happens in finally.
    });

    try {
      await downloadAndRegisterModel(model);
      phase = 'idle';
      await loadInstalledModels();
    } catch (e) {
      phase = 'error';
      errorMsg = $errorText(e);
    } finally {
      unlisten?.();
      unlisten = null;
      doneUnsubscribe?.();
      doneUnsubscribe = null;
      activeDownloadFilename = null;
    }
  }

  async function handleLoadModel(model: ModelInfo) {
    phase = 'loading';
    errorMsg = '';
    try {
      await loadEngineModel(model.filename, inferenceProfile);
      status = get(engine).status;
      await loadInstalledModels();
      phase = 'done';
    } catch (e) {
      phase = 'error';
      errorMsg = $errorText(e);
    }
  }

  async function handleDeleteModel(model: ModelInfo) {
    try {
      await deleteModel(model.id);
      await loadInstalledModels();
    } catch (e) {
      modelManagementError = $errorText(e);
    }
  }

  async function handleSetDefaultModel(model: ModelInfo) {
    try {
      await setDefaultModel(model.id);
      await loadInstalledModels();
    } catch (e) {
      modelManagementError = $errorText(e);
    }
  }
</script>

<PageShell
  width="full"
  title={$t('settings.nav.models')}
  description={$t('settings.panes.modelsDesc')}
>
  <SettingsGroup
    title={$t('settings.models.writingStatus')}
    description={$t('settings.models.writingStatusDesc')}
  >
    {#snippet actions()}
      <Button size="sm" onclick={() => (importDialogOpen = true)}>
        {$t('settings.importPromotedModel')}
      </Button>
    {/snippet}

    <SettingsRow label={$t('settings.sharedRuntime')}>
      <div class="mt-1.5">
        {#if $engine.isLoading && $engine.loadingStartedAt}
          <ThinkingIndicator
            startedAt={$engine.loadingStartedAt}
            label={$t('settings.loadingModelIntoMemory').replace(
              '{name}',
              loadingModelLabel($engine.loadingFilename)
            )}
          />
          <p class="mt-1 text-caption text-fg-subtle">{$t('settings.loadingModelHint')}</p>
        {:else if status?.is_loaded}
          <StatusIndicator
            tone="ok"
            label={$t('settings.loadedModel').replace('{name}', status.model_name ?? '')}
          />
        {:else}
          <StatusIndicator tone="idle" label={$t('settings.noModelLoaded')} />
        {/if}
        {#if status?.total_ram_bytes}
          <p class="mt-1 text-caption text-fg-subtle">
            {$t('settings.systemRam').replace('{ram}', ramLabel)}
          </p>
        {/if}
      </div>
    </SettingsRow>

    {#if loadingModels}
      <ListSkeleton count={2} class="py-3" />
    {:else if installedModels.length === 0}
      <p class="py-3 text-body text-fg-muted">{$t('settings.noModelsInstalled')}</p>
    {:else}
      {#each installedModels as model (model.id)}
        <InstalledModelRow
          {model}
          {busy}
          onLoad={handleLoadModel}
          onSetDefault={handleSetDefaultModel}
          onDelete={(m) => {
            pendingDelete = m;
            deleteOpen = true;
          }}
        />
      {/each}
    {/if}

    <ReportSampling />
  </SettingsGroup>

  {#if phase === 'error' && errorMsg}
    <Alert tone="danger" class="mb-section">{errorMsg}</Alert>
  {/if}
  {#if modelManagementError}
    <Alert tone="danger" class="mb-section">{modelManagementError}</Alert>
  {/if}

  <SettingsGroup
    title={$t('settings.availableModels')}
    description={$t('settings.availableModelsHint').replace('{ram}', ramLabel)}
  >
    {#if loadingModels}
      <ListSkeleton count={3} class="py-3" />
    {:else}
      {#if recommendedOutsideCatalogue}
        <AvailableModelRow
          model={{
            name: recommendedOutsideCatalogue.name,
            filename: recommendedOutsideCatalogue.filename,
            size_bytes: recommendedOutsideCatalogue.size_bytes,
            description: recommendedOutsideCatalogue.reason,
            min_ram_gb: 0,
            context_window_tokens: 0,
            parameters: '',
            license: '',
            disclaimer: null,
            is_downloaded: false,
            model_id: null,
          }}
          totalRamBytes={status?.total_ram_bytes}
          recommended
          downloading={activeDownloadFilename === recommendedOutsideCatalogue.filename}
          progress={downloadProgress}
          disabled={busy}
          onDownload={handleDownloadNewModel}
        />
      {/if}
      {#each availableModels as model (model.filename)}
        <AvailableModelRow
          {model}
          totalRamBytes={status?.total_ram_bytes}
          recommended={recommended?.filename === model.filename}
          downloading={activeDownloadFilename === model.filename}
          progress={downloadProgress}
          disabled={busy}
          onDownload={handleDownloadNewModel}
        />
      {/each}
    {/if}
  </SettingsGroup>

  <SettingsGroup title={$t('settings.models.routing')}>
    <RouterSettings {installedModels} />
  </SettingsGroup>

  <SettingsGroup title={$t('settings.models.retrieval')}>
    <EmbeddingRow />
  </SettingsGroup>

  <ModelDiagnostics
    {status}
    bind:inferenceProfile
    disabled={phase === 'loading' || $engine.isLoading}
  />
</PageShell>

<PromotedModelImportDialog bind:open={importDialogOpen} onImported={loadInstalledModels} />

<ConfirmDialog
  bind:open={deleteOpen}
  tone="danger"
  title={$t('settings.models.deleteTitle').replace('{name}', pendingDelete?.name ?? '')}
  message={$t('settings.models.deleteMessage')}
  confirmLabel={$t('settings.delete')}
  onConfirm={() => (pendingDelete ? handleDeleteModel(pendingDelete) : undefined)}
  onCancel={() => (pendingDelete = null)}
/>
