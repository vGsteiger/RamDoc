<script lang="ts">
  import { get } from 'svelte/store';
  import { open } from '@tauri-apps/plugin-dialog';
  import {
    importCsvData,
    parseCsvPreview,
    type ColumnMapping,
    type CsvPreview,
    type ImportResult,
  } from '$lib/api';
  import { Alert, Button, Select, SettingsRow } from '$lib/components/ui';
  import { errorText } from '$lib/translations/labels';
  import { t } from '$lib/translations';

  /* CSV patient import: pick a file, map its columns, import. */
  let selectedCsvPath = $state<string | null>(null);
  let csvPreview = $state<CsvPreview | null>(null);
  let csvError = $state('');
  let importing = $state(false);
  let importResult = $state<ImportResult | null>(null);
  let columnMappings = $state<ColumnMapping[]>([]);

  const REQUIRED_PATIENT_FIELDS = ['ahv_number', 'first_name', 'last_name', 'date_of_birth'];

  // The CSV target fields are the patient fields, so they reuse the labels the
  // patient form already has rather than duplicating them under settings.
  const PATIENT_FIELD_KEYS: Record<string, string> = {
    ahv_number: 'ahvNumber',
    first_name: 'firstName',
    last_name: 'lastName',
    date_of_birth: 'dateOfBirth',
    gender: 'gender',
    address: 'address',
    phone: 'phone',
    email: 'email',
    insurance: 'insurance',
    gp_name: 'gpName',
    gp_address: 'gpAddress',
    notes: 'notes',
  };

  let patientFields = $derived([
    { value: '', label: $t('settings.csvImport.skipColumn') },
    ...Object.entries(PATIENT_FIELD_KEYS).map(([value, key]) => ({
      value,
      label: $t(`patients.${key}`) + (REQUIRED_PATIENT_FIELDS.includes(value) ? ' *' : ''),
    })),
  ]);

  let allRequiredMapped = $derived.by(() => {
    const mapped = new Set(columnMappings.map((m) => m.patient_field).filter((f) => f));
    return REQUIRED_PATIENT_FIELDS.every((field) => mapped.has(field));
  });

  let fileName = $derived(
    selectedCsvPath
      ? selectedCsvPath.split('/').pop() || selectedCsvPath.split('\\').pop() || ''
      : ''
  );

  async function handleSelectCsvFile() {
    try {
      const selected = await open({
        title: get(t)('settings.selectCsvDialogTitle'),
        filters: [{ name: 'CSV', extensions: ['csv'] }],
        multiple: false,
      });

      if (!selected) {
        selectedCsvPath = null;
        csvPreview = null;
        return;
      }

      selectedCsvPath = selected as string;
      csvError = '';
      csvPreview = null;
      importResult = null;

      csvPreview = await parseCsvPreview(selectedCsvPath);
      columnMappings = csvPreview.detected_mappings;
    } catch (e) {
      csvError = $errorText(e);
      selectedCsvPath = null;
    }
  }

  async function handleImportCsv() {
    if (!selectedCsvPath || !csvPreview) return;

    importing = true;
    csvError = '';
    try {
      importResult = await importCsvData(selectedCsvPath, columnMappings);
      if (importResult.success) {
        selectedCsvPath = null;
        csvPreview = null;
        columnMappings = [];
      }
    } catch (e) {
      csvError = $errorText(e);
    } finally {
      importing = false;
    }
  }

  function updateColumnMapping(csvHeader: string, patientField: string) {
    const existingIndex = columnMappings.findIndex((m) => m.csv_header === csvHeader);
    if (existingIndex >= 0) {
      columnMappings = columnMappings.map((m, index) =>
        index === existingIndex ? { ...m, patient_field: patientField } : m
      );
    } else {
      columnMappings = [...columnMappings, { csv_header: csvHeader, patient_field: patientField }];
    }
  }

  function rowPrefix(row: number | null | undefined) {
    return row ? $t('settings.csvImport.rowPrefix').replace('{row}', String(row)) : '';
  }
</script>

<SettingsRow
  label={$t('settings.csvImport.title')}
  description={$t('settings.csvImport.description')}
>
  {#snippet control()}
    <Button onclick={handleSelectCsvFile} disabled={importing}>
      {$t('settings.csvImport.selectFile')}
    </Button>
  {/snippet}

  {#if selectedCsvPath}
    <p class="mt-2 truncate text-caption text-fg-muted">
      {$t('settings.csvImport.selected').replace('{name}', fileName)}
    </p>
  {/if}

  {#if csvPreview}
    <div class="mt-3 space-y-3 rounded-control border border-line-subtle bg-surface p-3">
      <p class="text-label font-medium text-fg">
        {$t('settings.csvImport.parsed').replace('{rows}', String(csvPreview.total_rows))}
      </p>

      {#if csvPreview.warnings.length > 0}
        <Alert tone="warning" title={$t('settings.csvImport.warnings')}>
          <ul class="list-disc pl-4 text-label">
            {#each csvPreview.warnings as warning, i (i)}
              <li>{rowPrefix(warning.row)}{warning.message}</li>
            {/each}
          </ul>
        </Alert>
      {/if}

      <div>
        <p class="mb-2 text-label font-medium text-fg-muted">
          {$t('settings.csvImport.columnMapping')}
        </p>
        <div class="space-y-1.5">
          {#each csvPreview.headers as header (header)}
            <div class="flex min-w-0 items-center gap-2">
              <span class="min-w-0 flex-1 truncate text-label text-fg">{header}</span>
              <span class="text-caption text-fg-subtle" aria-hidden="true">→</span>
              <div class="w-48 shrink-0">
                <Select
                  controlSize="sm"
                  aria-label={header}
                  value={columnMappings.find((m) => m.csv_header === header)?.patient_field || ''}
                  onchange={(e: Event & { currentTarget: HTMLSelectElement }) =>
                    updateColumnMapping(header, e.currentTarget.value)}
                >
                  {#each patientFields as field (field.value)}
                    <option value={field.value}>{field.label}</option>
                  {/each}
                </Select>
              </div>
            </div>
          {/each}
        </div>
      </div>

      <div>
        <p class="mb-2 text-label font-medium text-fg-muted">
          {$t('settings.csvImport.sampleData')}
        </p>
        <div class="overflow-x-auto">
          <table class="w-full text-caption">
            <thead>
              <tr class="border-b border-line">
                {#each csvPreview.headers as header (header)}
                  <th class="px-2 py-1 text-left font-medium text-fg-muted">{header}</th>
                {/each}
              </tr>
            </thead>
            <tbody>
              {#each csvPreview.sample_rows.slice(0, 3) as row, r (r)}
                <tr class="border-b border-line-subtle">
                  {#each row as cell, c (c)}
                    <td class="px-2 py-1 text-fg-muted">{cell}</td>
                  {/each}
                </tr>
              {/each}
            </tbody>
          </table>
        </div>
      </div>

      <div class="flex flex-wrap items-center gap-3">
        <Button
          variant="primary"
          onclick={handleImportCsv}
          loading={importing}
          disabled={!allRequiredMapped}
        >
          {importing ? $t('settings.csvImport.importing') : $t('settings.csvImport.importPatients')}
        </Button>
        {#if !allRequiredMapped}
          <p class="text-caption text-warning-fg">{$t('settings.csvImport.requiredFieldsHint')}</p>
        {/if}
      </div>
    </div>
  {/if}

  {#if importResult}
    <Alert
      tone={importResult.success ? 'success' : 'warning'}
      title={$t('settings.csvImport.completed')}
      class="mt-3"
    >
      <p>
        {$t('settings.csvImport.imported').replace('{count}', String(importResult.imported_count))}
      </p>
      <p>
        {$t('settings.csvImport.failed').replace('{count}', String(importResult.failed_count))}
      </p>
      {#if importResult.errors.length > 0}
        <div class="mt-2 max-h-40 overflow-y-auto text-danger-fg">
          <p class="font-medium">{$t('settings.csvImport.errors')}</p>
          {#each importResult.errors.slice(0, 10) as error, i (i)}
            <p>{rowPrefix(error.row)}{error.message}</p>
          {/each}
          {#if importResult.errors.length > 10}
            <p class="mt-1 text-fg-muted">
              {$t('settings.csvImport.moreErrors').replace(
                '{count}',
                String(importResult.errors.length - 10)
              )}
            </p>
          {/if}
        </div>
      {/if}
    </Alert>
  {/if}

  {#if csvError}
    <Alert tone="danger" class="mt-3">{csvError}</Alert>
  {/if}
</SettingsRow>
