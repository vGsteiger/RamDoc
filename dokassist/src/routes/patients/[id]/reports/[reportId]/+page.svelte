<script lang="ts">
  import { page } from '$app/stores';
  import { goto } from '$app/navigation';
  import { onMount } from 'svelte';
  import {
    getReport,
    updateReport,
    deleteReport,
    exportReportToPdf,
    exportReportToDocx,
    parseError,
    type Report,
    type UpdateReport,
    type AppError,
  } from '$lib/api';
  import EnhancedReportEditor from '$lib/components/EnhancedReportEditor.svelte';
  import ErrorDisplay from '$lib/components/ErrorDisplay.svelte';
  import { t } from '$lib/translations';
  import { get } from 'svelte/store';
  import { marked } from 'marked';

  $: patientId = $page.params.id;
  $: reportId = $page.params.reportId;

  let report: Report | null = null;
  let editMode = false;
  let editableContent = '';
  let loading = true;
  let error: AppError | null = null;

  async function loadReport() {
    try {
      loading = true;
      error = null;
      report = await getReport(reportId);
      editableContent = report.content;
    } catch (e) {
      error = parseError(e);
    } finally {
      loading = false;
    }
  }

  async function saveChanges() {
    if (!report) return;

    try {
      error = null;
      const input: UpdateReport = { content: editableContent };
      await updateReport(reportId, input);
      report.content = editableContent;
      editMode = false;
    } catch (e) {
      error = parseError(e);
    }
  }

  async function handleDeleteReport() {
    if (!confirm(get(t)('reports.confirmDelete'))) {
      return;
    }
    try {
      await deleteReport(reportId);
      await goto(`/patients/${patientId}/reports`);
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

  async function handleExportPdf() {
    if (!report) return;

    try {
      error = null;
      const bytes = await exportReportToPdf(reportId);

      // Convert number[] to Uint8Array
      const uint8Array = new Uint8Array(bytes);

      // Create a blob from the byte array
      const blob = new Blob([uint8Array], { type: 'application/pdf' });

      // Create a download link
      const url = URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = url;
      a.download = `${formatReportType(report.report_type)}_${new Date(report.generated_at).toISOString().split('T')[0]}.pdf`;
      document.body.appendChild(a);
      a.click();
      document.body.removeChild(a);
      URL.revokeObjectURL(url);
    } catch (e) {
      error = parseError(e);
    }
  }

  async function handleExportDocx() {
    if (!report) return;

    try {
      error = null;
      const bytes = await exportReportToDocx(reportId);

      // Convert number[] to Uint8Array
      const uint8Array = new Uint8Array(bytes);

      // Create a blob from the byte array
      const blob = new Blob([uint8Array], {
        type: 'application/vnd.openxmlformats-officedocument.wordprocessingml.document',
      });

      // Create a download link
      const url = URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = url;
      a.download = `${formatReportType(report.report_type)}_${new Date(report.generated_at).toISOString().split('T')[0]}.docx`;
      document.body.appendChild(a);
      a.click();
      document.body.removeChild(a);
      URL.revokeObjectURL(url);
    } catch (e) {
      error = parseError(e);
    }
  }

  onMount(() => {
    loadReport();
  });
</script>

<div class="p-8">
  <div class="max-w-5xl mx-auto">
    {#if loading}
      <div class="text-gray-500 dark:text-gray-400">{$t('reports.loading')}</div>
    {:else if error}
      <ErrorDisplay {error} showDetails={true} />
    {:else if report}
      <div class="mb-6">
        <div class="flex items-center justify-between mb-4">
          <div>
            <h2 class="text-2xl font-bold text-gray-900 dark:text-gray-100">
              {formatReportType(report.report_type)}
            </h2>
            <p class="text-sm text-gray-500 dark:text-gray-400 mt-1">
              {$t('reports.generated')} {formatDate(report.generated_at)}
            </p>
            {#if report.model_name}
              <p class="text-xs text-gray-400 dark:text-gray-500 mt-1">{$t('reports.model')} {report.model_name}</p>
            {/if}
          </div>
          <a
            href={`/patients/${patientId}/reports`}
            class="text-sm text-gray-500 dark:text-gray-400 hover:text-gray-700 dark:hover:text-gray-300"
          >
            {$t('reports.backToReports')}
          </a>
        </div>

        <div class="flex space-x-4 mb-6">
          {#if !editMode}
            <button
              on:click={() => (editMode = true)}
              class="px-4 py-2 bg-blue-600 text-white rounded hover:bg-blue-700 transition-colors"
            >
              {$t('reports.edit')}
            </button>
          {:else}
            <button
              on:click={saveChanges}
              class="px-4 py-2 bg-green-600 text-white rounded hover:bg-green-700 transition-colors"
            >
              {$t('reports.save')}
            </button>
            <button
              on:click={() => {
                editMode = false;
                editableContent = report.content;
              }}
              class="px-4 py-2 bg-gray-100 dark:bg-gray-700 text-gray-700 dark:text-gray-300 rounded hover:bg-gray-200 dark:hover:bg-gray-600 transition-colors"
            >
              {$t('reports.cancel')}
            </button>
          {/if}
          <button
            on:click={handleExportPdf}
            disabled={editMode}
            class="px-4 py-2 bg-blue-600 text-white rounded hover:bg-blue-700 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
          >
            {$t('reports.exportPDF')}
          </button>
          <button
            on:click={handleExportDocx}
            disabled={editMode}
            class="px-4 py-2 bg-blue-600 text-white rounded hover:bg-blue-700 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
          >
            {$t('reports.exportDOCX')}
          </button>
          <button
            on:click={handleDeleteReport}
            class="px-4 py-2 bg-red-900/20 text-red-400 rounded hover:bg-red-900/40 transition-colors"
          >
            {$t('reports.delete')}
          </button>
        </div>
      </div>

      {#if editMode}
        <div class="h-[600px]">
          <EnhancedReportEditor bind:content={editableContent} />
        </div>
      {:else}
        <div
          class="bg-white dark:bg-gray-800 rounded-lg p-6 border border-gray-200 dark:border-gray-700"
        >
          <div class="prose prose-gray dark:prose-invert max-w-none">
            {@html marked(report.content)}
          </div>
        </div>
      {/if}
    {/if}
  </div>
</div>
