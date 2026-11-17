<script lang="ts">
  import { reportTypeLabel } from '$lib/translations/labels';
  import { page } from '$app/state';
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

  let patientId = $derived(page.params.id!);
  let reportId = $derived(page.params.reportId!);

  let report: Report | null = $state(null);
  let editMode = $state(false);
  let editableContent = $state('');
  let loading = $state(true);
  let error: AppError | null = $state(null);

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
      a.download = `${report.report_type}_${new Date(report.generated_at).toISOString().split('T')[0]}.pdf`;
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
      a.download = `${report.report_type}_${new Date(report.generated_at).toISOString().split('T')[0]}.docx`;
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
      <div class="text-fg-muted">{$t('reports.loading')}</div>
    {:else if error}
      <ErrorDisplay {error} showDetails={true} />
    {:else if report}
      <div class="mb-6">
        <div class="flex items-center justify-between mb-4">
          <div>
            <h2 class="text-display font-semibold text-fg">
              {$reportTypeLabel(report.report_type)}
            </h2>
            <p class="text-body text-fg-muted mt-1">
              {$t('reports.generated')}
              {formatDate(report.generated_at)}
            </p>
            {#if report.model_name}
              <p class="text-caption text-fg-subtle mt-1">
                {$t('reports.model')}
                {report.model_name}
              </p>
            {/if}
          </div>
          <a href={`/patients/${patientId}/reports`} class="text-body text-fg-muted hover:text-fg">
            {$t('reports.backToReports')}
          </a>
        </div>

        <div class="flex space-x-4 mb-6">
          {#if !editMode}
            <button
              onclick={() => (editMode = true)}
              class="h-8 px-3 bg-accent text-on-accent rounded-control hover:bg-accent-hover transition-colors"
            >
              {$t('reports.edit')}
            </button>
          {:else}
            <button
              onclick={saveChanges}
              class="h-8 px-3 bg-success text-on-success rounded-control hover:bg-success-hover transition-colors"
            >
              {$t('reports.save')}
            </button>
            <button
              onclick={() => {
                editMode = false;
                editableContent = report?.content ?? '';
              }}
              class="h-8 px-3 bg-surface-hover text-fg-muted rounded-control hover:bg-surface-selected transition-colors"
            >
              {$t('reports.cancel')}
            </button>
          {/if}
          <button
            onclick={handleExportPdf}
            disabled={editMode}
            class="h-8 px-3 bg-accent text-on-accent rounded-control hover:bg-accent-hover transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
          >
            {$t('reports.exportPDF')}
          </button>
          <button
            onclick={handleExportDocx}
            disabled={editMode}
            class="h-8 px-3 bg-accent text-on-accent rounded-control hover:bg-accent-hover transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
          >
            {$t('reports.exportDOCX')}
          </button>
          <button
            onclick={handleDeleteReport}
            class="h-8 px-3 bg-danger-subtle text-danger-fg rounded-control hover:bg-danger-subtle/40 transition-colors"
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
        <div class="bg-surface-raised rounded-card p-6 border border-line">
          <div class="prose prose-gray dark:prose-invert max-w-none">
            {@html marked(report.content)}
          </div>
        </div>
      {/if}
    {/if}
  </div>
</div>
