<script lang="ts">
  import { page } from '$app/stores';
  import { goto } from '$app/navigation';
  import { t } from '$lib/translations';
  import { listen, type UnlistenFn } from '@tauri-apps/api/event';
  import { onDestroy, onMount } from 'svelte';
  import {
    createLetter,
    generateLetter,
    getPatient,
    listDiagnoses,
    listMedications,
    type LetterType,
    type LetterLanguage,
    type Patient,
    type Diagnosis,
    type Medication,
  } from '$lib/api';
  import { language as appLanguage } from '$lib/stores/language';

  let patientId: string;
  $: patientId = $page.params.id;

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
  let isLoading = $state(true);
  let isSaving = $state(false);
  let error = $state<string | null>(null);

  let unlistenChunk: UnlistenFn | null = null;
  let unlistenDone: UnlistenFn | null = null;

  onMount(async () => {
    try {
      patient = await getPatient(patientId);
      diagnoses = await listDiagnoses(patientId, 10, 0);
      medications = await listMedications(patientId, 10, 0);

      // Auto-fill patient context
      patientContext = formatPatientContext();

      // Set default language based on app language
      letterLanguage = $appLanguage === 'fr' ? 'fr' : 'de'; // Only de/fr supported for letters

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
      diagnoses.forEach(d => {
        lines.push(`- ${d.icd10_code}: ${d.description} (${d.status})`);
      });
    }

    if (medications.length > 0) {
      lines.push('\nAktuelle Medikation:');
      medications.forEach(m => {
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
    generatedContent = '';
    error = null;

    try {
      unlistenChunk = await listen<string>('letter-chunk', (event) => {
        generatedContent += event.payload;
      });

      unlistenDone = await listen('letter-done', () => {
        isGenerating = false;
        editableContent = generatedContent;
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
        recipientName || undefined
      );
    } catch (err) {
      error = String(err);
      isGenerating = false;
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

      goto(`/patients/${patientId}`);
    } catch (err) {
      error = String(err);
      isSaving = false;
    }
  }

  function handleCancel() {
    goto(`/patients/${patientId}`);
  }
</script>

<div class="max-w-4xl mx-auto p-6">
  <div class="mb-6">
    <button
      onclick={handleCancel}
      class="text-blue-600 hover:text-blue-800 mb-4"
    >
      {$t('letters.backToLetters')}
    </button>
    <h1 class="text-3xl font-bold text-gray-800">{$t('letters.newLetterTitle')}</h1>
  </div>

  {#if isLoading}
    <div class="text-center py-8">
      <p class="text-gray-600">{$t('letters.loading')}</p>
    </div>
  {:else if error}
    <div class="bg-red-50 border border-red-200 rounded-lg p-4 mb-6">
      <p class="text-red-800">{error}</p>
    </div>
  {:else}
    <div class="space-y-6">
      <!-- Letter Type Selection -->
      <div>
        <label class="block text-sm font-medium text-gray-700 mb-2">
          {$t('letters.selectType')}
        </label>
        <select
          bind:value={letterType}
          class="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500"
        >
          <option value="referral">{$t('letters.types.referral')}</option>
          <option value="insurance_authorization">{$t('letters.types.insurance_authorization')}</option>
          <option value="therapy_extension">{$t('letters.types.therapy_extension')}</option>
        </select>
        <p class="text-sm text-gray-500 mt-1">
          {$t(`letters.typeDescriptions.${letterType}`)}
        </p>
      </div>

      <!-- Language Selection -->
      <div>
        <label class="block text-sm font-medium text-gray-700 mb-2">
          {$t('letters.selectLanguage')}
        </label>
        <select
          bind:value={letterLanguage}
          class="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500"
        >
          <option value="de">Deutsch</option>
          <option value="fr">Français</option>
        </select>
      </div>

      <!-- Recipient Information -->
      <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
        <div>
          <label class="block text-sm font-medium text-gray-700 mb-2">
            {$t('letters.recipientName')}
          </label>
          <input
            type="text"
            bind:value={recipientName}
            placeholder={$t('letters.recipientNamePlaceholder')}
            class="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500"
          />
        </div>

        <div>
          <label class="block text-sm font-medium text-gray-700 mb-2">
            {$t('letters.subject')}
          </label>
          <input
            type="text"
            bind:value={subject}
            placeholder={$t('letters.subjectPlaceholder')}
            class="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500"
          />
        </div>
      </div>

      <div>
        <label class="block text-sm font-medium text-gray-700 mb-2">
          {$t('letters.recipientAddress')}
        </label>
        <textarea
          bind:value={recipientAddress}
          placeholder={$t('letters.recipientAddressPlaceholder')}
          rows="2"
          class="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500"
        ></textarea>
      </div>

      <!-- Patient Context -->
      <div>
        <label class="block text-sm font-medium text-gray-700 mb-2">
          {$t('letters.patientContext')} <span class="text-gray-500 text-xs">{$t('letters.patientContextHint')}</span>
        </label>
        <textarea
          bind:value={patientContext}
          rows="6"
          class="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 font-mono text-sm"
        ></textarea>
      </div>

      <!-- Clinical Summary -->
      <div>
        <label class="block text-sm font-medium text-gray-700 mb-2">
          {$t('letters.clinicalSummary')}
        </label>
        <textarea
          bind:value={clinicalSummary}
          placeholder={$t('letters.clinicalSummaryPlaceholder')}
          rows="4"
          class="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500"
        ></textarea>
      </div>

      <!-- Generate Button -->
      <div class="flex gap-4">
        <button
          onclick={handleGenerate}
          disabled={isGenerating}
          class="px-6 py-3 bg-blue-600 text-white rounded-lg hover:bg-blue-700 disabled:bg-gray-400 disabled:cursor-not-allowed"
        >
          {isGenerating ? $t('letters.generating') : $t('letters.generate')}
        </button>

        {#if editableContent}
          <button
            onclick={handleGenerate}
            disabled={isGenerating}
            class="px-6 py-3 bg-gray-200 text-gray-700 rounded-lg hover:bg-gray-300 disabled:bg-gray-100 disabled:cursor-not-allowed"
          >
            {$t('letters.regenerate')}
          </button>
        {/if}
      </div>

      <!-- Generated Content -->
      {#if generatedContent || editableContent}
        <div>
          <label class="block text-sm font-medium text-gray-700 mb-2">
            {$t('letters.generatedLetter')}
            {#if isGenerating}
              <span class="text-blue-600 text-xs">({$t('letters.generating')})</span>
            {/if}
          </label>
          <p class="text-sm text-gray-500 mb-2">{$t('letters.reviewBeforeSaving')}</p>
          {#if isGenerating}
            <textarea
              value={generatedContent}
              rows="20"
              readonly
              class="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 font-mono text-sm bg-gray-50"
            ></textarea>
          {:else}
            <textarea
              bind:value={editableContent}
              rows="20"
              class="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 font-mono text-sm"
            ></textarea>
          {/if}
        </div>

        <!-- Save/Cancel Buttons -->
        <div class="flex gap-4">
          <button
            onclick={handleSave}
            disabled={isSaving}
            class="px-6 py-3 bg-green-600 text-white rounded-lg hover:bg-green-700 disabled:bg-gray-400 disabled:cursor-not-allowed"
          >
            {isSaving ? $t('common.loading') : $t('common.save')}
          </button>

          <button
            onclick={handleCancel}
            class="px-6 py-3 bg-gray-200 text-gray-700 rounded-lg hover:bg-gray-300"
          >
            {$t('common.cancel')}
          </button>
        </div>
      {/if}
    </div>
  {/if}
</div>
