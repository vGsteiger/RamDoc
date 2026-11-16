<script lang="ts">
  import { reportTypeLabel } from '$lib/translations/labels';
  import { page } from '$app/state';
  import { onMount } from 'svelte';
  import { listReports, deleteReport, parseError, type Report, type AppError } from '$lib/api';
  import ErrorDisplay from '$lib/components/ErrorDisplay.svelte';
  import { t } from '$lib/translations';

  let patientId = $derived(page.params.id!);
  let reports: Report[] = $state([]);
  let loading = $state(true);
  let error: AppError | null = $state(null);

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

  onMount(() => {
    loadReports();
  });
</script>

<div class="p-8">
  <div class="flex justify-between items-center mb-6">
    <h2 class="text-display font-semibold text-fg">{$t('reports.title')}</h2>
    <a
      href={`/patients/${patientId}/reports/new`}
      class="inline-flex items-center h-8 px-3 bg-accent text-on-accent rounded-card hover:bg-accent-hover transition-colors"
    >
      {$t('reports.generateNew')}
    </a>
  </div>

  {#if loading}
    <div class="text-fg-muted">{$t('reports.loading')}</div>
  {:else if error}
    <ErrorDisplay {error} showDetails={true} />
  {:else if reports.length === 0}
    <div class="text-center py-12">
      <p class="text-fg-muted mb-4">{$t('reports.noReportsYet')}</p>
      <a
        href={`/patients/${patientId}/reports/new`}
        class="inline-flex items-center inline-block h-8 px-3 bg-accent text-on-accent rounded-card hover:bg-accent-hover transition-colors"
      >
        {$t('reports.generateFirst')}
      </a>
    </div>
  {:else}
    <div class="space-y-4">
      {#each reports as report}
        <div class="bg-surface-raised rounded-card p-6 border border-line">
          <div class="flex justify-between items-start mb-3">
            <div>
              <h3 class="text-heading font-semibold text-fg">
                {$reportTypeLabel(report.report_type)}
              </h3>
              <p class="text-body text-fg-muted mt-1">
                {$t('reports.generated')}
                {formatDate(report.generated_at)}
              </p>
              {#if report.model_name}
                <p class="text-caption text-fg-muted mt-1">
                  {$t('reports.model')}
                  {report.model_name}
                </p>
              {/if}
            </div>
            <div class="flex space-x-2">
              <a
                href={`/patients/${patientId}/reports/${report.id}`}
                class="inline-flex items-center h-7 px-2.5 text-body bg-surface-selected text-fg-muted rounded-card hover:bg-surface-selected transition-colors"
              >
                {$t('reports.view')}
              </a>
              <button
                onclick={() => handleDeleteReport(report.id)}
                class="h-7 px-2.5 text-body bg-danger-subtle text-danger-fg rounded-control hover:bg-danger-subtle/40 transition-colors"
              >
                {$t('common.delete')}
              </button>
            </div>
          </div>
          <div class="text-body text-fg-muted line-clamp-3">
            {report.content.substring(0, 300)}...
          </div>
        </div>
      {/each}
    </div>
  {/if}
</div>
