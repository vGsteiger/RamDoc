<script lang="ts">
  import { errorText } from '$lib/translations/labels';
  import { t } from '$lib/translations';
  import { goto } from '$app/navigation';
  import { completeOnboarding } from '$lib/api';
  import {
    ChevronLeft,
    ChevronRight,
    Users,
    Mic,
    Search,
    FileText,
    Calendar,
    ShieldCheck,
  } from 'lucide-svelte';
  import OnboardingFrame from '$lib/components/OnboardingFrame.svelte';
  import { Alert, Button, Card, Kbd, SectionHeader } from '$lib/components/ui';

  let isCompleting = $state(false);
  let error = $state<string | null>(null);

  async function handleComplete() {
    isCompleting = true;
    error = null;

    try {
      await completeOnboarding();
      goto('/dashboard');
    } catch (err) {
      error = $errorText(err);
      isCompleting = false;
    }
  }

  function handleBack() {
    goto('/onboarding/step3');
  }

  const features = [
    { icon: Users, key: 'patients' },
    { icon: Calendar, key: 'scheduling' },
    { icon: Mic, key: 'recording' },
    { icon: Search, key: 'search' },
    { icon: FileText, key: 'reports' },
  ];
</script>

<OnboardingFrame
  step={4}
  title={$t('onboarding.step4.title')}
  subtitle={$t('onboarding.step4.subtitle')}
>
  {#if error}
    <Alert tone="danger" class="mb-6">{error}</Alert>
  {/if}

  <ul class="grid gap-x-6 gap-y-5 sm:grid-cols-2">
    {#each features as feature (feature.key)}
      {@const FeatureIcon = feature.icon}
      <li class="flex items-start gap-3">
        <FeatureIcon size={18} class="mt-0.5 shrink-0 text-fg-subtle" aria-hidden="true" />
        <div class="min-w-0">
          <h2 class="text-body font-medium text-fg">
            {$t(`onboarding.step4.features.${feature.key}Title`)}
          </h2>
          <p class="mt-0.5 text-label text-fg-muted">
            {$t(`onboarding.step4.features.${feature.key}Desc`)}
          </p>
        </div>
      </li>
    {/each}
  </ul>

  <Card class="mt-8">
    <div class="flex items-start gap-3">
      <ShieldCheck size={18} class="mt-0.5 shrink-0 text-fg-subtle" aria-hidden="true" />
      <div class="min-w-0">
        <h2 class="text-body font-medium text-fg">{$t('onboarding.step4.privacyTitle')}</h2>
        <p class="mt-0.5 text-label leading-relaxed text-fg-muted">
          {$t('onboarding.step4.privacyBody')}
        </p>
      </div>
    </div>
  </Card>

  <section class="mt-8" aria-labelledby="onboarding-shortcuts">
    <SectionHeader id="onboarding-shortcuts" title={$t('onboarding.step4.shortcutsTitle')} />
    <dl class="grid gap-x-8 gap-y-2 sm:grid-cols-2">
      <div class="flex items-center justify-between gap-3">
        <dt class="text-body text-fg-muted">{$t('onboarding.step4.openPalette')}</dt>
        <dd><Kbd>Cmd+K</Kbd></dd>
      </div>
      <div class="flex items-center justify-between gap-3">
        <dt class="text-body text-fg-muted">{$t('onboarding.step4.newChat')}</dt>
        <dd><Kbd>{$t('onboarding.step4.shortcutNewChat')}</Kbd></dd>
      </div>
      <div class="flex items-center justify-between gap-3">
        <dt class="text-body text-fg-muted">{$t('onboarding.step4.newPatient')}</dt>
        <dd><Kbd>{$t('onboarding.step4.shortcutNewPatient')}</Kbd></dd>
      </div>
      <div class="flex items-center justify-between gap-3">
        <dt class="text-body text-fg-muted">{$t('onboarding.step4.newSession')}</dt>
        <dd><Kbd>Cmd+Shift+S</Kbd></dd>
      </div>
      <div class="flex items-center justify-between gap-3">
        <dt class="text-body text-fg-muted">{$t('onboarding.step4.closePalette')}</dt>
        <dd><Kbd>Esc</Kbd></dd>
      </div>
    </dl>
  </section>

  {#snippet actions()}
    <Button onclick={handleBack} disabled={isCompleting}>
      <ChevronLeft size={16} aria-hidden="true" />
      {$t('common.back')}
    </Button>
    <Button variant="primary" onclick={handleComplete} loading={isCompleting}>
      {#if isCompleting}
        {$t('onboarding.step4.completing')}
      {:else}
        {$t('onboarding.step4.getStarted')}
        <ChevronRight size={16} aria-hidden="true" />
      {/if}
    </Button>
  {/snippet}
</OnboardingFrame>
