<script lang="ts">
  import { page } from '$app/state';
  import { goto } from '$app/navigation';
  import { t } from '$lib/translations';
  import { listen, type UnlistenFn } from '@tauri-apps/api/event';
  import { onDestroy, onMount } from 'svelte';
  import {
    createLetter,
    generateLetter,
    getPatient,
    listDiagnosesForPatient,
    listMedicationsForPatient,
    type LetterType,
    type LetterLanguage,
    type Patient,
    type Diagnosis,
    type Medication,
  } from '$lib/api';
  import { language as appLanguage } from '$lib/stores/language';
  import { thinkingEffort } from '$lib/stores/thinking';
  import { get } from 'svelte/store';
  import ThinkingEffortSelect from '$lib/components/ThinkingEffortSelect.svelte';
  import {
    Alert,
    Button,
    Field,
    Input,
    ListSkeleton,
    PageShell,
    SectionHeader,
    Select,
    Textarea,
    ThinkingIndicator,
  } from '$lib/components/ui';
  import { ArrowLeft } from 'lucide-svelte';
  import StickyActionBar from '$lib/components/documents/StickyActionBar.svelte';
  import { documentsUrl } from '$lib/components/documents/documents';
  import { stripThinkTags } from '$lib/llm/strip-think';

  const patientId = $derived(page.params.id!);

  let patient = $state<Patient | null>(null);
  let diagnoses = $state<Diagnosis[]>([]);
  let medications = $state<Medication[]>([]);

  let letterType = $state<LetterType>('referral');
  let letterLanguage = $state<LetterLanguage>('de');
  let recipientName = $state('');
  let recipientAddress = $state('');
  let subject = $state('');
  let patientContext = $state('');
  let clinicalSummary = $state('');
  let generatedContent = $state('');
  let editableContent = $state('');

  let isGenerating = $state(false);
  let activityStartedAt = $state<number | null>(null);
  let isLoading = $state(true);
  let isSaving = $state(false);
  let error = $state<string | null>(null);

  let unlistenChunk: UnlistenFn | null = null;
  let unlistenDone: UnlistenFn | null = null;

  onMount(async () => {
    try {
      patient = await getPatient(patientId);
      diagnoses = await listDiagnosesForPatient(patientId, 10, 0);
      medications = await listMedicationsForPatient(patientId, 10, 0);

      // Auto-fill patient context
      patientContext = formatPatientContext();

      // Set default language based on app language
      letterLanguage = ($appLanguage as string) === 'fr' ? 'fr' : 'de'; // Only de/fr supported for letters

      isLoading = false;
    } catch (err) {
      error = String(err);
      isLoading = false;
    }
  });

  onDestroy(() => {
    if (unlistenChunk) unlistenChunk();
    if (unlistenDone) unlistenDone();
  });

  function formatPatientContext(): string {
    if (!patient) return '';

    const lines: string[] = [];
    lines.push(`Name: ${patient.first_name} ${patient.last_name}`);
    if (patient.date_of_birth) lines.push(`Geburtsdatum: ${patient.date_of_birth}`);
    if (patient.ahv_number) lines.push(`AHV-Nummer: ${patient.ahv_number}`);
    if (patient.insurance) lines.push(`Versicherung: ${patient.insurance}`);

    if (diagnoses.length > 0) {
      lines.push('\nDiagnosen:');
      diagnoses.forEach((d) => {
        lines.push(`- ${d.icd10_code}: ${d.description} (${d.status})`);
      });
    }

    if (medications.length > 0) {
      lines.push('\nAktuelle Medikation:');
      medications.forEach((m) => {
        lines.push(`- ${m.substance} ${m.dosage}, ${m.frequency}`);
      });
    }

    return lines.join('\n');
  }

  async function handleGenerate() {
    if (!letterType) {
      error = $t('letters.selectTypeRequired');
      return;
    }

    // Unlisten to previous handlers before registering new ones
    if (unlistenChunk) {
      unlistenChunk();
      unlistenChunk = null;
    }
    if (unlistenDone) {
      unlistenDone();
      unlistenDone = null;
    }

    isGenerating = true;
    activityStartedAt = Date.now();
    generatedContent = '';
    error = null;

    try {
      unlistenChunk = await listen<string>('letter-chunk', (event) => {
        generatedContent += event.payload;
      });

      unlistenDone = await listen('letter-done', () => {
        isGenerating = false;
        activityStartedAt = null;
        editableContent = stripThinkTags(generatedContent);
        // Unlisten after generation completes
        if (unlistenChunk) {
          unlistenChunk();
          unlistenChunk = null;
        }
        if (unlistenDone) {
          unlistenDone();
          unlistenDone = null;
        }
      });

      await generateLetter(
        letterType,
        letterLanguage,
        patientContext,
        clinicalSummary,
        recipientName || undefined,
        undefined,
        get(thinkingEffort)
      );
    } catch (err) {
      error = String(err);
      isGenerating = false;
      activityStartedAt = null;
      if (unlistenChunk) unlistenChunk();
      if (unlistenDone) unlistenDone();
    }
  }

  async function handleSave() {
    if (!editableContent.trim()) {
      error = $t('letters.contentRequired');
      return;
    }

    isSaving = true;
    error = null;

    try {
      await createLetter({
        patient_id: patientId,
        letter_type: letterType,
        template_language: letterLanguage,
        recipient_name: recipientName || undefined,
        recipient_address: recipientAddress || undefined,
        subject: subject || `${letterType} - ${patient?.first_name} ${patient?.last_name}`,
        content: editableContent,
      });

      goto(documentsUrl(patientId, 'letters'));
    } catch (err) {
      error = String(err);
      isSaving = false;
    }
  }

  let hasDraft = $derived(Boolean(generatedContent || editableContent || isGenerating));
</script>

<PageShell width="prose" title={$t('letters.newLetterTitle')}>
  {#snippet breadcrumb()}
    <a
      href={documentsUrl(patientId, 'letters')}
      class="inline-flex items-center gap-1 text-fg-subtle hover:text-fg"
    >
      <ArrowLeft size={12} aria-hidden="true" />
      {$t('letters.backToDocuments')}
    </a>
  {/snippet}

  {#if isLoading}
    <p class="sr-only">{$t('letters.loading')}</p>
    <ListSkeleton count={4} />
  {:else if error && !patient}
    <Alert tone="danger">{error}</Alert>
  {:else}
    {#if error}
      <Alert tone="danger" class="mb-4">{error}</Alert>
    {/if}

    <section class="space-y-4">
      <SectionHeader title={$t('letters.settings')} />
      <div class="grid grid-cols-1 gap-4 sm:grid-cols-2">
        <Field
          label={$t('letters.selectType')}
          for="letter-type"
          hint={$t(`letters.typeDescriptions.${letterType}`)}
        >
          <Select id="letter-type" bind:value={letterType}>
            <option value="referral">{$t('letters.types.referral')}</option>
            <option value="insurance_authorization"
              >{$t('letters.types.insurance_authorization')}</option
            >
            <option value="therapy_extension">{$t('letters.types.therapy_extension')}</option>
          </Select>
        </Field>

        <Field label={$t('letters.selectLanguage')} for="letter-language">
          <Select id="letter-language" bind:value={letterLanguage}>
            <option value="de">Deutsch</option>
            <option value="fr">Français</option>
          </Select>
        </Field>

        <Field label={$t('letters.recipientName')} for="letter-recipient-name">
          <Input
            id="letter-recipient-name"
            bind:value={recipientName}
            placeholder={$t('letters.recipientNamePlaceholder')}
          />
        </Field>

        <Field label={$t('letters.subject')} for="letter-subject">
          <Input
            id="letter-subject"
            bind:value={subject}
            placeholder={$t('letters.subjectPlaceholder')}
          />
        </Field>
      </div>

      <Field label={$t('letters.recipientAddress')} for="letter-recipient-address">
        <Textarea
          id="letter-recipient-address"
          bind:value={recipientAddress}
          placeholder={$t('letters.recipientAddressPlaceholder')}
          rows={2}
        />
      </Field>
    </section>

    <section class="mt-section space-y-4">
      <SectionHeader title={$t('letters.sources')} />
      <Field
        label={$t('letters.patientContext')}
        for="letter-patient-context"
        hint={$t('letters.patientContextHint')}
      >
        <Textarea
          id="letter-patient-context"
          bind:value={patientContext}
          rows={6}
          class="font-mono text-label"
        />
      </Field>

      <Field label={$t('letters.clinicalSummary')} for="letter-clinical-summary">
        <Textarea
          id="letter-clinical-summary"
          bind:value={clinicalSummary}
          placeholder={$t('letters.clinicalSummaryPlaceholder')}
          rows={4}
        />
      </Field>
    </section>

    <section class="mt-section space-y-3">
      <SectionHeader title={$t('letters.draftSection')}>
        {#snippet actions()}
          {#if isGenerating && activityStartedAt}
            <span role="status" aria-live="polite">
              <ThinkingIndicator
                startedAt={activityStartedAt}
                label={$t('letters.activityWriting')}
              />
            </span>
          {/if}
        {/snippet}
      </SectionHeader>

      {#if hasDraft}
        <p class="text-label text-fg-muted">{$t('letters.reviewBeforeSaving')}</p>
        {#if isGenerating}
          <Textarea
            id="letter-generated"
            value={generatedContent}
            rows={20}
            readonly
            class="bg-surface-sunken leading-relaxed"
            aria-label={$t('letters.generatedLetter')}
          />
        {:else}
          <Textarea
            id="letter-generated"
            bind:value={editableContent}
            rows={20}
            class="leading-relaxed"
            aria-label={$t('letters.generatedLetter')}
          />
        {/if}
      {:else}
        <p class="text-label text-fg-subtle">{$t('letters.generateFirstHint')}</p>
      {/if}
    </section>

    <StickyActionBar>
      {#snippet start()}
        <Button variant="ghost" href={documentsUrl(patientId, 'letters')}>
          {$t('common.cancel')}
        </Button>
      {/snippet}
      <ThinkingEffortSelect disabled={isGenerating} />
      <Button
        variant={editableContent ? 'secondary' : 'primary'}
        onclick={handleGenerate}
        disabled={isGenerating}
      >
        {isGenerating
          ? $t('letters.generating')
          : editableContent
            ? $t('letters.regenerate')
            : $t('letters.generate')}
      </Button>
      {#if editableContent}
        <Button variant="primary" onclick={handleSave} disabled={isSaving || isGenerating}>
          {isSaving ? $t('common.loading') : $t('common.save')}
        </Button>
      {/if}
    </StickyActionBar>
  {/if}
</PageShell>
