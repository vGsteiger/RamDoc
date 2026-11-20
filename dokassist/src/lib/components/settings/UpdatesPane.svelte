<script lang="ts">
  import { onDestroy, onMount } from 'svelte';
  import { listen, type UnlistenFn } from '@tauri-apps/api/event';
  import { getVersion } from '@tauri-apps/api/app';
  import { checkForUpdates, installUpdate, type UpdateInfo } from '$lib/api';
  import {
    Alert,
    Button,
    Markdown,
    PageShell,
    ProgressBar,
    SettingsRow,
    StatusIndicator,
  } from '$lib/components/ui';
  import SettingsGroup from './SettingsGroup.svelte';
  import { errorText } from '$lib/translations/labels';
  import { t } from '$lib/translations';

  /*
   * Updates & about. The installed version is shown exactly once, in the row
   * that also checks for a newer one.
   *
   * A failed check clears any earlier result: a stale "You're up to date"
   * must never sit next to (or instead of) the error. The backend reports
   * unreachable or half-published update metadata as UPDATE_INFO_UNAVAILABLE,
   * which errorText turns into a readable message.
   */
  let appVersion = $state<string | null>(null);
  let updateInfo = $state<UpdateInfo | null>(null);
  let checkingUpdate = $state(false);
  let installingUpdate = $state(false);
  let updateProgress = $state(0);
  let updateError = $state('');
  let updateUnlisten: UnlistenFn | null = null;

  onMount(async () => {
    appVersion = await getVersion().catch(() => $t('common.unknown'));
  });

  onDestroy(() => {
    updateUnlisten?.();
  });

  async function handleCheckForUpdates() {
    checkingUpdate = true;
    updateError = '';
    try {
      updateInfo = await checkForUpdates();
    } catch (e) {
      updateInfo = null;
      updateError = $errorText(e);
    } finally {
      checkingUpdate = false;
    }
  }

  async function handleInstallUpdate() {
    if (!updateInfo?.update_available) return;

    installingUpdate = true;
    updateError = '';
    updateProgress = 0;

    updateUnlisten = await listen<number>('updater-download-progress', (e) => {
      updateProgress = Math.round(e.payload * 100);
    });

    const completeUnsub = await listen('updater-download-complete', () => {
      completeUnsub();
    });

    try {
      await installUpdate();
      // After a successful install the app restarts on its own.
    } catch (e) {
      updateUnlisten?.();
      updateUnlisten = null;
      installingUpdate = false;
      updateError = $errorText(e);
    }
  }
</script>

<PageShell
  width="full"
  title={$t('settings.nav.updates')}
  description={$t('settings.panes.updatesDesc')}
>
  <SettingsGroup>
    <SettingsRow
      label={$t('settings.currentVersion')}
      description={appVersion ?? $t('common.loading')}
    >
      {#snippet control()}
        <Button
          onclick={handleCheckForUpdates}
          loading={checkingUpdate}
          disabled={installingUpdate}
          data-testid="check-for-updates"
        >
          {checkingUpdate ? $t('settings.checking') : $t('settings.checkForUpdates')}
        </Button>
      {/snippet}

      {#if updateError && !updateInfo}
        <Alert tone="danger" title={$t('settings.updateError')} class="mt-3">{updateError}</Alert>
      {:else if updateInfo && !updateInfo.update_available}
        <div class="mt-2">
          <StatusIndicator tone="ok" label={$t('settings.updates.upToDate')} live />
        </div>
      {/if}
    </SettingsRow>

    {#if updateInfo?.update_available}
      <SettingsRow
        label={$t('settings.updates.newVersion').replace(
          '{version}',
          updateInfo.latest_version ?? ''
        )}
        description={installingUpdate ? $t('settings.autoRestart') : undefined}
      >
        {#snippet control()}
          {#if !installingUpdate}
            <Button variant="primary" onclick={handleInstallUpdate}>
              {$t('settings.installUpdate')}
            </Button>
          {/if}
        {/snippet}

        {#if installingUpdate}
          <ProgressBar
            class="mt-3"
            value={updateProgress}
            label={$t('settings.downloadingAndInstalling')}
            showValue
          />
        {/if}
        {#if updateError}
          <Alert tone="danger" class="mt-3">{updateError}</Alert>
        {/if}
        {#if updateInfo.body}
          <div class="mt-3 rounded-control border border-line-subtle bg-surface p-3">
            <p class="mb-1 text-label font-medium text-fg-muted">
              {$t('settings.updates.releaseNotes')}
            </p>
            <div class="max-h-64 overflow-y-auto text-label">
              <Markdown source={updateInfo.body} />
            </div>
          </div>
        {/if}
      </SettingsRow>
    {/if}
  </SettingsGroup>
</PageShell>
