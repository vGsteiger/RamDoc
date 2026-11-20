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
  import { fmt } from '$lib/format';
  import { ArrowLeft, Download, Pencil, Trash2 } from 'lucide-svelte';
  import {
    Button,
    ConfirmDialog,
    ListSkeleton,
    Markdown,
    Menu,
    PageShell,
    type MenuItem,
  } from '$lib/components/ui';
  import { documentsUrl, parseTimestamp } from '$lib/components/documents/documents';
  import StickyActionBar from '$lib/components/documents/StickyActionBar.svelte';

  let patientId = $derived(page.params.id!);
  let reportId = $derived(page.params.reportId!);

  let report = $state<Report | null>(null);
  let editMode = $state(false);
  let editableContent = $state('');
  let loading = $state(true);
  let error: AppError | null = $state(null);
  let confirmDeleteOpen = $state(false);

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
    try {
      await deleteReport(reportId);
      await goto(documentsUrl(patientId, 'reports'));
    } catch (e) {
      error = parseError(e);
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

  function cancelEdit() {
    editMode = false;
    editableContent = report?.content ?? '';
  }

  let generatedAt = $derived(report ? parseTimestamp(report.generated_at) : null);
  let typeLabel = $derived(report ? $reportTypeLabel(report.report_type) : '');

  let description = $derived(
    report
      ? [
          $t('reports.generatedOn').replace('{date}', $fmt.dateTime(generatedAt)),
          report.model_name ? $t('reports.modelName').replace('{name}', report.model_name) : '',
        ]
          .filter(Boolean)
          .join(' · ')
      : undefined
  );

  let moreItems = $derived<MenuItem[]>([
    {
      label: $t('reports.delete'),
      icon: Trash2,
      danger: true,
      onSelect: () => (confirmDeleteOpen = true),
    },
  ]);

  onMount(() => {
    loadReport();
  });
</script>

<PageShell width="prose" title={report ? typeLabel : undefined} {description}>
  {#snippet breadcrumb()}
    <a
      href={documentsUrl(patientId, 'reports')}
      class="inline-flex items-center gap-1 text-fg-subtle hover:text-fg"
    >
      <ArrowLeft size={12} aria-hidden="true" />
      {$t('reports.backToDocuments')}
    </a>
  {/snippet}

  {#if loading}
    <p class="sr-only">{$t('reports.loading')}</p>
    <ListSkeleton count={3} />
  {:else if error && !report}
    <ErrorDisplay {error} showDetails={true} />
  {:else if report}
    {#if error}
      <div class="mb-4">
        <ErrorDisplay {error} showDetails={true} />
      </div>
    {/if}

    {#if editMode}
      <EnhancedReportEditor bind:content={editableContent} />
    {:else}
      <article class="card-surface rounded-panel px-5 py-4 sm:px-8 sm:py-6">
        <Markdown source={report.content} headingOffset={1} />
      </article>
    {/if}

    <StickyActionBar>
      {#snippet start()}
        {#if !editMode}
          <Menu items={moreItems} size="md" label={$t('common.moreActions')} align="start" />
        {:else}
          <span class="text-label text-fg-muted">{$t('reports.unsavedHint')}</span>
        {/if}
      {/snippet}
      <Button onclick={handleExportPdf} disabled={editMode}>
        <Download size={14} aria-hidden="true" />
        {$t('reports.exportPDF')}
      </Button>
      <Button onclick={handleExportDocx} disabled={editMode}>
        <Download size={14} aria-hidden="true" />
        {$t('reports.exportDOCX')}
      </Button>
      {#if !editMode}
        <Button variant="primary" onclick={() => (editMode = true)}>
          <Pencil size={14} aria-hidden="true" />
          {$t('reports.edit')}
        </Button>
      {:else}
        <Button variant="ghost" onclick={cancelEdit}>{$t('reports.cancel')}</Button>
        <Button variant="primary" onclick={saveChanges}>{$t('reports.save')}</Button>
      {/if}
    </StickyActionBar>
  {/if}
</PageShell>

{#if report}
  <ConfirmDialog
    bind:open={confirmDeleteOpen}
    tone="danger"
    title={$t('reports.deleteTitle')}
    message={$t('reports.confirmDeleteNamed')
      .replace('{type}', typeLabel)
      .replace('{date}', $fmt.date(generatedAt))}
    onConfirm={handleDeleteReport}
  />
{/if}
