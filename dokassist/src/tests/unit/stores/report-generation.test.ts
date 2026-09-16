import { beforeEach, describe, expect, it, vi } from 'vitest';
import { get } from 'svelte/store';

describe('report generation settings', () => {
  beforeEach(() => {
    localStorage.clear();
    vi.resetModules();
  });

  it('defaults to the clinical preset', async () => {
    const {
      reportGenerationSettings,
      resolveReportSampler,
      resolveReportThinkingEffort,
      REPORT_SAMPLER_PRESETS,
    } = await import('$lib/stores/report-generation');
    const settings = get(reportGenerationSettings);
    expect(settings.preset).toBe('clinical');
    expect(resolveReportSampler(settings)).toEqual(REPORT_SAMPLER_PRESETS.clinical);
    expect(resolveReportThinkingEffort('Ueberweisungsschreiben', null)).toBe('low');
    expect(resolveReportThinkingEffort('Befundbericht', null)).toBe('medium');
    expect(resolveReportThinkingEffort('Ueberweisungsschreiben', 'high')).toBe('high');
  });

  it('persists a valid custom sampler', async () => {
    const { reportGenerationSettings, resolveReportSampler } =
      await import('$lib/stores/report-generation');
    reportGenerationSettings.update((settings) => ({
      ...settings,
      preset: 'custom',
      custom: { ...settings.custom, temperature: 0.45, seed: 42 },
    }));
    const settings = get(reportGenerationSettings);
    expect(resolveReportSampler(settings).temperature).toBe(0.45);
    expect(resolveReportSampler(settings).seed).toBe(42);
    expect(localStorage.getItem('report-generation-settings-v1')).toContain('"custom"');
  });

  it('does not persist an out-of-range custom value', async () => {
    const { reportGenerationSettings } = await import('$lib/stores/report-generation');
    reportGenerationSettings.update((settings) => ({
      ...settings,
      preset: 'custom',
      custom: { ...settings.custom, top_p: 2 },
    }));
    expect(get(reportGenerationSettings).preset).toBe('clinical');
  });
});
