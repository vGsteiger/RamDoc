<script lang="ts">
  import { goto } from '$app/navigation';
  import { resolve } from '$app/paths';
  import { exportAllPatientData, resetApp } from '$lib/api';
  import { Button, PageShell, SettingsRow } from '$lib/components/ui';
  import SettingsGroup from './SettingsGroup.svelte';
  import TypedConfirmDialog from './TypedConfirmDialog.svelte';
  import { errorText } from '$lib/translations/labels';
  import { t } from '$lib/translations';

  /*
   * Emergency export and factory reset. Both keep their typed confirmation
   * words (EXPORT, RESET — translated keys, compared exactly) and now ask for
   * them in a dialog instead of an inline form.
   */
  let showExportConfirm = $state(false);
  let exporting = $state(false);
  let exportError = $state('');

  let showResetConfirm = $state(false);
  let resetting = $state(false);
  let resetError = $state('');

  async function handleExport() {
    exporting = true;
    exportError = '';
    try {
      const zipData = await exportAllPatientData();
      const blob = new Blob([new Uint8Array(zipData)], { type: 'application/zip' });
      const url = URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = url;
      a.download = `RamDoc_Export_${new Date().toISOString().split('T')[0]}.zip`;
      document.body.appendChild(a);
      a.click();
      document.body.removeChild(a);
      URL.revokeObjectURL(url);
      showExportConfirm = false;
    } catch (e) {
      exportError = $errorText(e);
    } finally {
      exporting = false;
    }
  }

  async function handleReset() {
    resetting = true;
    resetError = '';
    try {
      await resetApp();
      goto(resolve('/'));
    } catch (e) {
      resetError = $errorText(e);
      resetting = false;
    }
  }
</script>

<PageShell
  width="full"
  title={$t('settings.nav.dangerZone')}
  description={$t('settings.panes.dangerZoneDesc')}
>
  <SettingsGroup>
    <SettingsRow
      label={$t('settings.emergencyExport')}
      description={$t('settings.emergencyExportDesc')}
    >
      {#snippet control()}
        <Button
          onclick={() => {
            exportError = '';
            showExportConfirm = true;
          }}
        >
          {$t('settings.exportData')}
        </Button>
      {/snippet}
    </SettingsRow>
    <SettingsRow
      label={$t('settings.factoryReset')}
      description={$t('settings.factoryResetShortDesc')}
    >
      {#snippet control()}
        <Button
          variant="danger"
          onclick={() => {
            resetError = '';
            showResetConfirm = true;
          }}
        >
          {$t('settings.factoryReset')}
        </Button>
      {/snippet}
    </SettingsRow>
  </SettingsGroup>
</PageShell>

<TypedConfirmDialog
  bind:open={showExportConfirm}
  title={$t('settings.emergencyExport')}
  word={$t('settings.exportConfirmWord')}
  confirmLabel={$t('settings.confirmExport')}
  busyLabel={$t('settings.exporting')}
  tone="default"
  busy={exporting}
  error={exportError}
  onConfirm={handleExport}
>
  {$t('settings.exportConfirmHint')}
</TypedConfirmDialog>

<TypedConfirmDialog
  bind:open={showResetConfirm}
  title={$t('settings.factoryReset')}
  word={$t('settings.resetConfirmWord')}
  confirmLabel={$t('settings.confirmResetAction')}
  busyLabel={$t('settings.resetting')}
  busy={resetting}
  error={resetError}
  onConfirm={handleReset}
>
  <p>{$t('settings.factoryResetShortDesc')}</p>
  <p class="mt-2 text-danger-fg">{$t('settings.resetConfirmHint')}</p>
</TypedConfirmDialog>
