<script lang="ts">
  import { page } from '$app/stores';
  import { onMount } from 'svelte';
  import { listReports, deleteReport, parseError, type Report, type AppError } from '$lib/api';
  import ErrorDisplay from '$lib/components/ErrorDisplay.svelte';
  import { t } from '$lib/translations';

  $: patientId = $page.params.id;
  let reports: Report[] = [];
  let loading = true;
  let error: AppError | null = null;

  async function loadReports() {
    try {
      loading = true;
      error = null;
      reports = await listReports(patientId);
    } catch (e) {
      error = parseError(e);
    } finally {
      loading = false;
    }
  }

  async function handleDeleteReport(reportId: string) {
    if (!confirm($t('reports.confirmDelete'))) {
      return;
    }
    try {
      await deleteReport(reportId);
      await loadReports();
    } catch (e) {
      error = parseError(e);
    }
  }

  function formatDate(dateStr: string): string {
    return new Date(dateStr).toLocaleDateString('de-DE', {
      year: 'numeric',
      month: '2-digit',
      day: '2-digit',
      hour: '2-digit',
      minute: '2-digit',
    });
  }

  function formatReportType(type: string): string {
    switch (type) {
      case 'Befundbericht':
        return 'Befundbericht';
      case 'Verlaufsbericht':
        return 'Verlaufsbericht';
      case 'Ueberweisungsschreiben':
        return 'Überweisungsschreiben';
      default:
        return type;
    }
  }

  onMount(() => {
    loadReports();
  });
</script>

<div class="p-8">
  <div class="flex justify-between items-center mb-6">
    <h2 class="text-2xl font-bold text-gray-100">{$t('reports.title')}</h2>
    <a
      href={`/patients/${patientId}/reports/new`}
      class="px-4 py-2 bg-blue-600 text-white rounded hover:bg-blue-700 transition-colors"
    >
      {$t('reports.generateNew')}
    </a>
  </div>

  {#if loading}
    <div class="text-gray-400">{$t('reports.loading')}</div>
  {:else if error}
    <ErrorDisplay {error} showDetails={true} />
  {:else if reports.length === 0}
    <div class="text-center py-12">
      <p class="text-gray-400 mb-4">{$t('reports.noReportsYet')}</p>
      <a
        href={`/patients/${patientId}/reports/new`}
        class="inline-block px-4 py-2 bg-blue-600 text-white rounded hover:bg-blue-700 transition-colors"
      >
        {$t('reports.generateFirst')}
      </a>
    </div>
  {:else}
    <div class="space-y-4">
      {#each reports as report}
        <div
          class="bg-white dark:bg-gray-800 rounded-lg p-6 border border-gray-200 dark:border-gray-700"
        >
          <div class="flex justify-between items-start mb-3">
            <div>
              <h3 class="text-lg font-semibold text-gray-900 dark:text-gray-100">
                {formatReportType(report.report_type)}
              </h3>
              <p class="text-sm text-gray-400 mt-1">
                {$t('reports.generated')}
                {formatDate(report.generated_at)}
              </p>
              {#if report.model_name}
                <p class="text-xs text-gray-500 mt-1">{$t('reports.model')} {report.model_name}</p>
              {/if}
            </div>
            <div class="flex space-x-2">
              <a
                href={`/patients/${patientId}/reports/${report.id}`}
                class="px-3 py-1 text-sm bg-gray-700 text-gray-300 rounded hover:bg-gray-600 transition-colors"
              >
                {$t('reports.view')}
              </a>
              <button
                on:click={() => handleDeleteReport(report.id)}
                class="px-3 py-1 text-sm bg-red-900/20 text-red-400 rounded hover:bg-red-900/40 transition-colors"
              >
                {$t('common.delete')}
              </button>
            </div>
          </div>
          <div class="text-sm text-gray-400 line-clamp-3">
            {report.content.substring(0, 300)}...
          </div>
        </div>
      {/each}
    </div>
  {/if}
</div>
