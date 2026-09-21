<script lang="ts">
  import { errorText } from '$lib/translations/labels';
  import { get } from 'svelte/store';
  import { onMount } from 'svelte';
  import { page } from '$app/stores';
  import { goto } from '$app/navigation';
  import { save } from '@tauri-apps/plugin-dialog';
  import {
    getPatient,
    updatePatient,
    deletePatient,
    exportFhirBundle,
    exportPatientPdf,
    listScoresForPatient,
    type Patient,
    type UpdatePatient,
    type CreatePatient,
    type OutcomeScore,
  } from '$lib/api';
  import PatientForm from '$lib/components/PatientForm.svelte';
  import OutcomeScoreTrendChart from '$lib/components/OutcomeScoreTrendChart.svelte';
  import PatientHistoryQuery from '$lib/components/PatientHistoryQuery.svelte';
  import { t } from '$lib/translations';
  import { ChevronDown, ChevronUp } from 'lucide-svelte';

  let patient = $state<Patient | null>(null);
  let isLoading = $state(true);
  let isEditing = $state(false);
  let isSubmitting = $state(false);
  let isDeleting = $state(false);
  let isExporting = $state(false);
  let showDeleteConfirm = $state(false);
  let showExportMenu = $state(false);
  let error = $state('');

  let outcomeScores = $state<OutcomeScore[]>([]);
  let isLoadingScores = $state(false);
  let showTrendChart = $state(true);

  let patientId = $derived($page.params.id!);

  onMount(async () => {
    await loadPatient();
    await loadOutcomeScores();
  });

  async function loadPatient() {
    if (!patientId) {
      error = get(t)('patients.noPatientId');
      isLoading = false;
      return;
    }

    try {
      isLoading = true;
      error = '';
      patient = await getPatient(patientId);
    } catch (e) {
      error = $errorText(e, get(t)('patients.loadFailed'));
      console.error('Error loading patient:', e);
    } finally {
      isLoading = false;
    }
  }

  async function loadOutcomeScores() {
    if (!patientId) return;

    try {
      isLoadingScores = true;
      outcomeScores = await listScoresForPatient(patientId);
    } catch (e) {
      console.error('Error loading outcome scores:', e);
    } finally {
      isLoadingScores = false;
    }
  }

  async function handleUpdate(
    event: CustomEvent<CreatePatient | { id: string; data: UpdatePatient }>
  ) {
    if (!('id' in event.detail)) return;
    try {
      isSubmitting = true;
      error = '';
      patient = await updatePatient(event.detail.id, event.detail.data);
      isEditing = false;
    } catch (e) {
      error = $errorText(e, get(t)('patients.updateFailed'));
      console.error('Error updating patient:', e);
    } finally {
      isSubmitting = false;
    }
  }

  async function handleDelete() {
    if (!patientId) return;

    try {
      isDeleting = true;
      error = '';
      await deletePatient(patientId);
      goto('/patients');
    } catch (e) {
      error = $errorText(e, get(t)('patients.deleteFailed'));
      console.error('Error deleting patient:', e);
      isDeleting = false;
      showDeleteConfirm = false;
    }
  }

  function handleCancelEdit() {
    isEditing = false;
  }

  async function handleExportFhir() {
    if (!patientId || !patient) return;

    try {
      isExporting = true;
      showExportMenu = false;
      error = '';

      // Get FHIR bundle JSON
      const fhirJson = await exportFhirBundle(patientId);

      // Prompt user to save file
      const fileName = `FHIR_${patient.last_name}_${patient.first_name}_${new Date().toISOString().split('T')[0]}.json`;
      const filePath = await save({
        defaultPath: fileName,
        filters: [
          {
            name: 'FHIR Bundle',
            extensions: ['json'],
          },
        ],
      });

      if (filePath) {
        // Write the file
        const { writeTextFile } = await import('@tauri-apps/plugin-fs');
        await writeTextFile(filePath, fhirJson);
      }
    } catch (e) {
      error = $errorText(e, get(t)('patients.exportFhirFailed'));
      console.error('Error exporting FHIR bundle:', e);
    } finally {
      isExporting = false;
    }
  }

  async function handleExportPdf() {
    if (!patientId || !patient) return;

    try {
      isExporting = true;
      error = '';
      const bytes = await exportPatientPdf(patientId);
      const uint8Array = new Uint8Array(bytes);
      const blob = new Blob([uint8Array], { type: 'application/pdf' });
      const url = URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = url;
      a.download = `Patient_${patient.last_name}_${patient.first_name}_${new Date().toISOString().split('T')[0]}.pdf`;
      document.body.appendChild(a);
      a.click();
      document.body.removeChild(a);
      URL.revokeObjectURL(url);
    } catch (e) {
      error = $errorText(e, get(t)('patients.exportSummaryFailed'));
      console.error('Error exporting patient summary:', e);
    } finally {
      isExporting = false;
    }
  }

  function handleClickOutside(event: MouseEvent) {
    const target = event.target as HTMLElement;
    if (!target.closest('.export-dropdown')) {
      showExportMenu = false;
    }
  }

  function formatDate(dateStr: string): string {
    try {
      const date = new Date(dateStr);
      return date.toLocaleDateString('de-CH', {
        year: 'numeric',
        month: '2-digit',
        day: '2-digit',
      });
    } catch {
      return dateStr;
    }
  }
</script>

<svelte:window onclick={handleClickOutside} />

<div class="p-4 sm:p-6 lg:p-8">
  <div class="max-w-4xl mx-auto">
    {#if isLoading}
      <div class="flex justify-center items-center py-12">
        <div class="text-fg-muted">{$t('patients.loadingDetails')}</div>
      </div>
    {:else if error}
      <div class="bg-danger-subtle border border-danger-line rounded-card p-4 text-danger-fg mb-6">
        {error}
      </div>
    {:else if patient}
      <!-- Edit Mode -->
      {#if isEditing}
        <div class="bg-surface-raised rounded-card p-4 sm:p-6">
          <PatientForm
            {patient}
            on:submit={handleUpdate}
            on:cancel={handleCancelEdit}
            {isSubmitting}
          />
        </div>
      {:else}
        <!-- View Mode -->
        <div class="bg-surface-raised rounded-card p-4 sm:p-6">
          <!-- Action Buttons -->
          <div class="mb-6 flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
            <div class="flex flex-wrap gap-2">
              <button
                onclick={() => (isEditing = true)}
                class="min-h-10 px-3.5 bg-accent text-on-accent rounded-control hover:bg-accent-hover transition-colors"
              >
                {$t('patients.editPatient')}
              </button>
              <a
                href={`/patients/${patientId}/email/new`}
                class="min-h-10 px-3.5 bg-surface-hover text-fg rounded-control hover:bg-surface-selected transition-colors inline-flex items-center"
              >
                {$t('patients.sendEmail')}
              </a>
              <!-- Export Dropdown -->
              <div class="relative export-dropdown">
                <button
                  onclick={() => (showExportMenu = !showExportMenu)}
                  disabled={isExporting}
                  class="min-h-10 px-3.5 border border-line bg-surface-raised text-fg rounded-control hover:bg-surface-hover transition-colors disabled:opacity-50 disabled:cursor-not-allowed inline-flex items-center gap-2"
                  aria-expanded={showExportMenu}
                  aria-haspopup="menu"
                >
                  {isExporting ? $t('patients.exporting') : $t('common.export')}
                  <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path
                      stroke-linecap="round"
                      stroke-linejoin="round"
                      stroke-width="2"
                      d="M19 9l-7 7-7-7"
                    />
                  </svg>
                </button>
                {#if showExportMenu}
                  <div
                    class="absolute left-0 mt-2 w-48 bg-surface-raised rounded-card shadow-popover border border-line z-10"
                    role="menu"
                  >
                    <button
                      onclick={handleExportFhir}
                      class="w-full min-h-10 text-left px-3 hover:bg-surface-hover rounded-control transition-colors"
                      role="menuitem"
                    >
                      {$t('patients.exportFhir')}
                    </button>
                    <button
                      onclick={handleExportPdf}
                      disabled={isExporting}
                      class="w-full min-h-10 px-3 text-left bg-surface-raised text-fg rounded-control hover:bg-surface-hover transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
                      role="menuitem"
                    >
                      {isExporting ? $t('patients.exporting') : $t('patients.exportPdf')}
                    </button>
                  </div>
                {/if}
              </div>
            </div>
            <button
              onclick={() => (showDeleteConfirm = true)}
              class="min-h-10 px-3.5 bg-danger text-on-danger rounded-control hover:bg-danger-hover transition-colors self-start sm:self-auto"
            >
              {$t('patients.deletePatient')}
            </button>
          </div>

          <!-- Patient History Query Interface -->
          <section class="mb-6" aria-labelledby="patient-timeline-title">
            <div class="mb-3 border-b border-line-subtle pb-3">
              <h2 id="patient-timeline-title" class="text-title text-fg">
                {$t('patientHistory.timelineTitle')}
              </h2>
              <p class="mt-1 text-body text-fg-muted">{$t('patientHistory.timelineDescription')}</p>
            </div>
            <PatientHistoryQuery {patientId} />
          </section>

          <!-- Patient Details -->
          <div class="space-y-6">
            <!-- Outcome Score Trend Visualization -->
            {#if outcomeScores.length > 0}
              <div class="border-t border-line pt-6">
                <button
                  onclick={() => (showTrendChart = !showTrendChart)}
                  class="flex items-center justify-between w-full mb-4 hover:text-accent-fg transition-colors"
                >
                  <h3 class="text-heading font-semibold text-fg">{$t('outcomeScores.trends')}</h3>
                  {#if showTrendChart}
                    <ChevronUp class="w-5 h-5" />
                  {:else}
                    <ChevronDown class="w-5 h-5" />
                  {/if}
                </button>

                {#if showTrendChart}
                  <div class="space-y-6">
                    {#each ['PHQ-9', 'GAD-7', 'BDI-II'] as scaleType}
                      {@const scoresForScale = outcomeScores.filter(
                        (s) => s.scale_type === scaleType
                      )}
                      {#if scoresForScale.length > 0}
                        <div class="bg-surface-sunken rounded-card p-4">
                          <h4 class="text-body font-semibold text-fg-muted mb-3">
                            {scaleType}
                            <span class="text-caption font-normal text-fg-muted">
                              ({scoresForScale.length}
                              {scoresForScale.length === 1 ? 'measurement' : 'measurements'})
                            </span>
                          </h4>
                          <OutcomeScoreTrendChart scores={scoresForScale} {scaleType} />
                        </div>
                      {/if}
                    {/each}
                  </div>
                {/if}
              </div>
            {/if}

            <!-- Basic Info -->
            <div class="grid grid-cols-1 gap-4 sm:grid-cols-2 sm:gap-6">
              <div>
                <span class="block text-body font-medium text-fg-muted mb-1"
                  >{$t('patients.firstName')}</span
                >
                <p class="text-fg">{patient.first_name}</p>
              </div>
              <div>
                <span class="block text-body font-medium text-fg-muted mb-1"
                  >{$t('patients.lastName')}</span
                >
                <p class="text-fg">{patient.last_name}</p>
              </div>
            </div>

            <div class="grid grid-cols-1 gap-4 sm:grid-cols-2 sm:gap-6">
              <div>
                <span class="block text-body font-medium text-fg-muted mb-1"
                  >{$t('patients.ahvNumber')}</span
                >
                <p class="text-fg">{patient.ahv_number}</p>
              </div>
              <div>
                <span class="block text-body font-medium text-fg-muted mb-1"
                  >{$t('patients.dateOfBirth')}</span
                >
                <p class="text-fg">{formatDate(patient.date_of_birth)}</p>
              </div>
            </div>

            {#if patient.gender}
              <div>
                <span class="block text-body font-medium text-fg-muted mb-1"
                  >{$t('patients.gender')}</span
                >
                <p class="text-fg capitalize">{patient.gender}</p>
              </div>
            {/if}

            <!-- Contact Info -->
            {#if patient.phone || patient.email}
              <div class="border-t border-line pt-6">
                <h3 class="text-heading font-semibold text-fg mb-4">
                  {$t('patients.contactInfo')}
                </h3>
                <div class="grid grid-cols-1 gap-4 sm:grid-cols-2 sm:gap-6">
                  {#if patient.phone}
                    <div>
                      <span class="block text-body font-medium text-fg-muted mb-1"
                        >{$t('patients.phone')}</span
                      >
                      <p class="text-fg">{patient.phone}</p>
                    </div>
                  {/if}
                  {#if patient.email}
                    <div>
                      <span class="block text-body font-medium text-fg-muted mb-1"
                        >{$t('patients.email')}</span
                      >
                      <p class="text-fg">{patient.email}</p>
                    </div>
                  {/if}
                </div>
              </div>
            {/if}

            {#if patient.address}
              <div>
                <span class="block text-body font-medium text-fg-muted mb-1"
                  >{$t('patients.address')}</span
                >
                <p class="text-fg whitespace-pre-line">
                  {patient.address}
                </p>
              </div>
            {/if}

            <!-- Insurance & GP -->
            {#if patient.insurance || patient.gp_name || patient.gp_address}
              <div class="border-t border-line pt-6">
                <h3 class="text-heading font-semibold text-fg mb-4">
                  {$t('patients.medicalInfo')}
                </h3>

                {#if patient.insurance}
                  <div class="mb-4">
                    <span class="block text-body font-medium text-fg-muted mb-1"
                      >{$t('patients.insurance')}</span
                    >
                    <p class="text-fg">{patient.insurance}</p>
                  </div>
                {/if}

                {#if patient.gp_name || patient.gp_address}
                  <div class="grid grid-cols-1 gap-4 sm:grid-cols-2 sm:gap-6">
                    {#if patient.gp_name}
                      <div>
                        <span class="block text-body font-medium text-fg-muted mb-1"
                          >{$t('patients.gpName')}</span
                        >
                        <p class="text-fg">{patient.gp_name}</p>
                      </div>
                    {/if}
                    {#if patient.gp_address}
                      <div>
                        <span class="block text-body font-medium text-fg-muted mb-1"
                          >{$t('patients.gpAddress')}</span
                        >
                        <p class="text-fg">{patient.gp_address}</p>
                      </div>
                    {/if}
                  </div>
                {/if}
              </div>
            {/if}

            <!-- Notes -->
            {#if patient.notes}
              <div class="border-t border-line pt-6">
                <span class="block text-body font-medium text-fg-muted mb-1"
                  >{$t('patients.notes')}</span
                >
                <p class="text-fg whitespace-pre-line">{patient.notes}</p>
              </div>
            {/if}

            <!-- Metadata -->
            <div class="border-t border-line pt-6 text-body text-fg-muted">
              <div class="grid grid-cols-1 gap-2 sm:grid-cols-2 sm:gap-4">
                <div>{$t('patients.created')}: {formatDate(patient.created_at)}</div>
                <div>{$t('patients.lastUpdated')}: {formatDate(patient.updated_at)}</div>
              </div>
            </div>
          </div>
        </div>
      {/if}

      <!-- Delete Confirmation Modal -->
      {#if showDeleteConfirm}
        <div
          class="fixed inset-0 bg-black/50 flex items-center justify-center z-50"
          role="presentation"
          onclick={() => (showDeleteConfirm = false)}
          onkeydown={(e) => e.key === 'Escape' && (showDeleteConfirm = false)}
        >
          <div
            class="bg-surface-raised rounded-card p-6 max-w-md w-full mx-4"
            role="dialog"
            aria-modal="true"
            aria-labelledby="delete-dialog-title"
            tabindex="-1"
            onclick={(e) => e.stopPropagation()}
            onkeydown={(e) => e.key === 'Escape' && (showDeleteConfirm = false)}
          >
            <h2 id="delete-dialog-title" class="text-title font-semibold text-fg mb-4">
              {$t('patients.deletePatient')}
            </h2>
            <p class="text-fg-muted mb-6">
              {$t('patients.confirmDeleteText').replace(
                '{name}',
                `${patient.first_name} ${patient.last_name}`
              )}
            </p>
            <div class="flex gap-4 justify-end">
              <button
                onclick={() => (showDeleteConfirm = false)}
                disabled={isDeleting}
                class="h-8 px-3 border border-line rounded-control text-fg-muted hover:bg-surface-hover transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
              >
                {$t('common.cancel')}
              </button>
              <button
                onclick={handleDelete}
                disabled={isDeleting}
                class="h-8 px-3 bg-danger text-on-danger rounded-control hover:bg-danger-hover transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
              >
                {isDeleting ? $t('patients.deleting') : $t('patients.deletePatient')}
              </button>
            </div>
          </div>
        </div>
      {/if}
    {/if}
  </div>
</div>
