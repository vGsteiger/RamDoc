<script module lang="ts">
  import type { PracticeSettings } from '$lib/api';

  /** Swiss cantons. Names are proper nouns and shown with their code. */
  export const CANTONS: readonly { code: string; name: string }[] = [
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

  /** The editable text fields, in the order the form shows them. */
  export const PRACTICE_TEXT_FIELDS = [
    'practice_name',
    'therapist_name',
    'clinical_specialty',
    'practice_address',
    'practice_phone',
    'practice_email',
    'zsr_number',
    'canton',
  ] as const satisfies readonly (keyof PracticeSettings)[];

  /** An empty record, for callers that render the form before loading. */
  export function emptyPracticeSettings(): PracticeSettings {
    return {
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
    };
  }

  /**
   * What to hand to `updateSettings`: text fields trimmed, blanks stored as
   * null (the backend treats null as "not set"), everything else — the
   * language preference and the onboarding flag — passed through untouched.
   */
  export function normalizePracticeSettings(settings: PracticeSettings): PracticeSettings {
    const next: PracticeSettings = { ...settings };
    for (const field of PRACTICE_TEXT_FIELDS) {
      const value = settings[field];
      next[field] = typeof value === 'string' && value.trim() !== '' ? value.trim() : null;
    }
    return next;
  }
</script>

<script lang="ts">
  import { Input, Select, SettingsRow, Textarea } from '$lib/components/ui';
  import SettingsGroup from '$lib/components/settings/SettingsGroup.svelte';
  import { t } from '$lib/translations';

  /*
   * The practice details form, shared by Settings → Practice and onboarding
   * step 1. It only edits `settings` in place (bind it); loading, saving and
   * the submit button belong to the caller, which wraps it in a <form>:
   *
   *   <form onsubmit={save}>
   *     <PracticeForm bind:settings disabled={saving} />
   *     <Button type="submit" variant="primary">…</Button>
   *   </form>
   *
   * and saves with `updateSettings(normalizePracticeSettings(settings))`.
   */
  let {
    settings = $bindable(),
    disabled = false,
    idPrefix = 'practice',
  }: {
    settings: PracticeSettings;
    disabled?: boolean;
    /** Prefix for the input ids, if two forms could ever share a page. */
    idPrefix?: string;
  } = $props();

  const id = (field: string) => `${idPrefix}-${field}`;
  const controlWidth = 'w-full @min-[560px]:w-72';
</script>

<SettingsGroup title={$t('settings.practice.groupPractice')}>
  <SettingsRow label={$t('settings.practice.name')} for={id('name')}>
    {#snippet control()}
      <div class={controlWidth}>
        <Input
          id={id('name')}
          bind:value={settings.practice_name}
          placeholder={$t('settings.practice.namePlaceholder')}
          autocomplete="organization"
          {disabled}
        />
      </div>
    {/snippet}
  </SettingsRow>
  <SettingsRow label={$t('settings.practice.therapist')} for={id('therapist')}>
    {#snippet control()}
      <div class={controlWidth}>
        <Input
          id={id('therapist')}
          bind:value={settings.therapist_name}
          placeholder={$t('settings.practice.therapistPlaceholder')}
          autocomplete="name"
          {disabled}
        />
      </div>
    {/snippet}
  </SettingsRow>
  <SettingsRow label={$t('settings.practice.specialty')} for={id('specialty')}>
    {#snippet control()}
      <div class={controlWidth}>
        <Input
          id={id('specialty')}
          bind:value={settings.clinical_specialty}
          placeholder={$t('settings.practice.specialtyPlaceholder')}
          {disabled}
        />
      </div>
    {/snippet}
  </SettingsRow>
</SettingsGroup>

<SettingsGroup title={$t('settings.practice.groupContact')}>
  <SettingsRow label={$t('settings.practice.address')} for={id('address')}>
    {#snippet control()}
      <div class={controlWidth}>
        <Textarea
          id={id('address')}
          bind:value={() => settings.practice_address ?? '', (v) => (settings.practice_address = v)}
          placeholder={$t('settings.practice.addressPlaceholder')}
          rows={3}
          autocomplete="street-address"
          {disabled}
        />
      </div>
    {/snippet}
  </SettingsRow>
  <SettingsRow label={$t('settings.practice.phone')} for={id('phone')}>
    {#snippet control()}
      <div class={controlWidth}>
        <Input
          id={id('phone')}
          type="tel"
          bind:value={settings.practice_phone}
          placeholder="+41 XX XXX XX XX"
          autocomplete="tel"
          {disabled}
        />
      </div>
    {/snippet}
  </SettingsRow>
  <SettingsRow label={$t('settings.practice.email')} for={id('email')}>
    {#snippet control()}
      <div class={controlWidth}>
        <Input
          id={id('email')}
          type="email"
          bind:value={settings.practice_email}
          placeholder="practice@example.com"
          autocomplete="email"
          {disabled}
        />
      </div>
    {/snippet}
  </SettingsRow>
</SettingsGroup>

<SettingsGroup title={$t('settings.practice.groupBilling')}>
  <SettingsRow
    label={$t('settings.practice.zsr')}
    description={$t('settings.practice.zsrHint')}
    for={id('zsr')}
  >
    {#snippet control()}
      <div class={controlWidth}>
        <Input
          id={id('zsr')}
          bind:value={settings.zsr_number}
          placeholder={$t('settings.practice.zsrPlaceholder')}
          {disabled}
        />
      </div>
    {/snippet}
  </SettingsRow>
  <SettingsRow
    label={$t('settings.practice.canton')}
    description={$t('settings.practice.cantonHint')}
    for={id('canton')}
  >
    {#snippet control()}
      <div class={controlWidth}>
        <Select
          id={id('canton')}
          value={settings.canton ?? ''}
          onchange={(e: Event & { currentTarget: HTMLSelectElement }) => {
            settings.canton = e.currentTarget.value || null;
          }}
          {disabled}
        >
          <option value="">{$t('settings.practice.cantonSelect')}</option>
          {#each CANTONS as canton (canton.code)}
            <option value={canton.code}>{canton.name} ({canton.code})</option>
          {/each}
        </Select>
      </div>
    {/snippet}
  </SettingsRow>
</SettingsGroup>
