<script lang="ts">
  import { errorText } from '$lib/translations/labels';
  import { t } from '$lib/translations';
  import { goto } from '$app/navigation';
  import { onMount } from 'svelte';
  import { getSettings, updateSettings, type PracticeSettings } from '$lib/api';
  import { ChevronRight } from 'lucide-svelte';
  import OnboardingFrame from '$lib/components/OnboardingFrame.svelte';
  import PracticeForm, {
    emptyPracticeSettings,
    normalizePracticeSettings,
  } from '$lib/components/PracticeForm.svelte';
  import { Alert, Button, Spinner } from '$lib/components/ui';

  let settings = $state<PracticeSettings>(emptyPracticeSettings());

  let error = $state<string | null>(null);
  let isLoading = $state(false);
  let isSaving = $state(false);

  onMount(async () => {
    isLoading = true;
    try {
      settings = await getSettings();
    } catch (err) {
      console.error('Failed to load settings:', err);
    } finally {
      isLoading = false;
    }
  });

  async function handleContinue() {
    isSaving = true;
    error = null;

    try {
      await updateSettings(normalizePracticeSettings(settings));
      goto('/onboarding/step2');
    } catch (err) {
      error = $errorText(err);
    } finally {
      isSaving = false;
    }
  }

  function handleSkip() {
    goto('/onboarding/step2');
  }
</script>

<OnboardingFrame
  step={1}
  title={$t('onboarding.step1.title')}
  subtitle={$t('onboarding.step1.subtitle')}
>
  {#if error}
    <Alert tone="danger" class="mb-6">{error}</Alert>
  {/if}

  {#if isLoading}
    <div class="flex justify-center py-12">
      <Spinner label={$t('onboarding.step1.loading')} />
    </div>
  {:else}
    <form
      id="onboarding-practice"
      class="space-y-6"
      onsubmit={(e) => {
        e.preventDefault();
        void handleContinue();
      }}
    >
      <PracticeForm bind:settings disabled={isSaving} idPrefix="onboarding-practice" />
    </form>
  {/if}

  {#snippet actions()}
    {#if !isLoading}
      <Button variant="ghost" onclick={handleSkip}>{$t('onboarding.skipForNow')}</Button>
      <Button variant="primary" onclick={handleContinue} loading={isSaving}>
        {#if isSaving}
          {$t('common.saving')}
        {:else}
          {$t('common.continue')}
          <ChevronRight size={16} aria-hidden="true" />
        {/if}
      </Button>
    {/if}
  {/snippet}
</OnboardingFrame>
