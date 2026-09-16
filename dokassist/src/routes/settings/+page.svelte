<script lang="ts">
  import { errorText } from '$lib/translations/labels';
  import { get } from 'svelte/store';
  import { onMount, onDestroy } from 'svelte';
  import { listen, type UnlistenFn } from '@tauri-apps/api/event';
  import { getVersion } from '@tauri-apps/api/app';
  import { open } from '@tauri-apps/plugin-dialog';
  import { goto } from '$app/navigation';
  import {
    getEngineStatus,
    getRecommendedModel,
    downloadModel,
    resetApp,
    checkForUpdates,
    installUpdate,
    exportAllPatientData,
    createVaultBackup,
    restoreVaultBackup,
    validateBackupArchive,
    getEmbedStatus,
    initializeEmbedEngine,
    listModels,
    downloadAndRegisterModel,
    deleteModel,
    setDefaultModel,
    getDefaultModel,
    setTaskModel,
    getTaskModel,
    listTaskModels,
    clearTaskModel,
    listAvailableModels,
    parseCsvPreview,
    importCsvData,
    getMedicationReferenceVersion,
    downloadMedicationReference,
    type LlmEngineStatus,
    type InferenceFallbackCode,
    type InferenceProfile,
    type GenerationStats,
    type ModelChoice,
    type UpdateInfo,
    type EmbedStatus,
    type ModelInfo,
    type TaskModel,
    type AvailableModel,
    type CsvPreview,
    type ColumnMapping,
    type ImportResult,
    type BackupInfo,
    type SamplerConfig,
  } from '$lib/api';
  import { themePreference } from '$lib/stores/theme';
  import { language } from '$lib/stores/language';
  import { engine, loadEngineModel, loadingModelLabel } from '$lib/stores/engine';
  import {
    reportGenerationSettings,
    type ReportGenerationPreset,
  } from '$lib/stores/report-generation';
  import { ThinkingIndicator } from '$lib/components/ui';
  import { t } from '$lib/translations';
  import PromotedModelImportDialog from '$lib/components/PromotedModelImportDialog.svelte';

  function renderMarkdown(text: string): string {
    function escape(s: string) {
      return s.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');
    }
    function inline(s: string) {
      return s
        .replace(/\*\*(.*?)\*\*/g, '<strong>$1</strong>')
        .replace(/\[([^\]]+)\]\([^)]+\)/g, '$1');
    }
    return text
      .split('\n')
      .map((line) => {
        if (/^## /.test(line))
          return `<p class="font-semibold mt-2 mb-0.5">${inline(escape(line.slice(3)))}</p>`;
        if (/^### /.test(line))
          return `<p class="font-medium mt-1 mb-0.5">${inline(escape(line.slice(4)))}</p>`;
        if (/^- /.test(line)) return `<p class="ml-3">&bull; ${inline(escape(line.slice(2)))}</p>`;
        if (line.trim() === '') return '<div class="mt-1"></div>';
        return `<p>${inline(escape(line))}</p>`;
      })
      .join('');
  }

  let status = $state<LlmEngineStatus | null>(null);
  let recommended = $state<ModelChoice | null>(null);
  let downloadProgress = $state<number | null>(null);
  let phase = $state<'idle' | 'downloading' | 'loading' | 'done' | 'error'>('idle');
  let errorMsg = $state('');
  let unlisten: UnlistenFn | null = null;
  let appVersion = $state('');
  let activeDownloadFilename = $state<string | null>(null);

  // Model management state
  let installedModels = $state<ModelInfo[]>([]);
  let availableModels = $state<AvailableModel[]>([]);
  let taskModels = $state<TaskModel[]>([]);
  let selectedTaskModel = $state<Record<string, string>>({});
  let modelManagementError = $state('');
  let loadingModels = $state(false);
  let importDialogOpen = $state(false);
  let inferenceProfile = $state<InferenceProfile>('conservative');
  const samplerFields: readonly (readonly [keyof SamplerConfig, string, number, number, number])[] =
    [
      ['temperature', 'settings.samplerTemperature', 0, 2, 0.05],
      ['top_k', 'settings.samplerTopK', 1, 200, 1],
      ['top_p', 'settings.samplerTopP', 0, 1, 0.05],
      ['min_p', 'settings.samplerMinP', 0, 1, 0.05],
      ['repeat_penalty', 'settings.samplerRepeatPenalty', 0.8, 2, 0.05],
      ['presence_penalty', 'settings.samplerPresencePenalty', 0, 2, 0.1],
      ['seed', 'settings.samplerSeed', 0, 4_294_967_295, 1],
    ];

  function setReportPreset(preset: ReportGenerationPreset) {
    reportGenerationSettings.update((settings) => ({ ...settings, preset }));
  }

  function setSamplerValue(field: keyof SamplerConfig, value: number) {
    reportGenerationSettings.update((settings) => ({
      ...settings,
      custom: { ...settings.custom, [field]: value },
    }));
  }

  // Embedding model state
  let embedStatus = $state<EmbedStatus | null>(null);
  let embedPhase = $state<'idle' | 'loading' | 'done' | 'error'>('idle');
  let embedError = $state('');

  // Medication reference DB state
  let medRefVersion = $state<string | null>(null);
  let medRefPhase = $state<'idle' | 'downloading' | 'done' | 'error'>('idle');
  let medRefProgress = $state<number>(0);
  let medRefError = $state('');
  let medRefUnlisten: UnlistenFn | null = null;

  // Update state
  let updateInfo = $state<UpdateInfo | null>(null);
  let checkingUpdate = $state(false);
  let installingUpdate = $state(false);
  let updateProgress = $state<number>(0);
  let updateError = $state('');
  let updateUnlisten: UnlistenFn | null = null;

  onMount(async () => {
    [status, recommended, appVersion, embedStatus, medRefVersion] = await Promise.all([
      getEngineStatus(),
      getRecommendedModel(),
      getVersion().catch(() => 'Unknown'),
      getEmbedStatus(),
      getMedicationReferenceVersion().catch(() => null),
    ]);
    if (status.inference_config) inferenceProfile = status.inference_config.profile;
    if (status.is_loaded) phase = 'done';
    if (embedStatus.is_loaded) embedPhase = 'done';

    // Load installed models and task assignments
    await loadInstalledModels();
  });

  async function loadInstalledModels() {
    try {
      loadingModels = true;
      const results = await Promise.allSettled([
        listModels(),
        listTaskModels(),
        listAvailableModels(),
      ]);

      // Handle each result separately
      if (results[0].status === 'fulfilled') {
        installedModels = results[0].value;
      } else {
        console.error('Failed to load installed models:', results[0].reason);
      }

      if (results[1].status === 'fulfilled') {
        taskModels = results[1].value;
      } else {
        console.error('Failed to load task models:', results[1].reason);
      }

      if (results[2].status === 'fulfilled') {
        availableModels = results[2].value;
      } else {
        console.error('Failed to load available models:', results[2].reason);
      }

      // Build a map of task -> model_id for easy lookup
      selectedTaskModel = taskModels.reduce(
        (acc, tm) => {
          acc[tm.task_type] = tm.model_id;
          return acc;
        },
        {} as Record<string, string>
      );
    } catch (e) {
      modelManagementError = $errorText(e);
    } finally {
      loadingModels = false;
    }
  }

  async function handleInitEmbed() {
    embedPhase = 'loading';
    embedError = '';
    try {
      await initializeEmbedEngine();
      embedStatus = await getEmbedStatus();
      embedPhase = 'done';
    } catch (e) {
      embedPhase = 'error';
      embedError = $errorText(e);
    }
  }

  async function handleDownloadMedRef() {
    medRefPhase = 'downloading';
    medRefProgress = 0;
    medRefError = '';

    medRefUnlisten = await listen<number>('medication-ref-download-progress', (e) => {
      medRefProgress = Math.round(e.payload * 100);
    });

    try {
      await downloadMedicationReference();
      medRefVersion = await getMedicationReferenceVersion();
      medRefPhase = 'done';
    } catch (e) {
      medRefPhase = 'error';
      medRefError = $errorText(e);
    } finally {
      medRefUnlisten?.();
      medRefUnlisten = null;
    }
  }

  onDestroy(() => {
    unlisten?.();
    doneUnsubscribe?.();
    updateUnlisten?.();
    medRefUnlisten?.();
  });

  async function handleCheckForUpdates() {
    checkingUpdate = true;
    updateError = '';
    try {
      updateInfo = await checkForUpdates();
    } catch (e) {
      updateError = $errorText(e);
    } finally {
      checkingUpdate = false;
    }
  }

  async function handleInstallUpdate() {
    if (!updateInfo?.update_available) return;

    installingUpdate = true;
    updateError = '';
    updateProgress = 0;

    // Listen for download progress events
    updateUnlisten = await listen<number>('updater-download-progress', (e) => {
      updateProgress = Math.round(e.payload * 100);
    });

    const completeUnsub = await listen('updater-download-complete', () => {
      completeUnsub();
    });

    try {
      await installUpdate();
      // After successful install, the app will restart automatically
    } catch (e) {
      updateUnlisten?.();
      updateUnlisten = null;
      installingUpdate = false;
      updateError = $errorText(e);
    }
  }

  function formatBytes(bytes: number): string {
    const gb = bytes / 1024 ** 3;
    return `${gb.toFixed(1)} GB`;
  }

  function inferenceFallbackLabel(code: InferenceFallbackCode): string | null {
    switch (code) {
      case 'native_context_cap':
        return $t('settings.fallbackNativeContext');
      case 'flash_auto':
        return $t('settings.fallbackFlashAuto');
      case 'kv_f16':
        return $t('settings.fallbackKvF16');
      case 'kv_f16_flash_auto':
        return $t('settings.fallbackKvF16FlashAuto');
      default:
        return null;
    }
  }

  function inferenceFallbackDiagnostic(config: NonNullable<LlmEngineStatus['inference_config']>) {
    if (!config.fallback_code) return config.fallback;
    return inferenceFallbackLabel(config.fallback_code) || config.fallback;
  }

  async function handleDownload() {
    if (!recommended) return;
    phase = 'downloading';
    downloadProgress = 0;
    errorMsg = '';

    unlisten = await listen<number>('model-download-progress', (e) => {
      downloadProgress = Math.round(e.payload * 100);
    });

    const doneUnsub = await listen('model-download-done', () => {
      doneUnsub();
    });

    try {
      await downloadModel(recommended);
      unlisten();
      unlisten = null;
      await handleLoad();
    } catch (e) {
      unlisten?.();
      unlisten = null;
      phase = 'error';
      errorMsg = $errorText(e);
    }
  }

  async function handleLoad() {
    if (!recommended) return;
    phase = 'loading';
    errorMsg = '';
    try {
      await loadEngineModel(recommended.filename, inferenceProfile);
      status = get(engine).status;
      phase = 'done';
    } catch (e) {
      phase = 'error';
      errorMsg = $errorText(e);
    }
  }

  // Model management handlers
  let doneUnsubscribe: UnlistenFn | null = null;

  async function handleDownloadNewModel(model: ModelChoice) {
    phase = 'downloading';
    downloadProgress = 0;
    errorMsg = '';
    activeDownloadFilename = model.filename;

    unlisten = await listen<number>('model-download-progress', (e) => {
      downloadProgress = Math.round(e.payload * 100);
    });

    doneUnsubscribe = await listen('model-download-done', () => {
      // Event fired, cleanup handled in finally
    });

    try {
      await downloadAndRegisterModel(model);
      phase = 'idle';
      await loadInstalledModels();
    } catch (e) {
      phase = 'error';
      errorMsg = $errorText(e);
    } finally {
      unlisten?.();
      unlisten = null;
      doneUnsubscribe?.();
      doneUnsubscribe = null;
      activeDownloadFilename = null;
    }
  }

  async function handleLoadModel(filename: string) {
    phase = 'loading';
    errorMsg = '';
    try {
      await loadEngineModel(filename, inferenceProfile);
      status = get(engine).status;
      await loadInstalledModels(); // Refresh list to show loaded badge
      phase = 'done';
    } catch (e) {
      phase = 'error';
      errorMsg = $errorText(e);
    }
  }

  async function handleDeleteModel(modelId: string) {
    try {
      await deleteModel(modelId);
      await loadInstalledModels();
    } catch (e) {
      modelManagementError = $errorText(e);
    }
  }

  async function handleSetDefaultModel(modelId: string) {
    try {
      await setDefaultModel(modelId);
      await loadInstalledModels();
    } catch (e) {
      modelManagementError = $errorText(e);
    }
  }

  async function handleSetTaskModel(taskType: string, modelId: string) {
    try {
      if (modelId === '') {
        // Clear the task model assignment
        await clearTaskModel(taskType);
      } else {
        await setTaskModel(taskType, modelId);
      }
      await loadInstalledModels();
    } catch (e) {
      modelManagementError = $errorText(e);
    }
  }

  let showResetConfirm = $state(false);
  let resetInput = $state('');
  let resetting = $state(false);
  let resetError = $state('');

  // Export state
  let showExportConfirm = $state(false);
  let exportInput = $state('');
  let exporting = $state(false);
  let exportError = $state('');

  // Backup & Restore state
  let creatingBackup = $state(false);
  let backupError = $state('');
  let restoring = $state(false);
  let showRestoreConfirm = $state(false);
  let restoreInput = $state('');
  let restoreError = $state('');
  let selectedBackupFile: File | null = null;
  let validatedBackupInfo = $state<BackupInfo | null>(null);

  // CSV Import state
  let selectedCsvPath = $state<string | null>(null);
  let csvPreview = $state<CsvPreview | null>(null);
  let csvError = $state('');
  let importing = $state(false);
  let importResult = $state<ImportResult | null>(null);
  let columnMappings = $state<ColumnMapping[]>([]);

  async function handleReset() {
    resetting = true;
    resetError = '';
    try {
      await resetApp();
      goto('/');
    } catch (e) {
      resetError = $errorText(e);
      resetting = false;
    }
  }

  async function handleExport() {
    exporting = true;
    exportError = '';
    try {
      const zipData = await exportAllPatientData();

      // Convert number array to Uint8Array
      const blob = new Blob([new Uint8Array(zipData)], {
        type: 'application/zip',
      });

      // Create download link
      const url = URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = url;
      a.download = `RamDoc_Export_${new Date().toISOString().split('T')[0]}.zip`;
      document.body.appendChild(a);
      a.click();
      document.body.removeChild(a);
      URL.revokeObjectURL(url);

      // Reset the form
      showExportConfirm = false;
      exportInput = '';
    } catch (e) {
      exportError = $errorText(e);
    } finally {
      exporting = false;
    }
  }

  async function handleCreateBackup() {
    creatingBackup = true;
    backupError = '';
    try {
      const backupData = await createVaultBackup();

      // Convert number array to Uint8Array and create download
      const blob = new Blob([new Uint8Array(backupData)], {
        type: 'application/octet-stream',
      });
      const url = URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = url;
      a.download = `RamDoc_Backup_${new Date().toISOString().split('T')[0]}.dokassist`;
      document.body.appendChild(a);
      a.click();
      document.body.removeChild(a);
      URL.revokeObjectURL(url);
    } catch (e) {
      backupError = $errorText(e);
    } finally {
      creatingBackup = false;
    }
  }

  async function handleSelectRestoreFile(event: Event & { currentTarget: HTMLInputElement }) {
    const file = event.currentTarget.files?.[0];
    if (!file) {
      selectedBackupFile = null;
      validatedBackupInfo = null;
      return;
    }

    selectedBackupFile = file;
    restoreError = '';

    // Validate the backup file
    try {
      const arrayBuffer = await file.arrayBuffer();
      const backupArray = Array.from(new Uint8Array(arrayBuffer));
      validatedBackupInfo = await validateBackupArchive(backupArray);
    } catch (e) {
      restoreError = $errorText(e);
      selectedBackupFile = null;
      validatedBackupInfo = null;
    }
  }

  async function handleRestoreBackup() {
    if (!selectedBackupFile || !validatedBackupInfo) return;

    restoring = true;
    restoreError = '';
    try {
      const arrayBuffer = await selectedBackupFile.arrayBuffer();
      const backupArray = Array.from(new Uint8Array(arrayBuffer));
      await restoreVaultBackup(backupArray);

      // Reset state
      showRestoreConfirm = false;
      restoreInput = '';
      selectedBackupFile = null;
      validatedBackupInfo = null;

      // Redirect to unlock page since database was replaced
      goto('/');
    } catch (e) {
      restoreError = $errorText(e);
    } finally {
      restoring = false;
    }
  }

  async function handleSelectCsvFile() {
    try {
      const selected = await open({
        title: get(t)('settings.selectCsvDialogTitle'),
        filters: [
          {
            name: 'CSV',
            extensions: ['csv'],
          },
        ],
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

      // Parse CSV preview
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
        // Clear state on success
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

  // Check if all required fields are mapped
  const REQUIRED_PATIENT_FIELDS = ['ahv_number', 'first_name', 'last_name', 'date_of_birth'];

  function hasAllRequiredFieldsMapped(): boolean {
    const mappedFields = new Set(columnMappings.map((m) => m.patient_field).filter((f) => f));
    return REQUIRED_PATIENT_FIELDS.every((field) => mappedFields.has(field));
  }

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
</script>

<div
  class="settings-page mx-auto min-h-full w-full max-w-[1600px] bg-surface px-5 py-6 sm:px-8 lg:px-10 lg:py-9"
>
  <header class="border-b border-line-subtle pb-6">
    <p class="mb-2 text-caption font-semibold uppercase tracking-[0.14em] text-accent-fg">RamDoc</p>
    <h1 class="text-display font-semibold text-fg">{$t('settings.title')}</h1>
    <p class="mt-2 max-w-3xl text-body text-fg-muted">{$t('settings.subtitle')}</p>
  </header>

  <section class="mb-10">
    <h2 class="text-heading font-semibold text-fg mb-4">
      {$t('settings.applicationUpdates')}
    </h2>

    <div class="bg-surface-hover rounded-card p-4 mb-4">
      <div class="flex items-center justify-between mb-3">
        <div>
          <p class="text-body font-medium text-fg">
            {$t('settings.currentVersion')}
          </p>
          <p class="text-caption text-fg-muted mt-1">
            {appVersion || $t('common.loading')}
          </p>
        </div>
        <button
          onclick={handleCheckForUpdates}
          disabled={checkingUpdate || installingUpdate}
          class="h-8 px-3 text-body rounded-control bg-accent hover:bg-accent-hover disabled:opacity-50 disabled:cursor-not-allowed text-on-accent transition-colors"
        >
          {checkingUpdate ? $t('settings.checking') : $t('settings.checkForUpdates')}
        </button>
      </div>

      {#if updateInfo}
        {#if updateInfo.update_available}
          <div class="border-t border-line pt-3 mt-3">
            <div class="flex items-start justify-between gap-4 mb-2">
              <div>
                <p class="text-body font-medium text-success-fg">
                  {$t('settings.updateAvailable')}
                </p>
                <p class="text-caption text-fg-muted mt-1">
                  {$t('settings.version')}
                  {updateInfo.latest_version}
                  {$t('settings.versionAvailable')}
                </p>
              </div>
            </div>

            {#if updateInfo.body}
              <div
                class="text-caption text-fg-muted mb-3 max-h-32 overflow-y-auto bg-surface-selected rounded-card p-2"
              >
                <p class="font-medium mb-1">{$t('settings.releaseNotes')}</p>
                <div>{@html renderMarkdown(updateInfo.body)}</div>
              </div>
            {/if}

            {#if installingUpdate}
              <div class="mb-3">
                <div class="flex justify-between text-caption text-fg-muted mb-1">
                  <span>{$t('settings.downloadingAndInstalling')}</span>
                  <span>{updateProgress}%</span>
                </div>
                <div class="w-full bg-surface-selected rounded-full h-2">
                  <div
                    class="bg-accent h-2 rounded-full transition-colors"
                    style="width: {updateProgress}%"
                  ></div>
                </div>
                <p class="text-caption text-fg-muted mt-2">
                  {$t('settings.autoRestart')}
                </p>
              </div>
            {/if}

            {#if updateError}
              <p class="text-caption text-danger-fg mb-3">{updateError}</p>
            {/if}

            {#if !installingUpdate}
              <button
                onclick={handleInstallUpdate}
                class="h-8 px-3 text-body rounded-control bg-success hover:bg-success text-on-success transition-colors"
              >
                {$t('settings.installUpdate')}
              </button>
            {/if}
          </div>
        {:else}
          <div class="border-t border-line pt-3 mt-3">
            <p class="text-body text-success-fg">{$t('settings.noUpdatesAvailable')}</p>
          </div>
        {/if}
      {/if}

      {#if updateError && !updateInfo}
        <div class="border-t border-line pt-3 mt-3">
          <p class="text-caption text-danger-fg">{updateError}</p>
        </div>
      {/if}
    </div>
  </section>

  <section class="mb-10">
    <h2 class="text-heading font-semibold text-fg mb-4">
      {$t('settings.appearance')}
    </h2>

    <div class="bg-surface-hover rounded-card p-4 mb-4">
      <p class="text-body font-medium text-fg mb-3">
        {$t('settings.theme')}
      </p>
      <p class="text-caption text-fg-muted mb-4">
        {$t('settings.themeDescription')}
      </p>

      <div class="space-y-2">
        <label
          class="flex items-center gap-3 p-3 rounded-card border border-line cursor-pointer hover:bg-surface-hover transition-colors {$themePreference ===
          'light'
            ? 'bg-surface-selected border-accent'
            : ''}"
        >
          <input
            type="radio"
            name="theme"
            value="light"
            checked={$themePreference === 'light'}
            onchange={() => themePreference.set('light')}
            class="w-4 h-4 text-accent-fg focus:ring-accent/30"
          />
          <div>
            <p class="text-body font-medium text-fg">
              {$t('settings.light')}
            </p>
            <p class="text-caption text-fg-muted">
              {$t('settings.lightDescription')}
            </p>
          </div>
        </label>

        <label
          class="flex items-center gap-3 p-3 rounded-card border border-line cursor-pointer hover:bg-surface-hover transition-colors {$themePreference ===
          'dark'
            ? 'bg-surface-selected border-accent'
            : ''}"
        >
          <input
            type="radio"
            name="theme"
            value="dark"
            checked={$themePreference === 'dark'}
            onchange={() => themePreference.set('dark')}
            class="w-4 h-4 text-accent-fg focus:ring-accent/30"
          />
          <div>
            <p class="text-body font-medium text-fg">
              {$t('settings.dark')}
            </p>
            <p class="text-caption text-fg-muted">{$t('settings.darkDescription')}</p>
          </div>
        </label>

        <label
          class="flex items-center gap-3 p-3 rounded-card border border-line cursor-pointer hover:bg-surface-hover transition-colors {$themePreference ===
          'system'
            ? 'bg-surface-selected border-accent'
            : ''}"
        >
          <input
            type="radio"
            name="theme"
            value="system"
            checked={$themePreference === 'system'}
            onchange={() => themePreference.set('system')}
            class="w-4 h-4 text-accent-fg focus:ring-accent/30"
          />
          <div>
            <p class="text-body font-medium text-fg">
              {$t('settings.system')}
            </p>
            <p class="text-caption text-fg-muted">
              {$t('settings.systemDescription')}
            </p>
          </div>
        </label>
      </div>
    </div>

    <div class="bg-surface-hover rounded-card p-4">
      <p class="text-body font-medium text-fg mb-3">
        {$t('settings.language')}
      </p>
      <p class="text-caption text-fg-muted mb-4">
        {$t('settings.languageDescription')}
      </p>

      <div class="space-y-2">
        <label
          class="flex items-center gap-3 p-3 rounded-card border border-line cursor-pointer hover:bg-surface-hover transition-colors {$language ===
          'en'
            ? 'bg-surface-selected border-accent'
            : ''}"
        >
          <input
            type="radio"
            name="language"
            value="en"
            checked={$language === 'en'}
            onchange={() => language.set('en')}
            class="w-4 h-4 text-accent-fg focus:ring-accent/30"
          />
          <div>
            <p class="text-body font-medium text-fg">
              {$t('settings.english')}
            </p>
            <p class="text-caption text-fg-muted">English</p>
          </div>
        </label>

        <label
          class="flex items-center gap-3 p-3 rounded-card border border-line cursor-pointer hover:bg-surface-hover transition-colors {$language ===
          'de'
            ? 'bg-surface-selected border-accent'
            : ''}"
        >
          <input
            type="radio"
            name="language"
            value="de"
            checked={$language === 'de'}
            onchange={() => language.set('de')}
            class="w-4 h-4 text-accent-fg focus:ring-accent/30"
          />
          <div>
            <p class="text-body font-medium text-fg">
              {$t('settings.german')}
            </p>
            <p class="text-caption text-fg-muted">Deutsch</p>
          </div>
        </label>
      </div>
    </div>
  </section>

  <!-- Enhanced Model Management Section -->
  <section>
    <h2 class="text-heading font-semibold text-fg mb-4">
      {$t('settings.modelManagement')}
    </h2>

    <div class="bg-surface-hover rounded-card p-4 mb-6">
      <label for="inference-profile" class="block text-body font-medium text-fg mb-2">
        {$t('settings.inferenceProfile')}
      </label>
      <select
        id="inference-profile"
        bind:value={inferenceProfile}
        disabled={phase === 'loading' || $engine.isLoading}
        class="w-full max-w-md rounded-control border border-line bg-surface-raised px-3 py-2 text-body text-fg"
      >
        <option value="conservative">{$t('settings.inferenceConservative')}</option>
        <option value="f16-32k">{$t('settings.inferenceF16')}</option>
        <option value="q8-32k">{$t('settings.inferenceQ8')}</option>
        <option value="q4-32k">{$t('settings.inferenceQ4')}</option>
      </select>
      <p class="text-caption text-fg-muted mt-2">
        {$t('settings.inferenceProfileDescription')}
      </p>
    </div>

    <div class="bg-surface-hover rounded-card p-4 mb-6">
      <label for="report-generation-preset" class="block text-body font-medium text-fg mb-2">
        {$t('settings.reportGenerationPreset')}
      </label>
      <select
        id="report-generation-preset"
        value={$reportGenerationSettings.preset}
        onchange={(event) => setReportPreset(event.currentTarget.value as ReportGenerationPreset)}
        class="w-full max-w-md rounded-control border border-line bg-surface-raised px-3 py-2 text-body text-fg"
      >
        <option value="clinical">{$t('settings.reportPresetClinical')}</option>
        <option value="conservative">{$t('settings.reportPresetConservative')}</option>
        <option value="varied">{$t('settings.reportPresetVaried')}</option>
        <option value="custom">{$t('settings.reportPresetCustom')}</option>
      </select>
      <p class="text-caption text-fg-muted mt-2">
        {$t('settings.reportGenerationPresetDescription')}
      </p>

      {#if $reportGenerationSettings.preset === 'custom'}
        <div class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-3 mt-4">
          {#each samplerFields as field (field[0])}
            <label class="block text-caption text-fg-muted">
              {$t(field[1])}
              <input
                type="number"
                value={$reportGenerationSettings.custom[field[0]]}
                min={field[2]}
                max={field[3]}
                step={field[4]}
                onchange={(event) => setSamplerValue(field[0], event.currentTarget.valueAsNumber)}
                class="mt-1 w-full rounded-control border border-line bg-surface-raised px-3 py-2 text-body text-fg"
              />
            </label>
          {/each}
        </div>
        <p class="text-caption text-warning-fg mt-3">
          {$t('settings.customSamplerWarning')}
        </p>
      {/if}
    </div>

    <!-- Currently loaded model status -->
    <div class="bg-surface-hover rounded-card p-4 mb-6">
      {#if $engine.isLoading && $engine.loadingStartedAt}
        <div class="mb-4">
          <ThinkingIndicator
            startedAt={$engine.loadingStartedAt}
            label={$t('settings.loadingModelIntoMemory').replace(
              '{name}',
              loadingModelLabel($engine.loadingFilename)
            )}
          />
          <p class="text-caption text-fg-muted mt-2">{$t('settings.loadingModelHint')}</p>
        </div>
      {:else}
        <div class="flex items-center gap-3 mb-4">
          <div
            class="w-3 h-3 rounded-full shrink-0 {status?.is_loaded
              ? 'bg-success'
              : 'bg-surface-selected'}"
          ></div>
          <div class="flex-1">
            <p class="text-body font-medium text-fg">
              {status?.is_loaded
                ? $t('settings.loadedModel').replace('{name}', status.model_name ?? '')
                : $t('settings.noModelLoaded')}
            </p>
            {#if status?.total_ram_bytes}
              <p class="text-caption text-fg-muted">
                {$t('settings.systemRam').replace('{ram}', formatBytes(status.total_ram_bytes))}
              </p>
            {/if}
          </div>
        </div>
      {/if}

      {#if status?.inference_config}
        {@const config = status.inference_config}
        {@const fallbackDiagnostic = inferenceFallbackDiagnostic(config)}
        <div class="border-t border-line pt-3 mb-3">
          <p class="text-caption font-medium text-fg-muted mb-1">
            {$t('settings.activeInferenceConfiguration')}
          </p>
          <p class="text-caption text-fg-muted">
            {$t('settings.inferenceConfigSummary')
              .replace('{context}', String(Math.round(config.context_size / 1024)))
              .replace('{key}', config.kv_cache_k)
              .replace('{value}', config.kv_cache_v)
              .replace('{batch}', String(config.n_batch))
              .replace('{microBatch}', String(config.n_ubatch))
              .replace(
                '{flash}',
                $t(
                  config.flash_attention === 'enabled'
                    ? 'settings.flashEnabled'
                    : 'settings.flashAuto'
                )
              )}
          </p>
          <p class="text-caption text-fg-subtle mt-1">
            {$t('settings.completionHeadroom').replace(
              '{tokens}',
              String(config.completion_headroom)
            )}
          </p>
          {#if fallbackDiagnostic}
            <p
              class="text-caption text-warning-fg bg-warning-subtle border border-warning-line rounded-card px-2.5 py-2 mt-2"
            >
              {fallbackDiagnostic}
            </p>
          {/if}
        </div>
      {/if}

      {#if status?.last_generation_stats}
        {@const s = status.last_generation_stats}
        <div class="border-t border-line pt-3">
          <p class="text-caption font-medium text-fg-muted mb-2">
            {$t('settings.lastGenerationPerformance')}
          </p>
          <div class="grid grid-cols-2 gap-3">
            <div class="bg-surface-selected rounded-card p-2.5 text-center">
              <p class="text-heading font-semibold text-fg">
                {(s.ttft_ms / 1000).toFixed(2)}s
              </p>
              <p class="text-caption text-fg-muted">{$t('settings.timeToFirstToken')}</p>
            </div>
            <div class="bg-surface-selected rounded-card p-2.5 text-center">
              <p class="text-heading font-semibold text-fg">
                {s.tps.toFixed(1)} tok/s
              </p>
              <p class="text-caption text-fg-muted">{$t('settings.throughput')}</p>
            </div>
          </div>
          <p class="text-caption text-fg-subtle mt-2">
            {$t('settings.tokenStats')
              .replace('{prompt}', String(s.prompt_tokens))
              .replace('{completion}', String(s.completion_tokens))}
          </p>
          <p class="text-caption text-fg-subtle mt-1">
            {s.cache_hit ? $t('settings.contextWarm') : $t('settings.contextCold')} ·
            {$t('settings.cacheStats')
              .replace('{reused}', String(s.reused_prompt_tokens))
              .replace('{evaluated}', String(s.evaluated_prompt_tokens))
              .replace('{prefill}', s.prefill_ms.toFixed(0))}
          </p>
        </div>
      {/if}
    </div>

    <!-- Installed Models List -->
    <div class="mb-6">
      <div class="flex flex-wrap items-start justify-between gap-3 mb-3">
        <h3 class="text-md font-semibold text-fg">
          {$t('settings.installedModels')}
        </h3>
        <button
          onclick={() => (importDialogOpen = true)}
          class="h-8 px-3 text-caption rounded-control bg-surface-selected hover:bg-surface-selected text-fg transition-colors"
        >
          {$t('settings.importPromotedModel')}
        </button>
      </div>

      {#if loadingModels}
        <p class="text-body text-fg-muted">{$t('settings.loadingModels')}</p>
      {:else if installedModels.length === 0}
        <p class="text-body text-fg-muted mb-4">
          {$t('settings.noModelsInstalled')}
        </p>
      {:else}
        <div class="space-y-3">
          {#each installedModels as model (model.id)}
            <div class="bg-surface-hover border border-line rounded-card p-4">
              <div class="flex items-start justify-between mb-2">
                <div class="flex-1">
                  <div class="flex items-center gap-2 mb-1">
                    <p class="text-body font-medium text-fg">
                      {model.name}
                    </p>
                    {#if model.is_default}
                      <span class="px-2 py-0.5 text-caption rounded-card bg-accent text-on-accent">
                        {$t('settings.defaultBadge')}
                      </span>
                    {/if}
                    {#if model.is_loaded}
                      <span
                        class="px-2 py-0.5 text-caption rounded-card bg-success text-on-success"
                      >
                        {$t('settings.loadedBadge')}
                      </span>
                    {/if}
                    {#if model.quantization_promotion}
                      <span
                        class="px-2 py-0.5 text-caption rounded-card bg-info-subtle text-info-fg border border-info-line"
                      >
                        {$t('settings.clinicalGateBadge')}
                      </span>
                    {/if}
                    {#if !model.exists_on_disk}
                      <span class="px-2 py-0.5 text-caption rounded-card bg-danger text-on-danger">
                        {$t('settings.modelMissingOnDisk')}
                      </span>
                    {/if}
                  </div>
                  <p class="text-caption text-fg-muted">
                    {model.filename} • {formatBytes(model.size_bytes)}
                  </p>
                  {#if model.last_used}
                    <p class="text-caption text-fg-subtle mt-1">
                      {$t('settings.lastUsed').replace(
                        '{date}',
                        new Date(model.last_used).toLocaleDateString()
                      )}
                    </p>
                  {/if}
                  {#if model.quantization_promotion}
                    {@const promotion = model.quantization_promotion}
                    <details class="mt-2 rounded-card border border-line p-2.5">
                      <summary class="text-caption text-fg cursor-pointer">
                        {$t('settings.clinicalGateDetails')}
                      </summary>
                      <div class="mt-2 space-y-1">
                        <p class="text-caption text-fg">
                          {$t('settings.clinicalGateSummary')
                            .replace('{quantization}', promotion.quantization)
                            .replace('{baselines}', promotion.baseline_artifacts.join(', '))}
                        </p>
                        <p class="text-caption text-fg-muted">
                          {$t('settings.clinicalGateStudy').replace('{study}', promotion.study_id)}
                        </p>
                        <p class="text-caption text-fg-muted">
                          {$t('settings.clinicalGateEvidence').replace(
                            '{regression}',
                            (promotion.worst_category_regression * 100).toFixed(2)
                          )}
                        </p>
                        <p class="text-caption text-fg-subtle">
                          {$t('settings.clinicalGateHeldOut').replace(
                            '{hash}',
                            promotion.held_out_results_sha256.slice(0, 12)
                          )}
                        </p>
                        <p class="text-caption text-fg-muted mt-1">
                          {$t('settings.promotionDisclaimer')}
                        </p>
                      </div>
                    </details>
                  {/if}
                </div>
              </div>

              <div class="flex gap-2 mt-3">
                {#if model.exists_on_disk && !model.is_loaded}
                  <button
                    onclick={() => handleLoadModel(model.filename)}
                    disabled={phase === 'loading' || $engine.isLoading}
                    class="h-7 px-2.5 text-caption rounded-control bg-accent hover:bg-accent-hover disabled:opacity-50 disabled:cursor-not-allowed text-on-accent transition-colors"
                  >
                    {$t('settings.load')}
                  </button>
                {/if}
                {#if model.exists_on_disk && !model.is_default}
                  <button
                    onclick={() => handleSetDefaultModel(model.id)}
                    class="h-7 px-2.5 text-caption rounded-control bg-surface-selected hover:bg-surface-selected text-fg transition-colors"
                  >
                    {$t('settings.setDefault')}
                  </button>
                {/if}
                {#if !model.is_loaded}
                  <button
                    onclick={() => handleDeleteModel(model.id)}
                    class="h-7 px-2.5 text-caption rounded-control bg-danger hover:bg-danger text-on-danger transition-colors"
                  >
                    {$t('settings.delete')}
                  </button>
                {/if}
              </div>
            </div>
          {/each}
        </div>
      {/if}
    </div>

    <!-- Available Models Card -->
    <div class="mb-6">
      <h3 class="text-heading font-semibold text-fg mb-3">{$t('settings.availableModels')}</h3>
      <p class="text-caption text-fg-muted mb-4">
        {$t('settings.availableModelsHint').replace(
          '{ram}',
          status?.total_ram_bytes ? formatBytes(status.total_ram_bytes) : '…'
        )}
      </p>

      {#if loadingModels}
        <p class="text-body text-fg-muted">{$t('settings.loadingAvailableModels')}</p>
      {:else}
        <div class="space-y-3">
          {#each availableModels as model}
            {@const canRunModel = status?.total_ram_bytes
              ? status.total_ram_bytes >= model.min_ram_gb * 1024 * 1024 * 1024
              : false}
            {@const modelChoice = {
              name: model.name,
              filename: model.filename,
              size_bytes: model.size_bytes,
              reason: model.description,
            }}

            <div
              class="bg-surface-hover border border-line rounded-card p-4 {!canRunModel
                ? 'opacity-60'
                : ''}"
            >
              <div class="flex items-start justify-between gap-4 mb-2">
                <div class="flex-1">
                  <div class="flex items-center gap-2 mb-1">
                    <p class="text-body font-medium text-fg">
                      {model.name}
                    </p>
                    {#if model.is_downloaded}
                      <span
                        class="px-2 py-0.5 text-caption rounded-card bg-success text-on-success"
                      >
                        {$t('settings.downloadedBadge')}
                      </span>
                    {/if}
                    {#if !canRunModel}
                      <span
                        class="px-2 py-0.5 text-caption rounded-card bg-warning text-on-warning"
                      >
                        {$t('settings.needsRam').replace('{gb}', String(model.min_ram_gb))}
                      </span>
                    {:else}
                      <span class="px-2 py-0.5 text-caption rounded-card bg-accent text-on-accent">
                        {$t('settings.compatibleBadge')}
                      </span>
                    {/if}
                  </div>
                  <p class="text-caption text-fg-muted mb-1">
                    {model.description}
                  </p>
                  {#if model.disclaimer}
                    <p class="text-caption text-warning-fg mb-1">
                      {model.disclaimer}
                    </p>
                  {/if}
                  <p class="text-caption text-fg-subtle">
                    {$t('settings.modelSpecs')
                      .replace('{size}', formatBytes(model.size_bytes))
                      .replace('{ram}', String(model.min_ram_gb))
                      .replace('{context}', String(Math.round(model.context_window_tokens / 1024)))}
                  </p>
                  <p class="text-caption text-fg-subtle mt-1">
                    {model.parameters} • {model.license}
                  </p>
                </div>
              </div>

              {#if !model.is_downloaded}
                <div class="mt-3">
                  {#if phase === 'downloading' && activeDownloadFilename === model.filename}
                    <div class="mb-3">
                      <div class="flex justify-between text-caption text-fg-muted mb-1">
                        <span>{$t('settings.downloadingEllipsis')}</span>
                        <span>{downloadProgress ?? 0}%</span>
                      </div>
                      <div class="w-full bg-surface-selected rounded-full h-2">
                        <div
                          class="bg-accent h-2 rounded-full transition-colors"
                          style="width: {downloadProgress ?? 0}%"
                        ></div>
                      </div>
                    </div>
                  {/if}

                  <button
                    onclick={() => handleDownloadNewModel(modelChoice)}
                    disabled={phase === 'downloading' ||
                      phase === 'loading' ||
                      $engine.isLoading ||
                      !canRunModel}
                    class="h-8 px-3 text-body rounded-control bg-accent hover:bg-accent-hover disabled:opacity-50 disabled:cursor-not-allowed text-on-accent transition-colors"
                  >
                    {phase === 'downloading' && activeDownloadFilename === model.filename
                      ? $t('settings.downloadingEllipsis')
                      : $t('settings.downloadModel')}
                  </button>
                  {#if !canRunModel}
                    <p class="text-caption text-warning-fg mt-2">
                      {$t('settings.insufficientRam')
                        .replace('{required}', String(model.min_ram_gb))
                        .replace(
                          '{available}',
                          status?.total_ram_bytes ? formatBytes(status.total_ram_bytes) : '…'
                        )}
                    </p>
                  {/if}
                </div>
              {/if}
            </div>
          {/each}
        </div>
      {/if}
    </div>

    <!-- Download New Model -->
    {#if recommended}
      <div class="mb-6">
        <h3 class="text-md font-semibold text-fg mb-3">
          {$t('settings.recommendedModelSection')}
        </h3>
        <div class="bg-surface-hover border border-line rounded-card p-4">
          <div class="flex items-start justify-between gap-4 mb-2">
            <div>
              <p class="text-body font-medium text-fg">
                {recommended.name}
              </p>
              <p class="text-caption text-fg-muted mt-1">
                {recommended.reason}
              </p>
              <p class="text-caption text-fg-subtle mt-1">
                {$t('settings.size').replace('{size}', formatBytes(recommended.size_bytes))}
              </p>
            </div>
          </div>

          {#if phase === 'downloading'}
            <div class="mb-3">
              <div class="flex justify-between text-caption text-fg-muted mb-1">
                <span>{$t('settings.downloadingLabel')}</span>
                <span>{downloadProgress ?? 0}%</span>
              </div>
              <div class="w-full bg-surface-selected rounded-full h-2">
                <div
                  class="bg-accent h-2 rounded-full transition-colors"
                  style="width: {downloadProgress ?? 0}%"
                ></div>
              </div>
            </div>
          {/if}

          {#if phase === 'error'}
            <p class="text-caption text-danger-fg mb-3">{errorMsg}</p>
          {/if}

          <button
            onclick={() => handleDownloadNewModel(recommended!)}
            disabled={phase === 'downloading' || phase === 'loading' || $engine.isLoading}
            class="h-8 px-3 text-body rounded-control bg-accent hover:bg-accent-hover disabled:opacity-50 disabled:cursor-not-allowed text-on-accent transition-colors"
          >
            {phase === 'downloading'
              ? $t('settings.downloadingLabel')
              : $t('settings.downloadModel')}
          </button>
        </div>
      </div>
    {/if}

    <!-- Task-Specific Model Assignment -->
    <div class="mb-6">
      <h3 class="text-md font-semibold text-fg mb-3">
        {$t('settings.taskSpecificModels')}
      </h3>
      <p class="text-caption text-fg-muted mb-4">
        {$t('settings.taskSpecificModelsDesc')}
      </p>

      {#if installedModels.length > 0}
        <div class="space-y-3">
          {#each ['summary', 'letter', 'report'] as taskType}
            <div class="bg-surface-hover border border-line rounded-card p-4">
              <label
                for="task-{taskType}"
                class="block text-body font-medium text-fg mb-2 capitalize"
              >
                {$t(`settings.${taskType}`)}
              </label>
              <select
                id="task-{taskType}"
                value={selectedTaskModel[taskType] || ''}
                onchange={(e) => handleSetTaskModel(taskType, e.currentTarget.value)}
                class="w-full px-3 py-2 text-body border border-line rounded-control bg-surface-raised text-fg focus:ring-2 focus:ring-accent/30 focus:border-transparent"
              >
                <option value="">{$t('settings.useDefaultModel')}</option>
                {#each installedModels as model}
                  <option value={model.id}>
                    {model.name} ({formatBytes(model.size_bytes)})
                  </option>
                {/each}
              </select>
            </div>
          {/each}
        </div>
      {:else}
        <p class="text-caption text-fg-muted">
          {$t('settings.installModelFirst')}
        </p>
      {/if}
    </div>

    {#if modelManagementError}
      <div class="bg-danger-subtle border border-danger-line rounded-card p-3 mb-4">
        <p class="text-body text-danger-fg">{modelManagementError}</p>
      </div>
    {/if}
  </section>

  <PromotedModelImportDialog bind:open={importDialogOpen} onImported={loadInstalledModels} />

  <section class="mt-10">
    <h2 class="text-heading font-semibold text-fg mb-4">
      {$t('settings.embeddingModel')}
    </h2>
    <p class="text-caption text-fg-muted mb-4">
      {$t('settings.embeddingModelDesc')}
    </p>

    <div class="bg-surface-hover rounded-card p-4 mb-4 flex items-center gap-3">
      <div
        class="w-3 h-3 rounded-full shrink-0 {embedStatus?.is_loaded
          ? 'bg-success'
          : embedStatus?.is_downloaded
            ? 'bg-warning'
            : 'bg-danger'}"
      ></div>
      <div class="flex-1">
        {#if embedStatus?.is_loaded}
          <p class="text-body text-fg font-medium">nomic-embed-text-v1.5</p>
          <p class="text-caption text-fg-muted">{$t('settings.embeddingLoaded')}</p>
        {:else if embedStatus?.is_downloaded}
          <p class="text-body text-fg font-medium">
            {$t('settings.embeddingCached')}
          </p>
          <p class="text-caption text-fg-muted">
            {$t('settings.embeddingCachedDesc')}
          </p>
        {:else}
          <p class="text-body text-fg font-medium">
            {$t('settings.embeddingNotDownloaded')}
          </p>
          <p class="text-caption text-fg-muted">
            {$t('settings.embeddingNotDownloadedDesc')}
          </p>
        {/if}
      </div>

      {#if embedPhase !== 'done'}
        <button
          onclick={handleInitEmbed}
          disabled={embedPhase === 'loading'}
          class="h-8 px-3 text-body rounded-control bg-accent hover:bg-accent-hover disabled:opacity-50 disabled:cursor-not-allowed text-on-accent transition-colors shrink-0"
        >
          {embedPhase === 'loading' ? $t('common.loading') : $t('settings.loadNow')}
        </button>
      {/if}
    </div>

    {#if embedPhase === 'loading'}
      <p class="text-caption text-accent-fg">
        {$t('settings.embeddingInitializing')}
      </p>
    {/if}
    {#if embedPhase === 'error'}
      <p class="text-caption text-danger-fg">{embedError}</p>
    {/if}
  </section>

  <section class="mt-10">
    <h2 class="text-heading font-semibold text-fg mb-4">
      {$t('settings.medicationReference')}
    </h2>
    <p class="text-caption text-fg-muted mb-4">
      {$t('settings.medicationReferenceDesc')}
    </p>

    <div class="bg-surface-hover rounded-card p-4 mb-4 flex items-center gap-3">
      <div
        class="w-3 h-3 rounded-full shrink-0 {medRefVersion ? 'bg-success' : 'bg-surface-selected'}"
      ></div>
      <div class="flex-1">
        {#if medRefVersion}
          <p class="text-body text-fg font-medium">
            {$t('settings.medRefInstalled').replace('{version}', medRefVersion)}
          </p>
          <p class="text-caption text-fg-muted">
            {$t('settings.medRefInstalledDesc')}
          </p>
        {:else}
          <p class="text-body text-fg font-medium">{$t('settings.medRefNotInstalled')}</p>
          <p class="text-caption text-fg-muted">
            {$t('settings.medRefNotInstalledDesc')}
          </p>
        {/if}
      </div>

      <button
        onclick={handleDownloadMedRef}
        disabled={medRefPhase === 'downloading'}
        class="h-8 px-3 text-body rounded-control bg-accent hover:bg-accent-hover disabled:opacity-50 disabled:cursor-not-allowed text-on-accent transition-colors shrink-0"
      >
        {#if medRefPhase === 'downloading'}
          {medRefProgress}%…
        {:else if medRefVersion}
          {$t('settings.medRefUpdate')}
        {:else}
          {$t('settings.medRefDownload')}
        {/if}
      </button>
    </div>

    {#if medRefPhase === 'downloading'}
      <div class="w-full bg-surface-selected rounded-full h-1.5 mb-2">
        <div
          class="bg-accent h-1.5 rounded-full transition-colors"
          style="width: {medRefProgress}%"
        ></div>
      </div>
      <p class="text-caption text-accent-fg">{$t('settings.medRefDownloading')}</p>
    {/if}
    {#if medRefPhase === 'error'}
      <p class="text-caption text-danger-fg">{medRefError}</p>
    {/if}
  </section>

  <section class="mt-10">
    <h2 class="text-heading font-semibold text-fg mb-4">{$t('settings.csvImport.section')}</h2>

    <div class="bg-surface-hover rounded-card p-4">
      <div class="mb-3">
        <p class="text-body font-medium text-fg mb-1">{$t('settings.csvImport.title')}</p>
        <p class="text-caption text-fg-muted mb-3">{$t('settings.csvImport.description')}</p>
      </div>

      <div class="mb-3">
        <button
          onclick={handleSelectCsvFile}
          class="h-8 px-3 text-body rounded-control bg-accent hover:bg-accent-hover text-on-accent transition-colors"
        >
          {$t('settings.csvImport.selectFile')}
        </button>
        {#if selectedCsvPath}
          <p class="text-caption text-fg-muted mt-2">
            {$t('settings.csvImport.selected').replace(
              '{name}',
              selectedCsvPath.split('/').pop() || selectedCsvPath.split('\\').pop() || ''
            )}
          </p>
        {/if}
      </div>

      {#if csvPreview}
        <div class="mb-4 p-3 bg-accent-subtle border border-accent rounded-card">
          <p class="text-caption font-medium text-accent-fg mb-2">
            {$t('settings.csvImport.parsed').replace('{rows}', String(csvPreview.total_rows))}
          </p>

          {#if csvPreview.warnings.length > 0}
            <div class="mb-3 text-caption text-warning-fg">
              <p class="font-medium mb-1">{$t('settings.csvImport.warnings')}</p>
              {#each csvPreview.warnings as warning}
                <p>
                  • {warning.row
                    ? $t('settings.csvImport.rowPrefix').replace('{row}', String(warning.row))
                    : ''}{warning.message}
                </p>
              {/each}
            </div>
          {/if}

          <div class="mb-3">
            <p class="text-caption font-medium text-fg-muted mb-2">
              {$t('settings.csvImport.columnMapping')}
            </p>
            <div class="space-y-2">
              {#each csvPreview.headers as header}
                <div class="flex items-center gap-2">
                  <span class="text-caption text-fg-muted flex-1">{header}</span>
                  <span class="text-caption text-fg-muted">→</span>
                  <select
                    value={columnMappings.find((m) => m.csv_header === header)?.patient_field || ''}
                    onchange={(e) => updateColumnMapping(header, e.currentTarget.value)}
                    class="text-caption px-2 py-1 rounded-control bg-surface-hover border border-line text-fg focus:outline-none focus:border-accent"
                  >
                    {#each patientFields as field}
                      <option value={field.value}>{field.label}</option>
                    {/each}
                  </select>
                </div>
              {/each}
            </div>
          </div>

          <div class="mb-3">
            <p class="text-caption font-medium text-fg-muted mb-2">
              {$t('settings.csvImport.sampleData')}
            </p>
            <div class="overflow-x-auto">
              <table class="text-caption w-full">
                <thead>
                  <tr class="border-b border-line">
                    {#each csvPreview.headers as header}
                      <th class="text-left px-2 py-1 text-fg-muted">{header}</th>
                    {/each}
                  </tr>
                </thead>
                <tbody>
                  {#each csvPreview.sample_rows.slice(0, 3) as row}
                    <tr class="border-b border-line-subtle">
                      {#each row as cell}
                        <td class="px-2 py-1 text-fg-muted">{cell}</td>
                      {/each}
                    </tr>
                  {/each}
                </tbody>
              </table>
            </div>
          </div>

          <button
            onclick={handleImportCsv}
            disabled={importing || !hasAllRequiredFieldsMapped()}
            class="h-8 px-3 text-body rounded-control bg-success hover:bg-success disabled:opacity-50 disabled:cursor-not-allowed text-on-success transition-colors"
          >
            {importing
              ? $t('settings.csvImport.importing')
              : $t('settings.csvImport.importPatients')}
          </button>

          {#if !hasAllRequiredFieldsMapped()}
            <p class="text-caption text-warning-fg mt-2">
              {$t('settings.csvImport.requiredFieldsHint')}
            </p>
          {/if}
        </div>
      {/if}

      {#if importResult}
        <div
          class="mb-3 p-3 {importResult.success
            ? 'bg-success-subtle border border-success'
            : 'bg-warning-subtle border border-warning-line'} rounded-card"
        >
          <p
            class="text-caption font-medium {importResult.success
              ? 'text-success-fg'
              : 'text-warning-fg'} mb-2"
          >
            {$t('settings.csvImport.completed')}
          </p>
          <div class="text-caption text-fg-muted space-y-1">
            <p>
              {$t('settings.csvImport.imported').replace(
                '{count}',
                String(importResult.imported_count)
              )}
            </p>
            <p>
              {$t('settings.csvImport.failed').replace(
                '{count}',
                String(importResult.failed_count)
              )}
            </p>
          </div>

          {#if importResult.errors.length > 0}
            <div class="mt-2 text-caption text-danger-fg max-h-40 overflow-y-auto">
              <p class="font-medium mb-1">{$t('settings.csvImport.errors')}</p>
              {#each importResult.errors.slice(0, 10) as error}
                <p>
                  • {error.row
                    ? $t('settings.csvImport.rowPrefix').replace('{row}', String(error.row))
                    : ''}{error.message}
                </p>
              {/each}
              {#if importResult.errors.length > 10}
                <p class="text-fg-muted mt-1">
                  {$t('settings.csvImport.moreErrors').replace(
                    '{count}',
                    String(importResult.errors.length - 10)
                  )}
                </p>
              {/if}
            </div>
          {/if}
        </div>
      {/if}

      {#if csvError}
        <p class="text-caption text-danger-fg mt-2">{csvError}</p>
      {/if}
    </div>
  </section>

  <section class="mt-10">
    <h2 class="text-heading font-semibold text-fg mb-4">{$t('settings.backup.section')}</h2>

    <!-- Create Backup -->
    <div class="bg-surface-hover rounded-card p-4 mb-4">
      <div class="flex items-start justify-between gap-4">
        <div>
          <p class="text-body font-medium text-fg">{$t('settings.backup.createTitle')}</p>
          <p class="text-caption text-fg-muted mt-1">{$t('settings.backup.createDesc')}</p>
        </div>
        <button
          onclick={handleCreateBackup}
          disabled={creatingBackup}
          class="h-8 px-3 text-body rounded-control bg-accent hover:bg-accent-hover disabled:opacity-50 disabled:cursor-not-allowed text-on-accent transition-colors shrink-0"
        >
          {creatingBackup ? $t('settings.backup.creating') : $t('settings.backup.exportBackup')}
        </button>
      </div>
      {#if backupError}
        <p class="text-caption text-danger-fg mt-2">{backupError}</p>
      {/if}
    </div>

    <!-- Restore Backup -->
    <div class="bg-surface-hover rounded-card p-4">
      <div class="mb-3">
        <p class="text-body font-medium text-fg mb-1">{$t('settings.backup.restoreTitle')}</p>
        <p class="text-caption text-fg-muted">{$t('settings.backup.restoreDesc')}</p>
      </div>

      <div class="mb-3">
        <label for="restore-backup-file" class="block text-caption font-medium text-fg-muted mb-2">
          {$t('settings.backup.selectFile')}
        </label>
        <input
          id="restore-backup-file"
          type="file"
          accept=".dokassist"
          onchange={handleSelectRestoreFile}
          class="block w-full text-body text-fg border border-line-strong rounded-control cursor-pointer bg-surface-selected focus:outline-none"
        />
      </div>

      {#if validatedBackupInfo}
        <div class="mb-3 p-3 bg-success-subtle border border-success rounded-card">
          <p class="text-caption font-medium text-success-fg mb-2">
            {$t('settings.backup.validated')}
          </p>
          <div class="text-caption text-fg-muted space-y-1">
            <p>
              {$t('settings.backup.createdAt').replace(
                '{date}',
                new Date(validatedBackupInfo.created_at).toLocaleString()
              )}
            </p>
            <p>
              {$t('settings.backup.fileCount').replace(
                '{count}',
                String(validatedBackupInfo.file_count)
              )}
            </p>
            <p>
              {$t('settings.backup.dbSchema').replace(
                '{version}',
                String(validatedBackupInfo.db_schema_version)
              )}
            </p>
          </div>
        </div>

        {#if !showRestoreConfirm}
          <button
            onclick={() => {
              showRestoreConfirm = true;
              restoreInput = '';
              restoreError = '';
            }}
            class="h-8 px-3 text-body rounded-control bg-warning hover:bg-warning text-on-warning transition-colors"
          >
            {$t('settings.backup.restoreFromThis')}
          </button>
        {/if}

        {#if showRestoreConfirm}
          <div class="mt-3 border-t border-warning-line pt-3">
            <p class="text-body text-warning-fg mb-3">
              <strong>{$t('settings.backup.warningLabel')}</strong>
              {$t('settings.backup.warningBody')}
              <strong>RESTORE</strong>
              {$t('settings.backup.warningConfirm')}
            </p>
            <div class="flex gap-2">
              <input
                type="text"
                bind:value={restoreInput}
                placeholder="RESTORE"
                class="flex-1 px-3 py-2 text-body rounded-control bg-surface-selected border border-line-strong text-fg focus:outline-none focus:border-warning"
                onkeydown={(e) => {
                  if (e.key === 'Enter' && restoreInput === 'RESTORE') handleRestoreBackup();
                }}
              />
              <button
                onclick={handleRestoreBackup}
                disabled={restoring || restoreInput !== 'RESTORE'}
                class="h-8 px-3 text-body rounded-control bg-warning hover:bg-warning disabled:opacity-50 disabled:cursor-not-allowed text-on-warning transition-colors shrink-0"
              >
                {restoring ? $t('settings.backup.restoring') : $t('settings.backup.confirmRestore')}
              </button>
              <button
                onclick={() => {
                  showRestoreConfirm = false;
                  restoreInput = '';
                  restoreError = '';
                }}
                class="h-8 px-3 text-body rounded-control bg-surface-selected hover:bg-surface-selected text-fg transition-colors shrink-0"
              >
                {$t('common.cancel')}
              </button>
            </div>
          </div>
        {/if}
      {/if}

      {#if restoreError}
        <p class="text-caption text-danger-fg mt-2">{restoreError}</p>
      {/if}
    </div>
  </section>

  <section class="mt-10">
    <h2 class="text-heading font-semibold text-fg mb-2">
      {$t('settings.about')}
    </h2>
    <p class="text-body text-fg-muted">
      {$t('settings.appVersion')}:
      <span class="text-fg">{appVersion || '…'}</span>
    </p>
  </section>

  <section class="mt-10">
    <h2 class="text-heading font-semibold text-danger-fg mb-4">{$t('settings.dangerZone')}</h2>

    <!-- Emergency Export -->
    <div class="border border-warning rounded-card p-4 mb-4">
      <div class="flex items-start justify-between gap-4">
        <div>
          <p class="text-body font-medium text-fg">
            {$t('settings.emergencyExport')}
          </p>
          <p class="text-caption text-fg-muted mt-1">
            {$t('settings.emergencyExportDesc')}
          </p>
        </div>
        {#if !showExportConfirm}
          <button
            onclick={() => {
              showExportConfirm = true;
              exportInput = '';
              exportError = '';
            }}
            class="h-8 px-3 text-body rounded-control bg-warning hover:bg-warning text-on-warning transition-colors shrink-0"
          >
            {$t('settings.exportData')}
          </button>
        {/if}
      </div>

      {#if showExportConfirm}
        <div class="mt-4 border-t border-warning-line pt-4">
          <p class="text-body text-warning-fg mb-3">
            {$t('settings.exportConfirmHint')}
          </p>
          <div class="flex gap-2">
            <input
              type="text"
              bind:value={exportInput}
              placeholder={$t('settings.exportConfirmWord')}
              class="flex-1 px-3 py-2 text-body rounded-control bg-surface-selected border border-line-strong text-fg focus:outline-none focus:border-warning"
              onkeydown={(e) => {
                if (e.key === 'Enter' && exportInput === $t('settings.exportConfirmWord'))
                  handleExport();
              }}
            />
            <button
              onclick={handleExport}
              disabled={exporting || exportInput !== $t('settings.exportConfirmWord')}
              class="h-8 px-3 text-body rounded-control bg-warning hover:bg-warning disabled:opacity-50 disabled:cursor-not-allowed text-on-warning transition-colors shrink-0"
            >
              {exporting ? $t('settings.exporting') : $t('settings.confirmExport')}
            </button>
            <button
              onclick={() => {
                showExportConfirm = false;
                exportInput = '';
                exportError = '';
              }}
              class="h-8 px-3 text-body rounded-control bg-surface-selected hover:bg-surface-selected text-fg transition-colors shrink-0"
            >
              {$t('common.cancel')}
            </button>
          </div>
          {#if exportError}
            <p class="text-caption text-danger-fg mt-2">{exportError}</p>
          {/if}
        </div>
      {/if}
    </div>

    <!-- Factory Reset -->
    <div class="border border-danger-line rounded-card p-4">
      <div class="flex items-start justify-between gap-4">
        <div>
          <p class="text-body font-medium text-fg">
            {$t('settings.factoryReset')}
          </p>
          <p class="text-caption text-fg-muted mt-1">
            {$t('settings.factoryResetShortDesc')}
          </p>
        </div>
        {#if !showResetConfirm}
          <button
            onclick={() => {
              showResetConfirm = true;
              resetInput = '';
              resetError = '';
            }}
            class="h-8 px-3 text-body rounded-control bg-danger hover:bg-danger-hover text-on-danger transition-colors shrink-0"
          >
            {$t('settings.factoryReset')}
          </button>
        {/if}
      </div>

      {#if showResetConfirm}
        <div class="mt-4 border-t border-danger-line pt-4">
          <p class="text-body text-danger-fg mb-3">
            {$t('settings.resetConfirmHint')}
          </p>
          <div class="flex gap-2">
            <input
              type="text"
              bind:value={resetInput}
              placeholder={$t('settings.resetConfirmWord')}
              class="flex-1 px-3 py-2 text-body rounded-control bg-surface-selected border border-line-strong text-fg focus:outline-none focus:border-danger"
              onkeydown={(e) => {
                if (e.key === 'Enter' && resetInput === $t('settings.resetConfirmWord'))
                  handleReset();
              }}
            />
            <button
              onclick={handleReset}
              disabled={resetting || resetInput !== $t('settings.resetConfirmWord')}
              class="h-8 px-3 text-body rounded-control bg-danger hover:bg-danger-hover disabled:opacity-50 disabled:cursor-not-allowed text-on-danger transition-colors shrink-0"
            >
              {resetting ? $t('settings.resetting') : $t('settings.confirmResetAction')}
            </button>
            <button
              onclick={() => {
                showResetConfirm = false;
                resetInput = '';
                resetError = '';
              }}
              class="h-8 px-3 text-body rounded-control bg-surface-selected hover:bg-surface-selected text-fg transition-colors shrink-0"
            >
              {$t('common.cancel')}
            </button>
          </div>
          {#if resetError}
            <p class="text-caption text-danger-fg mt-2">{resetError}</p>
          {/if}
        </div>
      {/if}
    </div>
  </section>
</div>

<style>
  .settings-page {
    display: grid;
    grid-template-columns: repeat(12, minmax(0, 1fr));
    gap: 1.5rem;
    align-items: start;
  }

  .settings-page > :global(header),
  .settings-page > :global(section) {
    grid-column: span 12;
    min-width: 0;
  }

  .settings-page > :global(section) {
    margin: 0;
    padding: 1.25rem;
    border: 1px solid var(--line-subtle);
    border-radius: var(--radius-card);
    background: var(--surface-raised);
  }

  @media (min-width: 768px) {
    .settings-page > :global(section) {
      padding: 1.5rem;
    }

    .settings-page > :global(section:nth-of-type(2)) {
      display: grid;
      grid-template-columns: repeat(2, minmax(0, 1fr));
      gap: 1rem;
    }

    .settings-page > :global(section:nth-of-type(2) > h2) {
      grid-column: 1 / -1;
      margin-bottom: 0;
    }

    .settings-page > :global(section:nth-of-type(2) > div) {
      margin: 0;
    }
  }

  @media (min-width: 1280px) {
    .settings-page > :global(section:nth-of-type(1)) {
      grid-column: span 4;
    }

    .settings-page > :global(section:nth-of-type(2)) {
      grid-column: span 8;
    }

    .settings-page > :global(section:nth-of-type(3)) {
      grid-column: span 12;
    }

    .settings-page > :global(section:nth-of-type(4)),
    .settings-page > :global(section:nth-of-type(5)),
    .settings-page > :global(section:nth-of-type(6)),
    .settings-page > :global(section:nth-of-type(7)) {
      grid-column: span 6;
    }

    .settings-page > :global(section:nth-of-type(8)) {
      grid-column: span 4;
    }

    .settings-page > :global(section:nth-of-type(9)) {
      grid-column: span 8;
    }
  }
</style>
