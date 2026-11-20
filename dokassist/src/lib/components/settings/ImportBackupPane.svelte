<script lang="ts">
  import { PageShell } from '$lib/components/ui';
  import BackupRestore from './BackupRestore.svelte';
  import CsvImport from './CsvImport.svelte';
  import SettingsGroup from './SettingsGroup.svelte';
  import { t } from '$lib/translations';
</script>

<PageShell
  width="form"
  title={$t('settings.nav.importBackup')}
  description={$t('settings.panes.importBackupDesc')}
>
  <SettingsGroup title={$t('settings.csvImport.section')}>
    <CsvImport />
  </SettingsGroup>
  <SettingsGroup title={$t('settings.backup.section')}>
    <BackupRestore />
  </SettingsGroup>
</PageShell>
