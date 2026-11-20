<script lang="ts">
  import { PageShell, SegmentedControl, SettingsRow } from '$lib/components/ui';
  import SettingsGroup from './SettingsGroup.svelte';
  import { themePreference, type ThemeMode } from '$lib/stores/theme';
  import { language, type LanguageCode } from '$lib/stores/language';
  import { t } from '$lib/translations';

  // Language names are written in their own language, so they never change
  // with the UI language.
  const languageOptions = [
    { value: 'en', label: 'English' },
    { value: 'de', label: 'Deutsch' },
  ];

  let themeOptions = $derived([
    { value: 'light', label: $t('settings.light') },
    { value: 'dark', label: $t('settings.dark') },
    { value: 'system', label: $t('settings.system') },
  ]);
</script>

<PageShell
  width="full"
  title={$t('settings.nav.general')}
  description={$t('settings.panes.generalDesc')}
>
  <SettingsGroup>
    <SettingsRow label={$t('settings.language')} description={$t('settings.languageDescription')}>
      {#snippet control({ labelId, descriptionId })}
        <SegmentedControl
          options={languageOptions}
          value={$language}
          onchange={(value) => language.set(value as LanguageCode)}
          aria-labelledby={labelId}
          aria-describedby={descriptionId}
        />
      {/snippet}
    </SettingsRow>
    <SettingsRow label={$t('settings.theme')} description={$t('settings.themeDescription')}>
      {#snippet control({ labelId, descriptionId })}
        <SegmentedControl
          options={themeOptions}
          value={$themePreference}
          onchange={(value) => themePreference.set(value as ThemeMode)}
          aria-labelledby={labelId}
          aria-describedby={descriptionId}
        />
      {/snippet}
    </SettingsRow>
  </SettingsGroup>
</PageShell>
