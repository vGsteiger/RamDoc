<script lang="ts">
  import { onMount } from 'svelte';
  import { getEmbedStatus, initializeEmbedEngine, type EmbedStatus } from '$lib/api';
  import { Alert, Button, SettingsRow, StatusIndicator } from '$lib/components/ui';
  import { errorText } from '$lib/translations/labels';
  import { t } from '$lib/translations';

  /* The embedding model behind literature search: status and "Load now". */
  let embedStatus = $state<EmbedStatus | null>(null);
  let embedPhase = $state<'idle' | 'loading' | 'done' | 'error'>('idle');
  let embedError = $state('');

  onMount(async () => {
    try {
      embedStatus = await getEmbedStatus();
      if (embedStatus.is_loaded) embedPhase = 'done';
    } catch (e) {
      embedError = $errorText(e);
    }
  });

  async function handleInitEmbed() {
    embedPhase = 'loading';
    embedError = '';
    try {
      await initializeEmbedEngine();
      embedStatus = await getEmbedStatus();
      embedPhase = 'done';
    } catch (e) {
      embedPhase = 'error';
      embedError = $errorText(e);
    }
  }
</script>

<SettingsRow label={$t('settings.embeddingModel')} description={$t('settings.embeddingModelDesc')}>
  {#snippet control()}
    {#if embedPhase !== 'done'}
      <Button onclick={handleInitEmbed} loading={embedPhase === 'loading'}>
        {embedPhase === 'loading' ? $t('common.loading') : $t('settings.loadNow')}
      </Button>
    {/if}
  {/snippet}

  <div class="mt-1.5">
    {#if embedPhase === 'loading'}
      <StatusIndicator tone="busy" label={$t('settings.embeddingInitializing')} live />
    {:else if embedStatus?.is_loaded}
      <StatusIndicator
        tone="ok"
        label={$t('settings.embeddingLoaded')}
        detail="nomic-embed-text-v1.5"
      />
    {:else if embedStatus?.is_downloaded}
      <StatusIndicator tone="warn" label={$t('settings.embeddingCached')} />
      <p class="mt-0.5 text-caption text-fg-subtle">{$t('settings.embeddingCachedDesc')}</p>
    {:else if embedStatus}
      <StatusIndicator tone="idle" label={$t('settings.embeddingNotDownloaded')} />
      <p class="mt-0.5 text-caption text-fg-subtle">{$t('settings.embeddingNotDownloadedDesc')}</p>
    {/if}
  </div>
  {#if embedError}
    <Alert tone="danger" class="mt-3">{embedError}</Alert>
  {/if}
</SettingsRow>
