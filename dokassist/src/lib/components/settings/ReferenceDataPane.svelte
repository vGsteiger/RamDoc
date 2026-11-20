<script lang="ts">
  import { onDestroy, onMount } from 'svelte';
  import { listen, type UnlistenFn } from '@tauri-apps/api/event';
  import { downloadMedicationReference, getMedicationReferenceVersion } from '$lib/api';
  import {
    Alert,
    Button,
    PageShell,
    ProgressBar,
    SettingsRow,
    StatusIndicator,
  } from '$lib/components/ui';
  import SettingsGroup from './SettingsGroup.svelte';
  import { errorText } from '$lib/translations/labels';
  import { t } from '$lib/translations';

  /* Reference data: the Swissmedic AIPS medication database. */
  let medRefVersion = $state<string | null>(null);
  let medRefPhase = $state<'idle' | 'downloading' | 'done' | 'error'>('idle');
  let medRefProgress = $state(0);
  let medRefError = $state('');
  let medRefUnlisten: UnlistenFn | null = null;

  onMount(async () => {
    medRefVersion = await getMedicationReferenceVersion().catch(() => null);
  });

  onDestroy(() => {
    medRefUnlisten?.();
  });

  async function handleDownloadMedRef() {
    medRefPhase = 'downloading';
    medRefProgress = 0;
    medRefError = '';

    medRefUnlisten = await listen<number>('medication-ref-download-progress', (e) => {
      medRefProgress = Math.round(e.payload * 100);
    });

    try {
      await downloadMedicationReference();
      medRefVersion = await getMedicationReferenceVersion();
      medRefPhase = 'done';
    } catch (e) {
      medRefPhase = 'error';
      medRefError = $errorText(e);
    } finally {
      medRefUnlisten?.();
      medRefUnlisten = null;
    }
  }
</script>

<PageShell
  width="form"
  title={$t('settings.nav.referenceData')}
  description={$t('settings.panes.referenceDataDesc')}
>
  <SettingsGroup>
    <SettingsRow
      label={$t('settings.medicationReference')}
      description={$t('settings.medicationReferenceDesc')}
    >
      {#snippet control()}
        <Button
          onclick={handleDownloadMedRef}
          loading={medRefPhase === 'downloading'}
          variant={medRefVersion ? 'secondary' : 'primary'}
        >
          {#if medRefPhase === 'downloading'}
            {medRefProgress}%
          {:else if medRefVersion}
            {$t('settings.medRefUpdate')}
          {:else}
            {$t('settings.medRefDownload')}
          {/if}
        </Button>
      {/snippet}

      <div class="mt-2 space-y-1">
        {#if medRefVersion}
          <StatusIndicator
            tone="ok"
            label={$t('settings.medRefInstalled').replace('{version}', medRefVersion)}
          />
          <p class="text-caption text-fg-subtle">{$t('settings.medRefInstalledDesc')}</p>
        {:else}
          <StatusIndicator tone="idle" label={$t('settings.medRefNotInstalled')} />
          <p class="text-caption text-fg-subtle">{$t('settings.medRefNotInstalledDesc')}</p>
        {/if}
      </div>

      {#if medRefPhase === 'downloading'}
        <ProgressBar
          class="mt-3"
          value={medRefProgress}
          label={$t('settings.medRefDownloading')}
          showValue
        />
      {/if}
      {#if medRefPhase === 'error'}
        <Alert tone="danger" class="mt-3">{medRefError}</Alert>
      {/if}
    </SettingsRow>
  </SettingsGroup>
</PageShell>
