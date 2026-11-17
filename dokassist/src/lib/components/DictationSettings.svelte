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
    type DictationStatus,
  } from '$lib/api';
  import { errorText } from '$lib/translations/labels';
  import { t } from '$lib/translations';

  let status = $state<DictationStatus | null>(null);
  let downloadingId = $state<string | null>(null);
  let progress = $state(0);
  let error = $state('');
  let unlisten: UnlistenFn | null = null;

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

  async function remove(id: string) {
    if (!confirm($t('dictation.confirmDelete'))) return;
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

<section class="mt-10">
  <h2 class="text-heading font-semibold text-fg mb-4">{$t('dictation.section')}</h2>
  <p class="text-caption text-fg-muted mb-4">{$t('dictation.sectionDesc')}</p>

  {#if status && !status.worker_available}
    <p class="text-caption text-warning-fg mb-4">{$t('dictation.workerMissingSettings')}</p>
  {/if}

  <h3 class="text-heading font-semibold text-fg mb-3">{$t('dictation.models')}</h3>
  <div class="space-y-3">
    {#each status?.models ?? [] as model (model.id)}
      <div class="bg-surface-hover border border-line rounded-card p-4">
        <div class="flex items-start justify-between gap-4">
          <div class="flex-1">
            <div class="flex flex-wrap items-center gap-2 mb-1">
              <p class="text-body font-medium text-fg">{model.name}</p>
              {#if model.recommended}
                <span class="px-2 py-0.5 text-caption rounded-card bg-accent text-on-accent">
                  {$t('dictation.recommendedBadge')}
                </span>
              {/if}
              {#if model.active}
                <span class="px-2 py-0.5 text-caption rounded-card bg-success text-on-success">
                  {$t('dictation.activeBadge')}
                </span>
              {:else if model.installed}
                <span class="px-2 py-0.5 text-caption rounded-card bg-surface-selected text-fg">
                  {$t('dictation.installedBadge')}
                </span>
              {/if}
            </div>
            <p class="text-caption text-fg-muted mb-1">{model.description}</p>
            <p class="text-caption text-fg-subtle">{model.languages}</p>
            <p class="text-caption text-fg-subtle mt-1">
              {$t('dictation.modelSpecs')
                .replace('{size}', formatSize(model.size_bytes))
                .replace('{license}', model.license)}
            </p>
          </div>

          <div class="flex shrink-0 items-center gap-2">
            {#if !model.installed}
              <button
                onclick={() => download(model.id)}
                disabled={downloadingId !== null}
                class="h-8 px-3 text-body rounded-control bg-accent hover:bg-accent-hover disabled:opacity-50 disabled:cursor-not-allowed text-on-accent transition-colors"
              >
                {downloadingId === model.id
                  ? $t('dictation.downloading')
                  : $t('dictation.download')}
              </button>
            {:else}
              {#if !model.active}
                <button
                  onclick={() => use(model.id)}
                  class="h-8 px-3 text-body rounded-control bg-accent hover:bg-accent-hover text-on-accent transition-colors"
                >
                  {$t('dictation.use')}
                </button>
              {/if}
              <button
                onclick={() => remove(model.id)}
                class="h-8 px-3 text-body rounded-control border border-line text-fg-muted hover:bg-danger-subtle hover:text-danger-fg transition-colors"
              >
                {$t('dictation.delete')}
              </button>
            {/if}
          </div>
        </div>

        {#if downloadingId === model.id}
          <div class="mt-3">
            <div class="flex justify-between text-caption text-fg-muted mb-1">
              <span>{$t('dictation.downloading')}</span>
              <span>{progress}%</span>
            </div>
            <div class="w-full bg-surface-selected rounded-full h-2">
              <div class="bg-accent h-2 rounded-full" style="width: {progress}%"></div>
            </div>
          </div>
        {/if}
      </div>
    {/each}
  </div>

  {#if error}
    <p class="text-caption text-danger-fg mt-3">{error}</p>
  {/if}
</section>
