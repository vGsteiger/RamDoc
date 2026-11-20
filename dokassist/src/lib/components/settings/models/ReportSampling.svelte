<script lang="ts">
  import type { SamplerConfig } from '$lib/api';
  import { Input, Select, SettingsRow } from '$lib/components/ui';
  import {
    reportGenerationSettings,
    type ReportGenerationPreset,
  } from '$lib/stores/report-generation';
  import { t } from '$lib/translations';

  /* Token sampling for reports and referral letters (a persisted preference). */
  const samplerFields: readonly (readonly [keyof SamplerConfig, string, number, number, number])[] =
    [
      ['temperature', 'settings.samplerTemperature', 0, 2, 0.05],
      ['top_k', 'settings.samplerTopK', 1, 200, 1],
      ['top_p', 'settings.samplerTopP', 0, 1, 0.05],
      ['min_p', 'settings.samplerMinP', 0, 1, 0.05],
      ['repeat_penalty', 'settings.samplerRepeatPenalty', 0.8, 2, 0.05],
      ['presence_penalty', 'settings.samplerPresencePenalty', 0, 2, 0.1],
      ['seed', 'settings.samplerSeed', 0, 4_294_967_295, 1],
    ];

  function setReportPreset(preset: ReportGenerationPreset) {
    reportGenerationSettings.update((settings) => ({ ...settings, preset }));
  }

  function setSamplerValue(field: keyof SamplerConfig, value: number) {
    reportGenerationSettings.update((settings) => ({
      ...settings,
      custom: { ...settings.custom, [field]: value },
    }));
  }
</script>

<SettingsRow
  label={$t('settings.reportGenerationPreset')}
  description={$t('settings.reportGenerationPresetDescription')}
  for="report-generation-preset"
>
  {#snippet control()}
    <div class="w-full @min-[560px]:w-72">
      <Select
        id="report-generation-preset"
        value={$reportGenerationSettings.preset}
        onchange={(event: Event & { currentTarget: HTMLSelectElement }) =>
          setReportPreset(event.currentTarget.value as ReportGenerationPreset)}
      >
        <option value="clinical">{$t('settings.reportPresetClinical')}</option>
        <option value="conservative">{$t('settings.reportPresetConservative')}</option>
        <option value="varied">{$t('settings.reportPresetVaried')}</option>
        <option value="custom">{$t('settings.reportPresetCustom')}</option>
      </Select>
    </div>
  {/snippet}

  {#if $reportGenerationSettings.preset === 'custom'}
    <div class="mt-3 grid grid-cols-2 gap-3 @min-[560px]:grid-cols-3">
      {#each samplerFields as field (field[0])}
        <label class="block text-caption text-fg-muted">
          {$t(field[1])}
          <Input
            type="number"
            class="mt-1"
            value={$reportGenerationSettings.custom[field[0]]}
            min={field[2]}
            max={field[3]}
            step={field[4]}
            onchange={(event: Event & { currentTarget: HTMLInputElement }) =>
              setSamplerValue(field[0], event.currentTarget.valueAsNumber)}
          />
        </label>
      {/each}
    </div>
    <p class="mt-3 text-caption text-warning-fg">{$t('settings.customSamplerWarning')}</p>
  {/if}
</SettingsRow>
