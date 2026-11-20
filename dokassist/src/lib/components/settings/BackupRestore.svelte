<script lang="ts">
  import { goto } from '$app/navigation';
  import { resolve } from '$app/paths';
  import {
    createVaultBackup,
    restoreVaultBackup,
    validateBackupArchive,
    type BackupInfo,
  } from '$lib/api';
  import { Alert, Button, SettingsRow } from '$lib/components/ui';
  import TypedConfirmDialog from './TypedConfirmDialog.svelte';
  import { fmt } from '$lib/format';
  import { errorText } from '$lib/translations/labels';
  import { t } from '$lib/translations';

  /*
   * Encrypted vault backup and restore. Restoring replaces every record, so
   * it keeps its typed RESTORE confirmation (now in a dialog).
   */
  let creatingBackup = $state(false);
  let backupError = $state('');

  let fileInput = $state<HTMLInputElement | null>(null);
  let selectedBackupFile = $state<File | null>(null);
  let validatedBackupInfo = $state<BackupInfo | null>(null);
  let showRestoreConfirm = $state(false);
  let restoring = $state(false);
  let restoreError = $state('');

  async function handleCreateBackup() {
    creatingBackup = true;
    backupError = '';
    try {
      const backupData = await createVaultBackup();
      const blob = new Blob([new Uint8Array(backupData)], { type: 'application/octet-stream' });
      const url = URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = url;
      a.download = `RamDoc_Backup_${new Date().toISOString().split('T')[0]}.dokassist`;
      document.body.appendChild(a);
      a.click();
      document.body.removeChild(a);
      URL.revokeObjectURL(url);
    } catch (e) {
      backupError = $errorText(e);
    } finally {
      creatingBackup = false;
    }
  }

  async function handleSelectRestoreFile(event: Event & { currentTarget: HTMLInputElement }) {
    const file = event.currentTarget.files?.[0];
    if (!file) {
      selectedBackupFile = null;
      validatedBackupInfo = null;
      return;
    }

    selectedBackupFile = file;
    restoreError = '';

    try {
      const arrayBuffer = await file.arrayBuffer();
      const backupArray = Array.from(new Uint8Array(arrayBuffer));
      validatedBackupInfo = await validateBackupArchive(backupArray);
    } catch (e) {
      restoreError = $errorText(e);
      selectedBackupFile = null;
      validatedBackupInfo = null;
    }
  }

  async function handleRestoreBackup() {
    if (!selectedBackupFile || !validatedBackupInfo) return;

    restoring = true;
    restoreError = '';
    try {
      const arrayBuffer = await selectedBackupFile.arrayBuffer();
      const backupArray = Array.from(new Uint8Array(arrayBuffer));
      await restoreVaultBackup(backupArray);

      showRestoreConfirm = false;
      selectedBackupFile = null;
      validatedBackupInfo = null;

      // The database was replaced: go back through the unlock gate.
      goto(resolve('/'));
    } catch (e) {
      restoreError = $errorText(e);
    } finally {
      restoring = false;
    }
  }
</script>

<SettingsRow
  label={$t('settings.backup.createTitle')}
  description={$t('settings.backup.createDesc')}
>
  {#snippet control()}
    <Button onclick={handleCreateBackup} loading={creatingBackup}>
      {creatingBackup ? $t('settings.backup.creating') : $t('settings.backup.exportBackup')}
    </Button>
  {/snippet}
  {#if backupError}
    <Alert tone="danger" class="mt-3">{backupError}</Alert>
  {/if}
</SettingsRow>

<SettingsRow
  label={$t('settings.backup.restoreTitle')}
  description={$t('settings.backup.restoreDesc')}
>
  {#snippet control()}
    <!-- Native file picker kept for the File object it yields; the visible
         control is the Button below. -->
    <input
      bind:this={fileInput}
      id="restore-backup-file"
      type="file"
      accept=".dokassist"
      class="sr-only"
      tabindex="-1"
      aria-label={$t('settings.backup.selectFile')}
      onchange={handleSelectRestoreFile}
    />
    <Button onclick={() => fileInput?.click()} disabled={restoring}>
      {$t('settings.backup.chooseFile')}
    </Button>
  {/snippet}

  <p class="mt-2 truncate text-caption text-fg-subtle">
    {selectedBackupFile ? selectedBackupFile.name : $t('settings.backup.noFile')}
  </p>

  {#if validatedBackupInfo}
    <div class="mt-3 rounded-control border border-line-subtle bg-surface p-3">
      <p class="text-label font-medium text-success-fg">{$t('settings.backup.validated')}</p>
      <div class="mt-1 space-y-0.5 text-caption text-fg-muted">
        <p>
          {$t('settings.backup.createdAt').replace(
            '{date}',
            $fmt.dateTime(validatedBackupInfo.created_at)
          )}
        </p>
        <p>
          {$t('settings.backup.fileCount').replace(
            '{count}',
            String(validatedBackupInfo.file_count)
          )}
        </p>
        <p>
          {$t('settings.backup.dbSchema').replace(
            '{version}',
            String(validatedBackupInfo.db_schema_version)
          )}
        </p>
      </div>
      <Button
        variant="danger"
        class="mt-3"
        onclick={() => {
          restoreError = '';
          showRestoreConfirm = true;
        }}
      >
        {$t('settings.backup.restoreFromThis')}
      </Button>
    </div>
  {/if}

  {#if restoreError && !showRestoreConfirm}
    <Alert tone="danger" class="mt-3">{restoreError}</Alert>
  {/if}
</SettingsRow>

<TypedConfirmDialog
  bind:open={showRestoreConfirm}
  title={$t('settings.backup.restoreTitle')}
  word="RESTORE"
  confirmLabel={$t('settings.backup.confirmRestore')}
  busyLabel={$t('settings.backup.restoring')}
  busy={restoring}
  error={restoreError}
  onConfirm={handleRestoreBackup}
>
  <strong class="font-semibold">{$t('settings.backup.warningLabel')}</strong>
  {$t('settings.backup.warningBody')}
  <strong class="font-semibold">RESTORE</strong>
  {$t('settings.backup.warningConfirm')}
</TypedConfirmDialog>
