<script lang="ts">
  import { onMount } from 'svelte';
  import { page } from '$app/stores';
  import { goto } from '$app/navigation';
  import {
    getPatient,
    updatePatient,
    deletePatient,
    exportPatientPdf,
    listScoresForPatient,
    type Patient,
    type UpdatePatient,
    type OutcomeScore,
  } from '$lib/api';
  import PatientForm from '$lib/components/PatientForm.svelte';
  import OutcomeScoreTrendChart from '$lib/components/OutcomeScoreTrendChart.svelte';
  import { t } from '$lib/translations';
  import { ChevronDown, ChevronUp } from 'lucide-svelte';

  let patient = $state<Patient | null>(null);
  let isLoading = $state(true);
  let isEditing = $state(false);
  let isSubmitting = $state(false);
  let isDeleting = $state(false);
  let isExporting = $state(false);
  let showDeleteConfirm = $state(false);
  let error = $state('');

  let outcomeScores = $state<OutcomeScore[]>([]);
  let isLoadingScores = $state(false);
  let showTrendChart = $state(true);

  let patientId = $derived($page.params.id);

  onMount(async () => {
    await loadPatient();
    await loadOutcomeScores();
  });

  async function loadPatient() {
    if (!patientId) {
      error = 'No patient ID provided';
      isLoading = false;
      return;
    }

    try {
      isLoading = true;
      error = '';
      patient = await getPatient(patientId);
    } catch (e) {
      error = e instanceof Error ? e.message : 'Failed to load patient';
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

  async function handleUpdate(event: CustomEvent<{ id: string; data: UpdatePatient }>) {
    try {
      isSubmitting = true;
      error = '';
      patient = await updatePatient(event.detail.id, event.detail.data);
      isEditing = false;
    } catch (e) {
      error = e instanceof Error ? e.message : 'Failed to update patient';
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
      error = e instanceof Error ? e.message : 'Failed to delete patient';
      console.error('Error deleting patient:', e);
      isDeleting = false;
      showDeleteConfirm = false;
    }
  }

  function handleCancelEdit() {
    isEditing = false;
  }

  async function handleExportPdf() {
    if (!patientId || !patient) return;

    try {
      isExporting = true;
      error = "";
      const bytes = await exportPatientPdf(patientId);
      const uint8Array = new Uint8Array(bytes);
      const blob = new Blob([uint8Array], { type: "application/pdf" });
      const url = URL.createObjectURL(blob);
      const a = document.createElement("a");
      a.href = url;
      a.download = `Patient_${patient.last_name}_${patient.first_name}_${new Date().toISOString().split("T")[0]}.pdf`;
      document.body.appendChild(a);
      a.click();
      document.body.removeChild(a);
      URL.revokeObjectURL(url);
    } catch (e) {
      error = e instanceof Error ? e.message : "Failed to export patient summary";
      console.error("Error exporting patient summary:", e);
    } finally {
      isExporting = false;
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

<div class="p-8">
  <div class="max-w-4xl mx-auto">
    {#if isLoading}
      <div class="flex justify-center items-center py-12">
        <div class="text-gray-500 dark:text-gray-400">{$t('patients.loadingDetails')}</div>
      </div>
    {:else if error}
      <div
        class="bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 rounded-lg p-4 text-red-600 dark:text-red-400 mb-6"
      >
        {error}
      </div>
    {:else if patient}
      <!-- Edit Mode -->
      {#if isEditing}
        <div class="bg-white dark:bg-gray-800 rounded-lg p-6">
          <PatientForm
            {patient}
            on:submit={handleUpdate}
            on:cancel={handleCancelEdit}
            {isSubmitting}
          />
        </div>
      {:else}
        <!-- View Mode -->
        <div class="bg-white dark:bg-gray-800 rounded-lg p-6">
          <!-- Action Buttons -->
          <div class="flex justify-between mb-6">
            <div class="flex gap-3">
              <button
                onclick={() => (isEditing = true)}
                class="px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 transition-colors"
              >
                {$t('patients.editPatient')}
              </button>
              <a
                href={`/patients/${patientId}/email/new`}
                class="px-4 py-2 bg-green-600 text-white rounded-lg hover:bg-green-700 transition-colors inline-flex items-center"
              >
                {$t('patients.sendEmail')}
              </a>
              <button
                onclick={handleExportPdf}
                disabled={isExporting}
                class="px-4 py-2 bg-purple-600 text-white rounded-lg hover:bg-purple-700 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
              >
                {isExporting ? "Exporting..." : "Export PDF"}
              </button>
            </div>
            <button
              onclick={() => (showDeleteConfirm = true)}
              class="px-4 py-2 bg-red-600 text-white rounded-lg hover:bg-red-700 transition-colors"
            >
              {$t('patients.deletePatient')}
            </button>
          </div>

          <!-- Patient Details -->
          <div class="space-y-6">
            <!-- Outcome Score Trend Visualization -->
            {#if outcomeScores.length > 0}
              <div class="border-t border-gray-200 dark:border-gray-700 pt-6">
                <button
                  onclick={() => (showTrendChart = !showTrendChart)}
                  class="flex items-center justify-between w-full mb-4 hover:text-blue-600 dark:hover:text-blue-400 transition-colors"
                >
                  <h3 class="text-lg font-semibold text-gray-900 dark:text-gray-100">
                    Outcome Score Trends
                  </h3>
                  {#if showTrendChart}
                    <ChevronUp class="w-5 h-5" />
                  {:else}
                    <ChevronDown class="w-5 h-5" />
                  {/if}
                </button>

                {#if showTrendChart}
                  <div class="space-y-6">
                    {#each ['PHQ-9', 'GAD-7', 'BDI-II'] as scaleType}
                      {@const scoresForScale = outcomeScores.filter((s) => s.scale_type === scaleType)}
                      {#if scoresForScale.length > 0}
                        <div class="bg-gray-50 dark:bg-gray-900/50 rounded-lg p-4">
                          <h4 class="text-sm font-semibold text-gray-700 dark:text-gray-300 mb-3">
                            {scaleType}
                            <span class="text-xs font-normal text-gray-500 dark:text-gray-400">
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
            <div class="grid grid-cols-2 gap-6">
              <div>
                <span class="block text-sm font-medium text-gray-500 dark:text-gray-400 mb-1"
                  >{$t('patients.firstName')}</span
                >
                <p class="text-gray-900 dark:text-gray-100">{patient.first_name}</p>
              </div>
              <div>
                <span class="block text-sm font-medium text-gray-500 dark:text-gray-400 mb-1"
                  >{$t('patients.lastName')}</span
                >
                <p class="text-gray-900 dark:text-gray-100">{patient.last_name}</p>
              </div>
            </div>

            <div class="grid grid-cols-2 gap-6">
              <div>
                <span class="block text-sm font-medium text-gray-500 dark:text-gray-400 mb-1"
                  >{$t('patients.ahvNumber')}</span
                >
                <p class="text-gray-900 dark:text-gray-100">{patient.ahv_number}</p>
              </div>
              <div>
                <span class="block text-sm font-medium text-gray-500 dark:text-gray-400 mb-1"
                  >{$t('patients.dateOfBirth')}</span
                >
                <p class="text-gray-900 dark:text-gray-100">{formatDate(patient.date_of_birth)}</p>
              </div>
            </div>

            {#if patient.gender}
              <div>
                <span class="block text-sm font-medium text-gray-500 dark:text-gray-400 mb-1"
                  >{$t('patients.gender')}</span
                >
                <p class="text-gray-900 dark:text-gray-100 capitalize">{patient.gender}</p>
              </div>
            {/if}

            <!-- Contact Info -->
            {#if patient.phone || patient.email}
              <div class="border-t border-gray-200 dark:border-gray-700 pt-6">
                <h3 class="text-lg font-semibold text-gray-900 dark:text-gray-100 mb-4">
                  {$t('patients.contactInfo')}
                </h3>
                <div class="grid grid-cols-2 gap-6">
                  {#if patient.phone}
                    <div>
                      <span class="block text-sm font-medium text-gray-500 dark:text-gray-400 mb-1"
                        >{$t('patients.phone')}</span
                      >
                      <p class="text-gray-900 dark:text-gray-100">{patient.phone}</p>
                    </div>
                  {/if}
                  {#if patient.email}
                    <div>
                      <span class="block text-sm font-medium text-gray-500 dark:text-gray-400 mb-1"
                        >{$t('patients.email')}</span
                      >
                      <p class="text-gray-900 dark:text-gray-100">{patient.email}</p>
                    </div>
                  {/if}
                </div>
              </div>
            {/if}

            {#if patient.address}
              <div>
                <span class="block text-sm font-medium text-gray-500 dark:text-gray-400 mb-1"
                  >{$t('patients.address')}</span
                >
                <p class="text-gray-900 dark:text-gray-100 whitespace-pre-line">
                  {patient.address}
                </p>
              </div>
            {/if}

            <!-- Insurance & GP -->
            {#if patient.insurance || patient.gp_name || patient.gp_address}
              <div class="border-t border-gray-200 dark:border-gray-700 pt-6">
                <h3 class="text-lg font-semibold text-gray-900 dark:text-gray-100 mb-4">
                  {$t('patients.medicalInfo')}
                </h3>

                {#if patient.insurance}
                  <div class="mb-4">
                    <span class="block text-sm font-medium text-gray-500 dark:text-gray-400 mb-1"
                      >{$t('patients.insurance')}</span
                    >
                    <p class="text-gray-900 dark:text-gray-100">{patient.insurance}</p>
                  </div>
                {/if}

                {#if patient.gp_name || patient.gp_address}
                  <div class="grid grid-cols-2 gap-6">
                    {#if patient.gp_name}
                      <div>
                        <span
                          class="block text-sm font-medium text-gray-500 dark:text-gray-400 mb-1"
                          >{$t('patients.gpName')}</span
                        >
                        <p class="text-gray-900 dark:text-gray-100">{patient.gp_name}</p>
                      </div>
                    {/if}
                    {#if patient.gp_address}
                      <div>
                        <span
                          class="block text-sm font-medium text-gray-500 dark:text-gray-400 mb-1"
                          >{$t('patients.gpAddress')}</span
                        >
                        <p class="text-gray-900 dark:text-gray-100">{patient.gp_address}</p>
                      </div>
                    {/if}
                  </div>
                {/if}
              </div>
            {/if}

            <!-- Notes -->
            {#if patient.notes}
              <div class="border-t border-gray-200 dark:border-gray-700 pt-6">
                <span class="block text-sm font-medium text-gray-500 dark:text-gray-400 mb-1"
                  >{$t('patients.notes')}</span
                >
                <p class="text-gray-900 dark:text-gray-100 whitespace-pre-line">{patient.notes}</p>
              </div>
            {/if}

            <!-- Metadata -->
            <div class="border-t border-gray-200 dark:border-gray-700 pt-6 text-sm text-gray-500">
              <div class="grid grid-cols-2 gap-4">
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
            class="bg-white dark:bg-gray-800 rounded-lg p-6 max-w-md w-full mx-4"
            role="dialog"
            aria-modal="true"
            aria-labelledby="delete-dialog-title"
            tabindex="-1"
            onclick={(e) => e.stopPropagation()}
            onkeydown={(e) => e.key === 'Escape' && (showDeleteConfirm = false)}
          >
            <h2
              id="delete-dialog-title"
              class="text-xl font-bold text-gray-900 dark:text-gray-100 mb-4"
            >
              {$t('patients.deletePatient')}
            </h2>
            <p class="text-gray-600 dark:text-gray-300 mb-6">
              {$t('patients.confirmDeleteText').replace(
                '{name}',
                `${patient.first_name} ${patient.last_name}`
              )}
            </p>
            <div class="flex gap-4 justify-end">
              <button
                onclick={() => (showDeleteConfirm = false)}
                disabled={isDeleting}
                class="px-4 py-2 border border-gray-300 dark:border-gray-600 rounded-lg text-gray-600 dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-700 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
              >
                {$t('common.cancel')}
              </button>
              <button
                onclick={handleDelete}
                disabled={isDeleting}
                class="px-4 py-2 bg-red-600 text-white rounded-lg hover:bg-red-700 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
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
