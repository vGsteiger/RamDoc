<script lang="ts">
  import { t } from '$lib/translations';
  import { goto } from '$app/navigation';
  import { ChevronRight, ChevronLeft, FileText, Database } from 'lucide-svelte';
  import OnboardingFrame from '$lib/components/OnboardingFrame.svelte';
  import { Alert, Button, Card, SectionHeader } from '$lib/components/ui';

  function handleBack() {
    goto('/onboarding/step2');
  }

  function handleImportLater() {
    goto('/onboarding/step4');
  }
</script>

<OnboardingFrame
  step={3}
  title={$t('onboarding.step3.title')}
  subtitle={$t('onboarding.step3.subtitle')}
>
  <SectionHeader
    title={$t('onboarding.step3.optionsTitle')}
    description={$t('onboarding.step3.optionsSubtitle')}
  />
  <div class="space-y-2">
    <Card href="/settings" padding="none">
      <div class="flex items-start gap-3 p-4">
        <FileText size={18} class="mt-0.5 shrink-0 text-fg-subtle" aria-hidden="true" />
        <div class="min-w-0 flex-1">
          <h3 class="text-body font-medium text-fg">{$t('onboarding.step3.importCsv')}</h3>
          <p class="mt-0.5 text-label text-fg-muted">{$t('onboarding.step3.importCsvDesc')}</p>
        </div>
        <ChevronRight size={16} class="mt-0.5 shrink-0 text-fg-subtle" aria-hidden="true" />
      </div>
    </Card>
    <Card href="/settings" padding="none">
      <div class="flex items-start gap-3 p-4">
        <Database size={18} class="mt-0.5 shrink-0 text-fg-subtle" aria-hidden="true" />
        <div class="min-w-0 flex-1">
          <h3 class="text-body font-medium text-fg">{$t('onboarding.step3.restoreBackup')}</h3>
          <p class="mt-0.5 text-label text-fg-muted">
            {$t('onboarding.step3.restoreBackupDesc')}
          </p>
        </div>
        <ChevronRight size={16} class="mt-0.5 shrink-0 text-fg-subtle" aria-hidden="true" />
      </div>
    </Card>
  </div>

  <Alert tone="info" class="mt-6">
    <span class="font-medium">{$t('onboarding.step3.noteLabel')}</span>
    {$t('onboarding.step3.note')}
  </Alert>

  {#snippet actions()}
    <Button onclick={handleBack}>
      <ChevronLeft size={16} aria-hidden="true" />
      {$t('common.back')}
    </Button>
    <Button variant="primary" onclick={handleImportLater}>
      {$t('onboarding.skipForNow')}
      <ChevronRight size={16} aria-hidden="true" />
    </Button>
  {/snippet}
</OnboardingFrame>
