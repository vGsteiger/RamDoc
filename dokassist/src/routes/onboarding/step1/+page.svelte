<script lang="ts">
  import { errorText } from '$lib/translations/labels';
  import { t } from '$lib/translations';
  import { goto } from '$app/navigation';
  import { onMount } from 'svelte';
  import { getSettings, updateSettings, type PracticeSettings } from '$lib/api';
  import { ChevronRight, Building, User, Mail, Phone, MapPin, Globe } from 'lucide-svelte';
  import OnboardingFrame from '$lib/components/OnboardingFrame.svelte';
  import { Alert, Button, Spinner } from '$lib/components/ui';

  let settings = $state<PracticeSettings>({
    practice_name: null,
    practice_address: null,
    practice_phone: null,
    practice_email: null,
    therapist_name: null,
    zsr_number: null,
    canton: null,
    clinical_specialty: null,
    language_preference: 'de',
    onboarding_completed: false,
  });

  let error = $state<string | null>(null);
  let isLoading = $state(false);
  let isSaving = $state(false);

  // Swiss cantons for dropdown
  const cantons = [
    { code: 'AG', name: 'Aargau' },
    { code: 'AI', name: 'Appenzell Innerrhoden' },
    { code: 'AR', name: 'Appenzell Ausserrhoden' },
    { code: 'BE', name: 'Bern' },
    { code: 'BL', name: 'Basel-Landschaft' },
    { code: 'BS', name: 'Basel-Stadt' },
    { code: 'FR', name: 'Fribourg' },
    { code: 'GE', name: 'Geneva' },
    { code: 'GL', name: 'Glarus' },
    { code: 'GR', name: 'Graubünden' },
    { code: 'JU', name: 'Jura' },
    { code: 'LU', name: 'Lucerne' },
    { code: 'NE', name: 'Neuchâtel' },
    { code: 'NW', name: 'Nidwalden' },
    { code: 'OW', name: 'Obwalden' },
    { code: 'SG', name: 'St. Gallen' },
    { code: 'SH', name: 'Schaffhausen' },
    { code: 'SO', name: 'Solothurn' },
    { code: 'SZ', name: 'Schwyz' },
    { code: 'TG', name: 'Thurgau' },
    { code: 'TI', name: 'Ticino' },
    { code: 'UR', name: 'Uri' },
    { code: 'VD', name: 'Vaud' },
    { code: 'VS', name: 'Valais' },
    { code: 'ZG', name: 'Zug' },
    { code: 'ZH', name: 'Zürich' },
  ];

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
      await updateSettings({ ...settings, canton: settings.canton || null });
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
    <!-- Practice form: fields and logic unchanged here; it is replaced by the
         shared PracticeForm from Settings once that lands. -->
    <div class="bg-surface-raised border border-line-subtle rounded-card p-8 space-y-6">
      <div class="grid grid-cols-1 md:grid-cols-2 gap-6">
        <div>
          <label for="practice-name" class="flex items-center gap-2 text-fg-muted mb-2">
            <Building size={16} />
            {$t('onboarding.step1.practiceName')}
          </label>
          <input
            id="practice-name"
            type="text"
            bind:value={settings.practice_name}
            placeholder={$t('onboarding.step1.practiceNamePlaceholder')}
            class="w-full px-4 py-3 bg-surface-hover border border-line rounded-control text-fg focus:outline-none focus:ring-2 focus:ring-accent/30"
          />
        </div>

        <div>
          <label for="therapist-name" class="flex items-center gap-2 text-fg-muted mb-2">
            <User size={16} />
            {$t('onboarding.step1.therapistName')}
          </label>
          <input
            id="therapist-name"
            type="text"
            bind:value={settings.therapist_name}
            placeholder={$t('onboarding.step1.therapistNamePlaceholder')}
            class="w-full px-4 py-3 bg-surface-hover border border-line rounded-control text-fg focus:outline-none focus:ring-2 focus:ring-accent/30"
          />
        </div>
      </div>

      <div>
        <label for="practice-address" class="flex items-center gap-2 text-fg-muted mb-2">
          <MapPin size={16} />
          {$t('onboarding.step1.practiceAddress')}
        </label>
        <textarea
          id="practice-address"
          bind:value={settings.practice_address}
          placeholder={$t('onboarding.step1.practiceAddressPlaceholder')}
          rows="3"
          class="w-full px-4 py-3 bg-surface-hover border border-line rounded-control text-fg focus:outline-none focus:ring-2 focus:ring-accent/30"
        ></textarea>
      </div>

      <div class="grid grid-cols-1 md:grid-cols-2 gap-6">
        <div>
          <label for="practice-phone" class="flex items-center gap-2 text-fg-muted mb-2">
            <Phone size={16} />
            {$t('onboarding.step1.phone')}
          </label>
          <input
            id="practice-phone"
            type="tel"
            bind:value={settings.practice_phone}
            placeholder="+41 XX XXX XX XX"
            class="w-full px-4 py-3 bg-surface-hover border border-line rounded-control text-fg focus:outline-none focus:ring-2 focus:ring-accent/30"
          />
        </div>

        <div>
          <label for="practice-email" class="flex items-center gap-2 text-fg-muted mb-2">
            <Mail size={16} />
            {$t('onboarding.step1.email')}
          </label>
          <input
            id="practice-email"
            type="email"
            bind:value={settings.practice_email}
            placeholder="practice@example.com"
            class="w-full px-4 py-3 bg-surface-hover border border-line rounded-control text-fg focus:outline-none focus:ring-2 focus:ring-accent/30"
          />
        </div>
      </div>

      <div class="grid grid-cols-1 md:grid-cols-2 gap-6">
        <div>
          <label for="zsr-number" class="text-fg-muted mb-2 block">
            {$t('onboarding.step1.zsrNumber')}
            <span class="text-fg-muted">{$t('onboarding.step1.zsrHint')}</span>
          </label>
          <input
            id="zsr-number"
            type="text"
            bind:value={settings.zsr_number}
            placeholder={$t('onboarding.step1.zsrPlaceholder')}
            class="w-full px-4 py-3 bg-surface-hover border border-line rounded-control text-fg focus:outline-none focus:ring-2 focus:ring-accent/30"
          />
        </div>

        <div>
          <label for="canton" class="text-fg-muted mb-2 block">
            {$t('onboarding.step1.canton')}
            <span class="text-fg-muted">{$t('onboarding.step1.cantonHint')}</span>
          </label>
          <select
            id="canton"
            value={settings.canton ?? ''}
            onchange={(e) => {
              settings.canton = e.currentTarget.value || null;
            }}
            class="w-full px-4 py-3 bg-surface-hover border border-line rounded-control text-fg focus:outline-none focus:ring-2 focus:ring-accent/30"
          >
            <option value="">{$t('onboarding.step1.cantonSelect')}</option>
            {#each cantons as canton (canton.code)}
              <option value={canton.code}>{canton.name} ({canton.code})</option>
            {/each}
          </select>
        </div>
      </div>

      <div>
        <label for="clinical-specialty" class="flex items-center gap-2 text-fg-muted mb-2">
          <Globe size={16} />
          {$t('onboarding.step1.clinicalSpecialty')}
        </label>
        <input
          id="clinical-specialty"
          type="text"
          bind:value={settings.clinical_specialty}
          placeholder={$t('onboarding.step1.clinicalSpecialtyPlaceholder')}
          class="w-full px-4 py-3 bg-surface-hover border border-line rounded-control text-fg focus:outline-none focus:ring-2 focus:ring-accent/30"
        />
      </div>
    </div>
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
