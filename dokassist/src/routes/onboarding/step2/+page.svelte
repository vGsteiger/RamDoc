<script lang="ts">
  import { errorText } from '$lib/translations/labels';
  import { t } from '$lib/translations';
  import { goto } from '$app/navigation';
  import { onMount, onDestroy } from 'svelte';
  import { listen, type UnlistenFn } from '@tauri-apps/api/event';
  import { getRecommendedModel, downloadAndRegisterModel, type ModelChoice } from '$lib/api';
  import {
    ChevronRight,
    ChevronLeft,
    Download,
    Cpu,
    Gauge,
    HardDrive,
    Target,
  } from 'lucide-svelte';
  import OnboardingFrame from '$lib/components/OnboardingFrame.svelte';
  import { Alert, Button, Card, ProgressBar, Spinner } from '$lib/components/ui';

  let recommended = $state<ModelChoice | null>(null);
  let error = $state<string | null>(null);
  let isLoading = $state(true);
  let isDownloading = $state(false);
  let downloadProgress = $state<number>(0);
  let isComplete = $state(false);

  let unlisten: UnlistenFn | null = null;
  let doneUnsubscribe: UnlistenFn | null = null;

  onMount(async () => {
    try {
      recommended = await getRecommendedModel();
    } catch (err) {
      error = $errorText(err);
    } finally {
      isLoading = false;
    }
  });

  onDestroy(() => {
    unlisten?.();
    doneUnsubscribe?.();
  });

  async function handleDownload() {
    if (!recommended) return;

    isDownloading = true;
    downloadProgress = 0;
    error = null;

    try {
      unlisten = await listen<number>('model-download-progress', (e) => {
        downloadProgress = Math.round(e.payload * 100);
      });

      doneUnsubscribe = await listen('model-download-done', () => {
        isComplete = true;
      });

      await downloadAndRegisterModel(recommended);
    } catch (err) {
      error = $errorText(err);
    } finally {
      doneUnsubscribe?.();
      unlisten?.();
      doneUnsubscribe = null;
      unlisten = null;
      isDownloading = false;
    }
  }

  function handleContinue() {
    goto('/onboarding/step3');
  }

  function handleBack() {
    goto('/onboarding/step1');
  }

  function formatBytes(bytes: number): string {
    const gb = bytes / 1024 ** 3;
    return `${gb.toFixed(1)} GB`;
  }
</script>

<OnboardingFrame
  step={2}
  title={$t('onboarding.step2.title')}
  subtitle={$t('onboarding.step2.subtitle')}
>
  {#if error}
    <Alert tone="danger" class="mb-6">{error}</Alert>
  {/if}

  {#if isLoading}
    <div class="flex justify-center py-12">
      <Spinner label={$t('onboarding.step2.loading')} />
    </div>
  {:else if recommended}
    <Card padding="none">
      <div class="flex items-start gap-3 p-5">
        <Cpu size={20} class="mt-0.5 shrink-0 text-fg-subtle" aria-hidden="true" />
        <div class="min-w-0">
          <h2 class="text-title text-fg">{recommended.name}</h2>
          <p class="mt-0.5 text-label text-fg-muted">
            {$t('onboarding.step2.recommendedForSystem')}
          </p>
        </div>
      </div>

      <dl class="grid grid-cols-3 border-y border-line-subtle">
        <div class="px-5 py-3">
          <dt class="flex items-center gap-1.5 text-caption text-fg-muted">
            <HardDrive size={14} aria-hidden="true" />
            {$t('onboarding.step2.size')}
          </dt>
          <dd class="mt-1 text-body font-medium text-fg" data-numeric>
            {formatBytes(recommended.size_bytes)}
          </dd>
        </div>
        <div class="border-l border-line-subtle px-5 py-3">
          <dt class="flex items-center gap-1.5 text-caption text-fg-muted">
            <Gauge size={14} aria-hidden="true" />
            {$t('onboarding.step2.speed')}
          </dt>
          <dd class="mt-1 text-body font-medium text-fg">
            {recommended.name.includes('30B')
              ? $t('onboarding.step2.speedGood')
              : $t('onboarding.step2.speedFast')}
          </dd>
        </div>
        <div class="border-l border-line-subtle px-5 py-3">
          <dt class="flex items-center gap-1.5 text-caption text-fg-muted">
            <Target size={14} aria-hidden="true" />
            {$t('onboarding.step2.quality')}
          </dt>
          <dd class="mt-1 text-body font-medium text-fg">
            {recommended.name.includes('30B')
              ? $t('onboarding.step2.qualityExcellent')
              : $t('onboarding.step2.qualityVeryGood')}
          </dd>
        </div>
      </dl>

      <div class="space-y-4 p-5">
        <p class="text-body text-fg-muted">
          <span class="font-medium text-fg">{$t('onboarding.step2.aboutModelLabel')}</span>
          {$t('onboarding.step2.aboutModel')}
        </p>

        {#if isDownloading}
          <div class="space-y-2">
            <ProgressBar
              value={downloadProgress}
              label={$t('onboarding.step2.downloading')}
              showValue
            />
            <p class="text-caption text-fg-muted">{$t('onboarding.step2.downloadHint')}</p>
          </div>
        {:else if isComplete}
          <Alert tone="success" title={$t('onboarding.step2.downloadComplete')}>
            {$t('onboarding.step2.modelReady')}
          </Alert>
        {/if}
      </div>
    </Card>
  {/if}

  {#snippet actions()}
    {#if !isLoading && recommended}
      <Button onclick={handleBack} disabled={isDownloading}>
        <ChevronLeft size={16} aria-hidden="true" />
        {$t('common.back')}
      </Button>

      <div class="flex flex-wrap gap-2">
        {#if !isComplete && !isDownloading}
          <Button variant="ghost" onclick={handleContinue}>{$t('onboarding.skipForNow')}</Button>
          <Button variant="primary" onclick={handleDownload}>
            <Download size={16} aria-hidden="true" />
            {$t('onboarding.step2.downloadModel')}
          </Button>
        {:else if isComplete}
          <Button variant="primary" onclick={handleContinue}>
            {$t('common.continue')}
            <ChevronRight size={16} aria-hidden="true" />
          </Button>
        {/if}
      </div>
    {/if}
  {/snippet}
</OnboardingFrame>
