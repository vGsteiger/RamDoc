<script lang="ts">
  /** Settings → Dictation: download, choose and delete local speech models. */
  import { onDestroy, onMount } from 'svelte';
  import { listen, type UnlistenFn } from '@tauri-apps/api/event';
  import {
    deleteDictationModel,
    downloadDictationModel,
    getDictationStatus,
    setActiveDictationModel,
    DICTATION_EVENTS,
    type DictationModel,
    type DictationStatus,
  } from '$lib/api';
  import {
    Alert,
    Badge,
    Button,
    ConfirmDialog,
    ProgressBar,
    SettingsRow,
  } from '$lib/components/ui';
  import SettingsGroup from '$lib/components/settings/SettingsGroup.svelte';
  import { errorText } from '$lib/translations/labels';
  import { t } from '$lib/translations';

  let status = $state<DictationStatus | null>(null);
  let downloadingId = $state<string | null>(null);
  let progress = $state(0);
  let error = $state('');
  let unlisten: UnlistenFn | null = null;
  let pendingDelete = $state<DictationModel | null>(null);
  let confirmOpen = $state(false);

  function formatSize(bytes: number): string {
    return `${(bytes / 1024 / 1024).toFixed(0)} MB`;
  }

  async function refresh() {
    try {
      status = await getDictationStatus();
    } catch (e) {
      error = $errorText(e);
    }
  }

  async function download(id: string) {
    downloadingId = id;
    progress = 0;
    error = '';
    try {
      await downloadDictationModel(id);
    } catch (e) {
      error = $errorText(e);
    } finally {
      downloadingId = null;
      await refresh();
    }
  }

  async function use(id: string) {
    error = '';
    try {
      await setActiveDictationModel(id);
    } catch (e) {
      error = $errorText(e);
    }
    await refresh();
  }

  function askRemove(model: DictationModel) {
    pendingDelete = model;
    confirmOpen = true;
  }

  async function remove(id: string) {
    error = '';
    try {
      await deleteDictationModel(id);
    } catch (e) {
      error = $errorText(e);
    }
    await refresh();
  }

  onMount(async () => {
    unlisten = await listen<number>(DICTATION_EVENTS.downloadProgress, (e) => {
      progress = Math.round(e.payload * 100);
    });
    await refresh();
  });

  onDestroy(() => unlisten?.());
</script>

{#if status && !status.worker_available}
  <Alert tone="warning" class="mb-section">{$t('dictation.workerMissingSettings')}</Alert>
{/if}

<SettingsGroup title={$t('dictation.models')}>
  {#each status?.models ?? [] as model (model.id)}
    <SettingsRow label={model.name} description={model.description}>
      {#snippet control()}
        {#if !model.installed}
          <Button
            onclick={() => download(model.id)}
            disabled={downloadingId !== null}
            loading={downloadingId === model.id}
          >
            {downloadingId === model.id ? $t('dictation.downloading') : $t('dictation.download')}
          </Button>
        {:else}
          {#if !model.active}
            <Button onclick={() => use(model.id)}>{$t('dictation.use')}</Button>
          {/if}
          <Button variant="danger" onclick={() => askRemove(model)}>
            {$t('dictation.delete')}
          </Button>
        {/if}
      {/snippet}

      <div class="mt-1.5 flex flex-wrap items-center gap-1.5">
        {#if model.active}
          <Badge tone="success">{$t('dictation.activeBadge')}</Badge>
        {:else if model.installed}
          <Badge>{$t('dictation.installedBadge')}</Badge>
        {/if}
        {#if model.recommended}
          <Badge tone="info">{$t('dictation.recommendedBadge')}</Badge>
        {/if}
        <span class="text-caption text-fg-subtle">
          {model.languages} · {$t('dictation.modelSpecs')
            .replace('{size}', formatSize(model.size_bytes))
            .replace('{license}', model.license)}
        </span>
      </div>

      {#if downloadingId === model.id}
        <ProgressBar class="mt-3" value={progress} label={$t('dictation.downloading')} showValue />
      {/if}
    </SettingsRow>
  {/each}
</SettingsGroup>

{#if error}
  <Alert tone="danger">{error}</Alert>
{/if}

<ConfirmDialog
  bind:open={confirmOpen}
  tone="danger"
  title={$t('dictation.deleteTitle').replace('{name}', pendingDelete?.name ?? '')}
  message={$t('dictation.confirmDelete')}
  confirmLabel={$t('dictation.delete')}
  onConfirm={() => (pendingDelete ? remove(pendingDelete.id) : undefined)}
  onCancel={() => (pendingDelete = null)}
/>
