<script lang="ts">
  import { ChevronRight } from 'lucide-svelte';
  import type { InferenceFallbackCode, InferenceProfile, LlmEngineStatus } from '$lib/api';
  import { Select, SettingsRow } from '$lib/components/ui';
  import { t } from '$lib/translations';

  /*
   * Expert diagnostics, behind a disclosure: the runtime profile used on the
   * next model load, the active inference configuration (KV cache, batch
   * sizes, Flash Attention, fallbacks) and the last generation's timing.
   */
  let {
    status,
    inferenceProfile = $bindable(),
    disabled = false,
  }: {
    status: LlmEngineStatus | null;
    inferenceProfile: InferenceProfile;
    disabled?: boolean;
  } = $props();

  function fallbackLabel(code: InferenceFallbackCode): string | null {
    switch (code) {
      case 'native_context_cap':
        return $t('settings.fallbackNativeContext');
      case 'flash_auto':
        return $t('settings.fallbackFlashAuto');
      case 'kv_f16':
        return $t('settings.fallbackKvF16');
      case 'kv_f16_flash_auto':
        return $t('settings.fallbackKvF16FlashAuto');
      default:
        return null;
    }
  }

  function fallbackDiagnostic(config: NonNullable<LlmEngineStatus['inference_config']>) {
    if (!config.fallback_code) return config.fallback;
    return fallbackLabel(config.fallback_code) || config.fallback;
  }
</script>

<details class="group mb-section rounded-card border border-line-subtle bg-surface-raised">
  <summary
    class="flex cursor-pointer list-none items-start gap-2 rounded-card px-4 py-3 hover:bg-surface-hover"
  >
    <ChevronRight
      size={16}
      strokeWidth={1.5}
      class="mt-0.5 shrink-0 text-fg-subtle group-open:rotate-90"
      aria-hidden="true"
    />
    <span class="min-w-0">
      <span class="block text-body font-medium text-fg">{$t('settings.models.diagnostics')}</span>
      <span class="block text-label text-fg-muted">{$t('settings.models.diagnosticsDesc')}</span>
    </span>
  </summary>

  <div class="border-t border-line-subtle px-4">
    <SettingsRow
      label={$t('settings.inferenceProfile')}
      description={$t('settings.inferenceProfileDescription')}
      for="inference-profile"
    >
      {#snippet control()}
        <div class="w-full @min-[560px]:w-72">
          <Select id="inference-profile" bind:value={inferenceProfile} {disabled}>
            <option value="conservative">{$t('settings.inferenceConservative')}</option>
            <option value="f16-32k">{$t('settings.inferenceF16')}</option>
            <option value="q8-32k">{$t('settings.inferenceQ8')}</option>
            <option value="q4-32k">{$t('settings.inferenceQ4')}</option>
          </Select>
        </div>
      {/snippet}
    </SettingsRow>

    {#if status?.inference_config}
      {@const config = status.inference_config}
      {@const fallback = fallbackDiagnostic(config)}
      <SettingsRow label={$t('settings.activeInferenceConfiguration')}>
        <p class="mt-1 text-label text-fg-muted" data-numeric>
          {$t('settings.inferenceConfigSummary')
            .replace('{context}', String(Math.round(config.context_size / 1024)))
            .replace('{key}', config.kv_cache_k)
            .replace('{value}', config.kv_cache_v)
            .replace('{batch}', String(config.n_batch))
            .replace('{microBatch}', String(config.n_ubatch))
            .replace(
              '{flash}',
              $t(
                config.flash_attention === 'enabled'
                  ? 'settings.flashEnabled'
                  : 'settings.flashAuto'
              )
            )}
        </p>
        <p class="mt-1 text-caption text-fg-subtle">
          {$t('settings.completionHeadroom').replace(
            '{tokens}',
            String(config.completion_headroom)
          )}
        </p>
        {#if fallback}
          <p
            class="mt-2 rounded-control border border-warning-line bg-warning-subtle px-2.5 py-2 text-caption text-warning-fg"
          >
            {fallback}
          </p>
        {/if}
      </SettingsRow>
    {/if}

    {#if status?.last_generation_stats}
      {@const s = status.last_generation_stats}
      <SettingsRow label={$t('settings.lastGenerationPerformance')}>
        <dl class="mt-2 grid grid-cols-2 gap-3" data-numeric>
          <div class="rounded-control border border-line-subtle p-2.5">
            <dt class="text-caption text-fg-muted">{$t('settings.timeToFirstToken')}</dt>
            <dd class="text-heading text-fg">{(s.ttft_ms / 1000).toFixed(2)} s</dd>
          </div>
          <div class="rounded-control border border-line-subtle p-2.5">
            <dt class="text-caption text-fg-muted">{$t('settings.throughput')}</dt>
            <dd class="text-heading text-fg">{s.tps.toFixed(1)} tok/s</dd>
          </div>
        </dl>
        <p class="mt-2 text-caption text-fg-subtle">
          {$t('settings.tokenStats')
            .replace('{prompt}', String(s.prompt_tokens))
            .replace('{completion}', String(s.completion_tokens))}
        </p>
        <p class="mt-1 text-caption text-fg-subtle">
          {s.cache_hit ? $t('settings.contextWarm') : $t('settings.contextCold')} ·
          {$t('settings.cacheStats')
            .replace('{reused}', String(s.reused_prompt_tokens))
            .replace('{evaluated}', String(s.evaluated_prompt_tokens))
            .replace('{prefill}', s.prefill_ms.toFixed(0))}
        </p>
      </SettingsRow>
    {/if}
  </div>
</details>
