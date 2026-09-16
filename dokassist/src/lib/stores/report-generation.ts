import { browser } from '$app/environment';
import { writable } from 'svelte/store';
import type { SamplerConfig } from '$lib/api';

const STORAGE_KEY = 'report-generation-settings-v1';

export type ReportGenerationPreset = 'clinical' | 'conservative' | 'varied' | 'custom';

export interface ReportGenerationSettings {
  preset: ReportGenerationPreset;
  custom: SamplerConfig;
}

// Presets deliberately change sampling only. Reasoning effort remains an
// independent user choice because it controls latency and the think budget.
export const REPORT_SAMPLER_PRESETS: Record<
  Exclude<ReportGenerationPreset, 'custom'>,
  SamplerConfig
> = {
  clinical: {
    temperature: 0.35,
    top_k: 40,
    top_p: 0.9,
    min_p: 0.05,
    repeat_penalty: 1,
    presence_penalty: 0,
    seed: 0,
  },
  conservative: {
    temperature: 0.2,
    top_k: 20,
    top_p: 0.9,
    min_p: 0,
    repeat_penalty: 1,
    presence_penalty: 0,
    seed: 0,
  },
  varied: {
    temperature: 0.6,
    top_k: 20,
    top_p: 0.95,
    min_p: 0,
    repeat_penalty: 1,
    presence_penalty: 0,
    seed: 0,
  },
};

const DEFAULT_SETTINGS: ReportGenerationSettings = {
  preset: 'clinical',
  custom: { ...REPORT_SAMPLER_PRESETS.clinical },
};

const PRESETS: ReportGenerationPreset[] = ['clinical', 'conservative', 'varied', 'custom'];

function isSamplerConfig(value: unknown): value is SamplerConfig {
  if (!value || typeof value !== 'object') return false;
  const sampler = value as Record<string, unknown>;
  return (
    typeof sampler.temperature === 'number' &&
    sampler.temperature >= 0 &&
    sampler.temperature <= 2 &&
    Number.isInteger(sampler.top_k) &&
    Number(sampler.top_k) >= 1 &&
    Number(sampler.top_k) <= 200 &&
    typeof sampler.top_p === 'number' &&
    sampler.top_p >= 0 &&
    sampler.top_p <= 1 &&
    typeof sampler.min_p === 'number' &&
    sampler.min_p >= 0 &&
    sampler.min_p <= 1 &&
    typeof sampler.repeat_penalty === 'number' &&
    sampler.repeat_penalty >= 0.8 &&
    sampler.repeat_penalty <= 2 &&
    typeof sampler.presence_penalty === 'number' &&
    sampler.presence_penalty >= 0 &&
    sampler.presence_penalty <= 2 &&
    Number.isInteger(sampler.seed) &&
    Number(sampler.seed) >= 0 &&
    Number(sampler.seed) <= 4_294_967_295
  );
}

function initialSettings(): ReportGenerationSettings {
  if (!browser) return DEFAULT_SETTINGS;
  try {
    const parsed = JSON.parse(
      localStorage.getItem(STORAGE_KEY) ?? 'null'
    ) as Partial<ReportGenerationSettings> | null;
    if (
      parsed &&
      PRESETS.includes(parsed.preset as ReportGenerationPreset) &&
      isSamplerConfig(parsed.custom)
    ) {
      return { preset: parsed.preset as ReportGenerationPreset, custom: parsed.custom };
    }
  } catch {
    // Ignore corrupt local settings and return the validated default.
  }
  return DEFAULT_SETTINGS;
}

function createReportGenerationSettingsStore() {
  const store = writable<ReportGenerationSettings>(initialSettings());
  return {
    subscribe: store.subscribe,
    set(value: ReportGenerationSettings) {
      const next =
        PRESETS.includes(value.preset) && isSamplerConfig(value.custom) ? value : DEFAULT_SETTINGS;
      if (browser) localStorage.setItem(STORAGE_KEY, JSON.stringify(next));
      store.set(next);
    },
    update(updater: (value: ReportGenerationSettings) => ReportGenerationSettings) {
      store.update((current) => {
        const candidate = updater(current);
        const next =
          PRESETS.includes(candidate.preset) && isSamplerConfig(candidate.custom)
            ? candidate
            : current;
        if (browser) localStorage.setItem(STORAGE_KEY, JSON.stringify(next));
        return next;
      });
    },
  };
}

export function resolveReportSampler(settings: ReportGenerationSettings): SamplerConfig {
  return settings.preset === 'custom'
    ? { ...settings.custom }
    : { ...REPORT_SAMPLER_PRESETS[settings.preset] };
}

export const reportGenerationSettings = createReportGenerationSettingsStore();
