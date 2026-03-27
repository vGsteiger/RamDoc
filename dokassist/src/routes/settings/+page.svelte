<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { listen, type UnlistenFn } from '@tauri-apps/api/event';
  import { getVersion } from '@tauri-apps/api/app';
  import { open } from '@tauri-apps/plugin-dialog';
  import { goto } from '$app/navigation';
  import {
    getEngineStatus,
    getRecommendedModel,
    downloadModel,
    loadModel,
    resetApp,
    parseError,
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
    parseCsvPreview,
    importCsvData,
    type LlmEngineStatus,
    type ModelChoice,
    type UpdateInfo,
    type EmbedStatus,
    type ModelInfo,
    type TaskModel,
    type CsvPreview,
    type ColumnMapping,
    type ImportResult,
  } from '$lib/api';
  import { themePreference } from '$lib/stores/theme';
  import { language } from '$lib/stores/language';
  import { t } from '$lib/translations';

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
        if (/^- /.test(line))
          return `<p class="ml-3">&bull; ${inline(escape(line.slice(2)))}</p>`;
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

  // Model management state
  let installedModels = $state<ModelInfo[]>([]);
  let taskModels = $state<TaskModel[]>([]);
  let selectedTaskModel = $state<Record<string, string>>({});
  let modelManagementError = $state('');
  let loadingModels = $state(false);

  // Embedding model state
  let embedStatus = $state<EmbedStatus | null>(null);
  let embedPhase = $state<'idle' | 'loading' | 'done' | 'error'>('idle');
  let embedError = $state('');

  // Update state
  let updateInfo = $state<UpdateInfo | null>(null);
  let checkingUpdate = $state(false);
  let installingUpdate = $state(false);
  let updateProgress = $state<number>(0);
  let updateError = $state('');
  let updateUnlisten: UnlistenFn | null = null;

  onMount(async () => {
    [status, recommended, appVersion, embedStatus] = await Promise.all([
      getEngineStatus(),
      getRecommendedModel(),
      getVersion().catch(() => 'Unknown'),
      getEmbedStatus(),
    ]);
    if (status.is_loaded) phase = 'done';
    if (embedStatus.is_loaded) embedPhase = 'done';

    // Load installed models and task assignments
    await loadInstalledModels();
  });

  async function loadInstalledModels() {
    try {
      loadingModels = true;
      [installedModels, taskModels] = await Promise.all([listModels(), listTaskModels()]);

      // Build a map of task -> model_id for easy lookup
      selectedTaskModel = taskModels.reduce((acc, tm) => {
        acc[tm.task_type] = tm.model_id;
        return acc;
      }, {} as Record<string, string>);
    } catch (e) {
      modelManagementError = parseError(e).message;
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
      embedError = parseError(e).message;
    }
  }

  onDestroy(() => {
    unlisten?.();
    doneUnsubscribe?.();
    updateUnlisten?.();
  });

  async function handleCheckForUpdates() {
    checkingUpdate = true;
    updateError = '';
    try {
      updateInfo = await checkForUpdates();
    } catch (e) {
      updateError = parseError(e).message;
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
      updateError = parseError(e).message;
    }
  }

  function formatBytes(bytes: number): string {
    const gb = bytes / 1024 ** 3;
    return `${gb.toFixed(1)} GB`;
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
      errorMsg = parseError(e).message;
    }
  }

  async function handleLoad() {
    if (!recommended) return;
    phase = 'loading';
    errorMsg = '';
    try {
      await loadModel(recommended.filename);
      status = await getEngineStatus();
      phase = 'done';
    } catch (e) {
      phase = 'error';
      errorMsg = parseError(e).message;
    }
  }

  // Model management handlers
  let doneUnsubscribe: UnlistenFn | null = null;

  async function handleDownloadNewModel(model: ModelChoice) {
    phase = 'downloading';
    downloadProgress = 0;
    errorMsg = '';

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
      errorMsg = parseError(e).message;
    } finally {
      unlisten?.();
      unlisten = null;
      doneUnsubscribe?.();
      doneUnsubscribe = null;
    }
  }

  async function handleLoadModel(filename: string) {
    phase = 'loading';
    errorMsg = '';
    try {
      await loadModel(filename);
      status = await getEngineStatus();
      await loadInstalledModels(); // Refresh list to show loaded badge
      phase = 'done';
    } catch (e) {
      phase = 'error';
      errorMsg = parseError(e).message;
    }
  }

  async function handleDeleteModel(modelId: string) {
    try {
      await deleteModel(modelId);
      await loadInstalledModels();
    } catch (e) {
      modelManagementError = parseError(e).message;
    }
  }

  async function handleSetDefaultModel(modelId: string) {
    try {
      await setDefaultModel(modelId);
      await loadInstalledModels();
    } catch (e) {
      modelManagementError = parseError(e).message;
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
      modelManagementError = parseError(e).message;
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
  let backupError = $state("");
  let restoring = $state(false);
  let showRestoreConfirm = $state(false);
  let restoreInput = $state("");
  let restoreError = $state("");
  let selectedBackupFile: File | null = null;
  let validatedBackupInfo: BackupInfo | null = null;

  // CSV Import state
  let selectedCsvPath = $state<string | null>(null);
  let csvPreview = $state<CsvPreview | null>(null);
  let csvError = $state("");
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
      resetError = parseError(e).message;
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
      exportError = parseError(e).message;
    } finally {
      exporting = false;
    }
  }

  async function handleCreateBackup() {
    creatingBackup = true;
    backupError = "";
    try {
      const backupData = await createVaultBackup();

      // Convert number array to Uint8Array and create download
      const blob = new Blob([new Uint8Array(backupData)], {
        type: "application/octet-stream",
      });
      const url = URL.createObjectURL(blob);
      const a = document.createElement("a");
      a.href = url;
      a.download = `RamDoc_Backup_${new Date().toISOString().split('T')[0]}.dokassist`;
      document.body.appendChild(a);
      a.click();
      document.body.removeChild(a);
      URL.revokeObjectURL(url);
    } catch (e) {
      backupError = parseError(e).message;
    } finally {
      creatingBackup = false;
    }
  }

  async function handleSelectRestoreFile(
    event: Event & { currentTarget: HTMLInputElement },
  ) {
    const file = event.currentTarget.files?.[0];
    if (!file) {
      selectedBackupFile = null;
      validatedBackupInfo = null;
      return;
    }

    selectedBackupFile = file;
    restoreError = "";

    // Validate the backup file
    try {
      const arrayBuffer = await file.arrayBuffer();
      const backupArray = Array.from(new Uint8Array(arrayBuffer));
      validatedBackupInfo = await validateBackupArchive(backupArray);
    } catch (e) {
      restoreError = parseError(e).message;
      selectedBackupFile = null;
      validatedBackupInfo = null;
    }
  }

  async function handleRestoreBackup() {
    if (!selectedBackupFile || !validatedBackupInfo) return;

    restoring = true;
    restoreError = "";
    try {
      const arrayBuffer = await selectedBackupFile.arrayBuffer();
      const backupArray = Array.from(new Uint8Array(arrayBuffer));
      await restoreVaultBackup(backupArray);

      // Reset state
      showRestoreConfirm = false;
      restoreInput = "";
      selectedBackupFile = null;
      validatedBackupInfo = null;

      // Redirect to unlock page since database was replaced
      goto("/");
    } catch (e) {
      restoreError = parseError(e).message;
    } finally {
      restoring = false;
    }
  }

  async function handleSelectCsvFile() {
    try {
      const selected = await open({
        title: 'Select CSV file',
        filters: [{
          name: 'CSV',
          extensions: ['csv']
        }],
        multiple: false
      });

      if (!selected) {
        selectedCsvPath = null;
        csvPreview = null;
        return;
      }

      selectedCsvPath = selected as string;
      csvError = "";
      csvPreview = null;
      importResult = null;

      // Parse CSV preview
      csvPreview = await parseCsvPreview(selectedCsvPath);
      columnMappings = csvPreview.detected_mappings;
    } catch (e) {
      csvError = parseError(e).message;
      selectedCsvPath = null;
    }
  }

  async function handleImportCsv() {
    if (!selectedCsvPath || !csvPreview) return;

    importing = true;
    csvError = "";
    try {
      importResult = await importCsvData(selectedCsvPath, columnMappings);

      if (importResult.success) {
        // Clear state on success
        selectedCsvPath = null;
        csvPreview = null;
        columnMappings = [];
      }
    } catch (e) {
      csvError = parseError(e).message;
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
      columnMappings = [
        ...columnMappings,
        { csv_header: csvHeader, patient_field: patientField }
      ];
    }
  }

  // Check if all required fields are mapped
  function hasAllRequiredFieldsMapped(): boolean {
    const requiredFields = ['ahv_number', 'first_name', 'last_name', 'date_of_birth'];
    const mappedFields = new Set(columnMappings.map(m => m.patient_field).filter(f => f));
    return requiredFields.every(field => mappedFields.has(field));
  }

  const patientFields = [
    { value: "", label: "(Skip)" },
    { value: "ahv_number", label: "AHV Number *" },
    { value: "first_name", label: "First Name *" },
    { value: "last_name", label: "Last Name *" },
    { value: "date_of_birth", label: "Date of Birth *" },
    { value: "gender", label: "Gender" },
    { value: "address", label: "Address" },
    { value: "phone", label: "Phone" },
    { value: "email", label: "Email" },
    { value: "insurance", label: "Insurance" },
    { value: "gp_name", label: "GP Name" },
    { value: "gp_address", label: "GP Address" },
    { value: "notes", label: "Notes" },
  ];
</script>

<div class="p-8 max-w-xl">
  <h1 class="text-2xl font-bold text-gray-900 dark:text-gray-100 mb-6">{$t('settings.title')}</h1>

  <section class="mb-10">
    <h2 class="text-lg font-semibold text-gray-900 dark:text-gray-200 mb-4">
      {$t('settings.applicationUpdates')}
    </h2>

    <div class="bg-gray-100 dark:bg-gray-800 rounded-lg p-4 mb-4">
      <div class="flex items-center justify-between mb-3">
        <div>
          <p class="text-sm font-medium text-gray-900 dark:text-gray-100">
            {$t('settings.currentVersion')}
          </p>
          <p class="text-xs text-gray-500 dark:text-gray-400 mt-1">
            {appVersion || $t('common.loading')}
          </p>
        </div>
        <button
          onclick={handleCheckForUpdates}
          disabled={checkingUpdate || installingUpdate}
          class="px-4 py-2 text-sm rounded-lg bg-blue-600 hover:bg-blue-500 disabled:opacity-50 disabled:cursor-not-allowed text-white transition-colors"
        >
          {checkingUpdate ? $t('settings.checking') : $t('settings.checkForUpdates')}
        </button>
      </div>

      {#if updateInfo}
        {#if updateInfo.update_available}
          <div class="border-t border-gray-300 dark:border-gray-700 pt-3 mt-3">
            <div class="flex items-start justify-between gap-4 mb-2">
              <div>
                <p class="text-sm font-medium text-green-400">
                  {$t('settings.updateAvailable')}
                </p>
                <p class="text-xs text-gray-600 dark:text-gray-400 mt-1">
                  {$t('settings.version')}
                  {updateInfo.latest_version}
                  {$t('settings.versionAvailable')}
                </p>
              </div>
            </div>

            {#if updateInfo.body}
              <div
                class="text-xs text-gray-600 dark:text-gray-400 mb-3 max-h-32 overflow-y-auto bg-gray-200 dark:bg-gray-900 rounded p-2"
              >
                <p class="font-medium mb-1">{$t('settings.releaseNotes')}</p>
                <div>{@html renderMarkdown(updateInfo.body)}</div>
              </div>
            {/if}

            {#if installingUpdate}
              <div class="mb-3">
                <div class="flex justify-between text-xs text-gray-600 dark:text-gray-400 mb-1">
                  <span>{$t('settings.downloadingAndInstalling')}</span>
                  <span>{updateProgress}%</span>
                </div>
                <div class="w-full bg-gray-300 dark:bg-gray-700 rounded-full h-2">
                  <div
                    class="bg-blue-500 h-2 rounded-full transition-all"
                    style="width: {updateProgress}%"
                  ></div>
                </div>
                <p class="text-xs text-gray-600 dark:text-gray-400 mt-2">
                  {$t('settings.autoRestart')}
                </p>
              </div>
            {/if}

            {#if updateError}
              <p class="text-xs text-red-400 mb-3">{updateError}</p>
            {/if}

            {#if !installingUpdate}
              <button
                onclick={handleInstallUpdate}
                class="px-4 py-2 text-sm rounded-lg bg-green-600 hover:bg-green-500 text-white transition-colors"
              >
                {$t('settings.installUpdate')}
              </button>
            {/if}
          </div>
        {:else}
          <div class="border-t border-gray-300 dark:border-gray-700 pt-3 mt-3">
            <p class="text-sm text-green-400">{$t('settings.noUpdatesAvailable')}</p>
          </div>
        {/if}
      {/if}

      {#if updateError && !updateInfo}
        <div class="border-t border-gray-300 dark:border-gray-700 pt-3 mt-3">
          <p class="text-xs text-red-400">{updateError}</p>
        </div>
      {/if}
    </div>
  </section>

  <section class="mb-10">
    <h2 class="text-lg font-semibold text-gray-900 dark:text-gray-200 mb-4">
      {$t('settings.appearance')}
    </h2>

    <div class="bg-gray-100 dark:bg-gray-800 rounded-lg p-4 mb-4">
      <p class="text-sm font-medium text-gray-900 dark:text-gray-100 mb-3">
        {$t('settings.theme')}
      </p>
      <p class="text-xs text-gray-600 dark:text-gray-400 mb-4">
        {$t('settings.themeDescription')}
      </p>

      <div class="space-y-2">
        <label
          class="flex items-center gap-3 p-3 rounded-lg border border-gray-300 dark:border-gray-700 cursor-pointer hover:bg-gray-200 dark:hover:bg-gray-700 transition-colors {$themePreference ===
          'light'
            ? 'bg-gray-200 dark:bg-gray-700 border-blue-500'
            : ''}"
        >
          <input
            type="radio"
            name="theme"
            value="light"
            checked={$themePreference === 'light'}
            onchange={() => themePreference.set('light')}
            class="w-4 h-4 text-blue-600 focus:ring-blue-500"
          />
          <div>
            <p class="text-sm font-medium text-gray-900 dark:text-gray-100">
              {$t('settings.light')}
            </p>
            <p class="text-xs text-gray-600 dark:text-gray-400">
              {$t('settings.lightDescription')}
            </p>
          </div>
        </label>

        <label
          class="flex items-center gap-3 p-3 rounded-lg border border-gray-300 dark:border-gray-700 cursor-pointer hover:bg-gray-200 dark:hover:bg-gray-700 transition-colors {$themePreference ===
          'dark'
            ? 'bg-gray-200 dark:bg-gray-700 border-blue-500'
            : ''}"
        >
          <input
            type="radio"
            name="theme"
            value="dark"
            checked={$themePreference === 'dark'}
            onchange={() => themePreference.set('dark')}
            class="w-4 h-4 text-blue-600 focus:ring-blue-500"
          />
          <div>
            <p class="text-sm font-medium text-gray-900 dark:text-gray-100">
              {$t('settings.dark')}
            </p>
            <p class="text-xs text-gray-600 dark:text-gray-400">{$t('settings.darkDescription')}</p>
          </div>
        </label>

        <label
          class="flex items-center gap-3 p-3 rounded-lg border border-gray-300 dark:border-gray-700 cursor-pointer hover:bg-gray-200 dark:hover:bg-gray-700 transition-colors {$themePreference ===
          'system'
            ? 'bg-gray-200 dark:bg-gray-700 border-blue-500'
            : ''}"
        >
          <input
            type="radio"
            name="theme"
            value="system"
            checked={$themePreference === 'system'}
            onchange={() => themePreference.set('system')}
            class="w-4 h-4 text-blue-600 focus:ring-blue-500"
          />
          <div>
            <p class="text-sm font-medium text-gray-900 dark:text-gray-100">
              {$t('settings.system')}
            </p>
            <p class="text-xs text-gray-600 dark:text-gray-400">
              {$t('settings.systemDescription')}
            </p>
          </div>
        </label>
      </div>
    </div>

    <div class="bg-gray-100 dark:bg-gray-800 rounded-lg p-4">
      <p class="text-sm font-medium text-gray-900 dark:text-gray-100 mb-3">
        {$t('settings.language')}
      </p>
      <p class="text-xs text-gray-600 dark:text-gray-400 mb-4">
        {$t('settings.languageDescription')}
      </p>

      <div class="space-y-2">
        <label
          class="flex items-center gap-3 p-3 rounded-lg border border-gray-300 dark:border-gray-700 cursor-pointer hover:bg-gray-200 dark:hover:bg-gray-700 transition-colors {$language ===
          'en'
            ? 'bg-gray-200 dark:bg-gray-700 border-blue-500'
            : ''}"
        >
          <input
            type="radio"
            name="language"
            value="en"
            checked={$language === 'en'}
            onchange={() => language.set('en')}
            class="w-4 h-4 text-blue-600 focus:ring-blue-500"
          />
          <div>
            <p class="text-sm font-medium text-gray-900 dark:text-gray-100">
              {$t('settings.english')}
            </p>
            <p class="text-xs text-gray-600 dark:text-gray-400">English</p>
          </div>
        </label>

        <label
          class="flex items-center gap-3 p-3 rounded-lg border border-gray-300 dark:border-gray-700 cursor-pointer hover:bg-gray-200 dark:hover:bg-gray-700 transition-colors {$language ===
          'de'
            ? 'bg-gray-200 dark:bg-gray-700 border-blue-500'
            : ''}"
        >
          <input
            type="radio"
            name="language"
            value="de"
            checked={$language === 'de'}
            onchange={() => language.set('de')}
            class="w-4 h-4 text-blue-600 focus:ring-blue-500"
          />
          <div>
            <p class="text-sm font-medium text-gray-900 dark:text-gray-100">
              {$t('settings.german')}
            </p>
            <p class="text-xs text-gray-600 dark:text-gray-400">Deutsch</p>
          </div>
        </label>
      </div>
    </div>
  </section>

<!-- Enhanced Model Management Section -->
<section>
  <h2 class="text-lg font-semibold text-gray-900 dark:text-gray-200 mb-4">
    {$t('settings.modelManagement')}
  </h2>

  <!-- Currently loaded model status -->
  <div class="bg-gray-100 dark:bg-gray-800 rounded-lg p-4 mb-6">
    <div class="flex items-center gap-3 mb-4">
      <div
        class="w-3 h-3 rounded-full shrink-0 {status?.is_loaded ? 'bg-green-500' : 'bg-gray-400'}"
      ></div>
      <div class="flex-1">
        <p class="text-sm font-medium text-gray-900 dark:text-gray-100">
          {status?.is_loaded
            ? $t('settings.loadedModel').replace('{name}', status.model_name ?? '')
            : $t('settings.noModelLoaded')}
        </p>
        {#if status?.total_ram_bytes}
          <p class="text-xs text-gray-600 dark:text-gray-400">
            {$t('settings.systemRam').replace('{ram}', formatBytes(status.total_ram_bytes))}
          </p>
        {/if}
      </div>
    </div>
  </div>

  <!-- Installed Models List -->
  <div class="mb-6">
    <h3 class="text-md font-semibold text-gray-900 dark:text-gray-200 mb-3">
      {$t('settings.installedModels')}
    </h3>

    {#if loadingModels}
      <p class="text-sm text-gray-600 dark:text-gray-400">{$t('settings.loadingModels')}</p>
    {:else if installedModels.length === 0}
      <p class="text-sm text-gray-600 dark:text-gray-400 mb-4">
        {$t('settings.noModelsInstalled')}
      </p>
    {:else}
      <div class="space-y-3">
        {#each installedModels as model}
          <div
            class="bg-gray-100 dark:bg-gray-800 border border-gray-300 dark:border-gray-700 rounded-lg p-4"
          >
            <div class="flex items-start justify-between mb-2">
              <div class="flex-1">
                <div class="flex items-center gap-2 mb-1">
                  <p class="text-sm font-medium text-gray-900 dark:text-gray-100">
                    {model.name}
                  </p>
                  {#if model.is_default}
                    <span class="px-2 py-0.5 text-xs rounded bg-blue-500 text-white">
                      {$t('settings.defaultBadge')}
                    </span>
                  {/if}
                  {#if model.is_loaded}
                    <span class="px-2 py-0.5 text-xs rounded bg-green-500 text-white">
                      {$t('settings.loadedBadge')}
                    </span>
                  {/if}
                  {#if !model.exists_on_disk}
                    <span class="px-2 py-0.5 text-xs rounded bg-red-500 text-white">
                      {$t('settings.modelMissingOnDisk')}
                    </span>
                  {/if}
                </div>
                <p class="text-xs text-gray-600 dark:text-gray-400">
                  {model.filename} • {formatBytes(model.size_bytes)}
                </p>
                {#if model.last_used}
                  <p class="text-xs text-gray-500 dark:text-gray-500 mt-1">
                    {$t('settings.lastUsed').replace(
                      '{date}',
                      new Date(model.last_used).toLocaleDateString()
                    )}
                  </p>
                {/if}
              </div>
            </div>

            <div class="flex gap-2 mt-3">
              {#if model.exists_on_disk && !model.is_loaded}
                <button
                  onclick={() => handleLoadModel(model.filename)}
                  disabled={phase === 'loading'}
                  class="px-3 py-1.5 text-xs rounded-lg bg-blue-600 hover:bg-blue-500 disabled:opacity-50 disabled:cursor-not-allowed text-white transition-colors"
                >
                  {$t('settings.load')}
                </button>
              {/if}
              {#if model.exists_on_disk && !model.is_default}
                <button
                  onclick={() => handleSetDefaultModel(model.id)}
                  class="px-3 py-1.5 text-xs rounded-lg bg-gray-300 dark:bg-gray-700 hover:bg-gray-400 dark:hover:bg-gray-600 text-gray-900 dark:text-gray-100 transition-colors"
                >
                  {$t('settings.setDefault')}
                </button>
              {/if}
              {#if !model.is_loaded}
                <button
                  onclick={() => handleDeleteModel(model.id)}
                  class="px-3 py-1.5 text-xs rounded-lg bg-red-600 hover:bg-red-500 text-white transition-colors"
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

  <!-- Download New Model -->
  {#if recommended}
    <div class="mb-6">
      <h3 class="text-md font-semibold text-gray-900 dark:text-gray-200 mb-3">
        {$t('settings.recommendedModelSection')}
      </h3>
      <div
        class="bg-gray-100 dark:bg-gray-800 border border-gray-300 dark:border-gray-700 rounded-lg p-4"
      >
        <div class="flex items-start justify-between gap-4 mb-2">
          <div>
            <p class="text-sm font-medium text-gray-900 dark:text-gray-100">
              {recommended.name}
            </p>
            <p class="text-xs text-gray-600 dark:text-gray-400 mt-1">
              {recommended.reason}
            </p>
            <p class="text-xs text-gray-500 dark:text-gray-500 mt-1">
              {$t('settings.size').replace('{size}', formatBytes(recommended.size_bytes))}
            </p>
          </div>
        </div>

        {#if phase === 'downloading'}
          <div class="mb-3">
            <div
              class="flex justify-between text-xs text-gray-600 dark:text-gray-400 mb-1"
            >
              <span>{$t('settings.downloadingLabel')}</span>
              <span>{downloadProgress ?? 0}%</span>
            </div>
            <div class="w-full bg-gray-300 dark:bg-gray-700 rounded-full h-2">
              <div
                class="bg-blue-500 h-2 rounded-full transition-all"
                style="width: {downloadProgress ?? 0}%"
              ></div>
            </div>
          </div>
        {/if}

        {#if phase === 'error'}
          <p class="text-xs text-red-400 mb-3">{errorMsg}</p>
        {/if}

        <button
          onclick={() => handleDownloadNewModel(recommended)}
          disabled={phase === 'downloading' || phase === 'loading'}
          class="px-4 py-2 text-sm rounded-lg bg-blue-600 hover:bg-blue-500 disabled:opacity-50 disabled:cursor-not-allowed text-white transition-colors"
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
    <h3 class="text-md font-semibold text-gray-900 dark:text-gray-200 mb-3">
      {$t('settings.taskSpecificModels')}
    </h3>
    <p class="text-xs text-gray-600 dark:text-gray-400 mb-4">
      {$t('settings.taskSpecificModelsDesc')}
    </p>

    {#if installedModels.length > 0}
      <div class="space-y-3">
        {#each ['summary', 'letter', 'report'] as taskType}
          <div
            class="bg-gray-100 dark:bg-gray-800 border border-gray-300 dark:border-gray-700 rounded-lg p-4"
          >
            <label
              for="task-{taskType}"
              class="block text-sm font-medium text-gray-900 dark:text-gray-100 mb-2 capitalize"
            >
              {$t(`settings.${taskType}`)}
            </label>
            <select
              id="task-{taskType}"
              value={selectedTaskModel[taskType] || ''}
              onchange={(e) => handleSetTaskModel(taskType, e.currentTarget.value)}
              class="w-full px-3 py-2 text-sm border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-900 text-gray-900 dark:text-gray-100 focus:ring-2 focus:ring-blue-500 focus:border-transparent"
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
      <p class="text-xs text-gray-600 dark:text-gray-400">
        {$t('settings.installModelFirst')}
      </p>
    {/if}
  </div>

  {#if modelManagementError}
    <div
      class="bg-red-100 dark:bg-red-900/20 border border-red-300 dark:border-red-800 rounded-lg p-3 mb-4"
    >
      <p class="text-sm text-red-600 dark:text-red-400">{modelManagementError}</p>
    </div>
  {/if}
</section>

  <section class="mt-10">
    <h2 class="text-lg font-semibold text-gray-900 dark:text-gray-200 mb-4">
      {$t('settings.embeddingModel')}
    </h2>
    <p class="text-xs text-gray-600 dark:text-gray-400 mb-4">
      {$t('settings.embeddingModelDesc')}
    </p>

    <div class="bg-gray-100 dark:bg-gray-800 rounded-lg p-4 mb-4 flex items-center gap-3">
      <div
        class="w-3 h-3 rounded-full shrink-0 {embedStatus?.is_loaded
          ? 'bg-green-500'
          : embedStatus?.is_downloaded
            ? 'bg-amber-400'
            : 'bg-red-500'}"
      ></div>
      <div class="flex-1">
        {#if embedStatus?.is_loaded}
          <p class="text-sm text-gray-900 dark:text-gray-100 font-medium">nomic-embed-text-v1.5</p>
          <p class="text-xs text-gray-600 dark:text-gray-400">{$t('settings.embeddingLoaded')}</p>
        {:else if embedStatus?.is_downloaded}
          <p class="text-sm text-gray-900 dark:text-gray-100 font-medium">
            {$t('settings.embeddingCached')}
          </p>
          <p class="text-xs text-gray-600 dark:text-gray-400">
            {$t('settings.embeddingCachedDesc')}
          </p>
        {:else}
          <p class="text-sm text-gray-900 dark:text-gray-100 font-medium">
            {$t('settings.embeddingNotDownloaded')}
          </p>
          <p class="text-xs text-gray-600 dark:text-gray-400">
            {$t('settings.embeddingNotDownloadedDesc')}
          </p>
        {/if}
      </div>

      {#if embedPhase !== 'done'}
        <button
          onclick={handleInitEmbed}
          disabled={embedPhase === 'loading'}
          class="px-4 py-2 text-sm rounded-lg bg-blue-600 hover:bg-blue-500 disabled:opacity-50 disabled:cursor-not-allowed text-white transition-colors shrink-0"
        >
          {embedPhase === 'loading' ? $t('common.loading') : $t('settings.loadNow')}
        </button>
      {/if}
    </div>

    {#if embedPhase === 'loading'}
      <p class="text-xs text-blue-400">
        {$t('settings.embeddingInitializing')}
      </p>
    {/if}
    {#if embedPhase === 'error'}
      <p class="text-xs text-red-400">{embedError}</p>
    {/if}
  </section>

  <section class="mt-10">
    <h2 class="text-lg font-semibold text-gray-900 dark:text-gray-200 mb-4">
      CSV Patient Import
    </h2>

    <div class="bg-gray-100 dark:bg-gray-800 rounded-lg p-4">
      <div class="mb-3">
        <p class="text-sm font-medium text-gray-900 dark:text-gray-100 mb-1">
          Import Patients from CSV
        </p>
        <p class="text-xs text-gray-600 dark:text-gray-400 mb-3">
          Import patient records from a CSV file. The wizard will detect columns and allow you to map them to patient fields.
        </p>
      </div>

      <div class="mb-3">
        <button
          onclick={handleSelectCsvFile}
          class="px-4 py-2 text-sm rounded-lg bg-blue-600 hover:bg-blue-500 text-white transition-colors"
        >
          Select CSV File
        </button>
        {#if selectedCsvPath}
          <p class="text-xs text-gray-600 dark:text-gray-400 mt-2">
            Selected: {selectedCsvPath.split('/').pop() || selectedCsvPath.split('\\').pop()}
          </p>
        {/if}
      </div>

      {#if csvPreview}
        <div class="mb-4 p-3 bg-blue-900/20 border border-blue-700 rounded-lg">
          <p class="text-xs font-medium text-blue-400 mb-2">
            ✓ CSV file parsed: {csvPreview.total_rows} rows detected
          </p>

          {#if csvPreview.warnings.length > 0}
            <div class="mb-3 text-xs text-amber-400">
              <p class="font-medium mb-1">Warnings:</p>
              {#each csvPreview.warnings as warning}
                <p>• {warning.row ? `Row ${warning.row}: ` : ''}{warning.message}</p>
              {/each}
            </div>
          {/if}

          <div class="mb-3">
            <p class="text-xs font-medium text-gray-300 mb-2">Column Mapping:</p>
            <div class="space-y-2">
              {#each csvPreview.headers as header}
                <div class="flex items-center gap-2">
                  <span class="text-xs text-gray-400 flex-1">{header}</span>
                  <span class="text-xs text-gray-500">→</span>
                  <select
                    value={columnMappings.find(m => m.csv_header === header)?.patient_field || ""}
                    onchange={(e) => updateColumnMapping(header, e.currentTarget.value)}
                    class="text-xs px-2 py-1 rounded bg-gray-800 border border-gray-600 text-gray-100 focus:outline-none focus:border-blue-500"
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
            <p class="text-xs font-medium text-gray-300 mb-2">Sample Data (first 3 rows):</p>
            <div class="overflow-x-auto">
              <table class="text-xs w-full">
                <thead>
                  <tr class="border-b border-gray-700">
                    {#each csvPreview.headers as header}
                      <th class="text-left px-2 py-1 text-gray-300">{header}</th>
                    {/each}
                  </tr>
                </thead>
                <tbody>
                  {#each csvPreview.sample_rows.slice(0, 3) as row}
                    <tr class="border-b border-gray-800">
                      {#each row as cell}
                        <td class="px-2 py-1 text-gray-400">{cell}</td>
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
            class="px-4 py-2 text-sm rounded-lg bg-green-600 hover:bg-green-500 disabled:opacity-50 disabled:cursor-not-allowed text-white transition-colors"
          >
            {importing ? "Importing…" : "Import Patients"}
          </button>

          {#if !hasAllRequiredFieldsMapped()}
            <p class="text-xs text-amber-400 mt-2">
              * Please map all required fields (AHV Number, First Name, Last Name, Date of Birth)
            </p>
          {/if}
        </div>
      {/if}

      {#if importResult}
        <div class="mb-3 p-3 {importResult.success ? 'bg-green-900/20 border border-green-700' : 'bg-amber-900/20 border border-amber-700'} rounded-lg">
          <p class="text-xs font-medium {importResult.success ? 'text-green-400' : 'text-amber-400'} mb-2">
            {importResult.success ? "✓" : "⚠"} Import completed
          </p>
          <div class="text-xs text-gray-300 space-y-1">
            <p>Imported: {importResult.imported_count}</p>
            <p>Failed: {importResult.failed_count}</p>
          </div>

          {#if importResult.errors.length > 0}
            <div class="mt-2 text-xs text-red-400 max-h-40 overflow-y-auto">
              <p class="font-medium mb-1">Errors:</p>
              {#each importResult.errors.slice(0, 10) as error}
                <p>• {error.row ? `Row ${error.row}: ` : ''}{error.message}</p>
              {/each}
              {#if importResult.errors.length > 10}
                <p class="text-gray-400 mt-1">... and {importResult.errors.length - 10} more errors</p>
              {/if}
            </div>
          {/if}
        </div>
      {/if}

      {#if csvError}
        <p class="text-xs text-red-400 mt-2">{csvError}</p>
      {/if}
    </div>
  </section>

  <section class="mt-10">
    <h2 class="text-lg font-semibold text-gray-900 dark:text-gray-200 mb-4">
      Encrypted Backup & Restore
    </h2>

    <!-- Create Backup -->
    <div class="bg-gray-100 dark:bg-gray-800 rounded-lg p-4 mb-4">
      <div class="flex items-start justify-between gap-4">
        <div>
          <p class="text-sm font-medium text-gray-900 dark:text-gray-100">
            Create Encrypted Backup
          </p>
          <p class="text-xs text-gray-600 dark:text-gray-400 mt-1">
            Export your entire vault (database + encrypted files) as a single
            encrypted .dokassist archive. The backup is encrypted with your master
            password and includes checksums for verification.
          </p>
        </div>
        <button
          onclick={handleCreateBackup}
          disabled={creatingBackup}
          class="px-4 py-2 text-sm rounded-lg bg-blue-600 hover:bg-blue-500 disabled:opacity-50 disabled:cursor-not-allowed text-white transition-colors shrink-0"
        >
          {creatingBackup ? "Creating…" : "Export Backup"}
        </button>
      </div>
      {#if backupError}
        <p class="text-xs text-red-400 mt-2">{backupError}</p>
      {/if}
    </div>

    <!-- Restore Backup -->
    <div class="bg-gray-100 dark:bg-gray-800 rounded-lg p-4">
      <div class="mb-3">
        <p class="text-sm font-medium text-gray-900 dark:text-gray-100 mb-1">
          Restore from Backup
        </p>
        <p class="text-xs text-gray-600 dark:text-gray-400">
          Restore your vault from a .dokassist backup archive. This will replace
          ALL current data with the backup contents.
        </p>
      </div>

      <div class="mb-3">
        <label
          class="block text-xs font-medium text-gray-700 dark:text-gray-300 mb-2"
        >
          Select Backup File (.dokassist)
        </label>
        <input
          type="file"
          accept=".dokassist"
          onchange={handleSelectRestoreFile}
          class="block w-full text-sm text-gray-900 dark:text-gray-100 border border-gray-400 dark:border-gray-600 rounded-lg cursor-pointer bg-gray-200 dark:bg-gray-900 focus:outline-none"
        />
      </div>

      {#if validatedBackupInfo}
        <div
          class="mb-3 p-3 bg-green-900/20 border border-green-700 rounded-lg"
        >
          <p class="text-xs font-medium text-green-400 mb-2">
            ✓ Backup validated successfully
          </p>
          <div class="text-xs text-gray-600 dark:text-gray-300 space-y-1">
            <p>
              Created: {new Date(validatedBackupInfo.created_at).toLocaleString()}
            </p>
            <p>Files: {validatedBackupInfo.file_count}</p>
            <p>DB Schema: v{validatedBackupInfo.db_schema_version}</p>
          </div>
        </div>

        {#if !showRestoreConfirm}
          <button
            onclick={() => {
              showRestoreConfirm = true;
              restoreInput = "";
              restoreError = "";
            }}
            class="px-4 py-2 text-sm rounded-lg bg-amber-700 hover:bg-amber-600 text-white transition-colors"
          >
            Restore from This Backup
          </button>
        {/if}

        {#if showRestoreConfirm}
          <div class="mt-3 border-t border-amber-700 pt-3">
            <p class="text-sm text-amber-300 mb-3">
              <strong>⚠️ WARNING:</strong> This will replace ALL current data with
              the backup. Type <strong>RESTORE</strong> to confirm.
            </p>
            <div class="flex gap-2">
              <input
                type="text"
                bind:value={restoreInput}
                placeholder="RESTORE"
                class="flex-1 px-3 py-2 text-sm rounded-lg bg-gray-200 dark:bg-gray-900 border border-gray-400 dark:border-gray-600 text-gray-900 dark:text-gray-100 placeholder-gray-500 focus:outline-none focus:border-amber-500"
                onkeydown={(e) => {
                  if (e.key === "Enter" && restoreInput === "RESTORE")
                    handleRestoreBackup();
                }}
              />
              <button
                onclick={handleRestoreBackup}
                disabled={restoring || restoreInput !== "RESTORE"}
                class="px-4 py-2 text-sm rounded-lg bg-amber-700 hover:bg-amber-600 disabled:opacity-50 disabled:cursor-not-allowed text-white transition-colors shrink-0"
              >
                {restoring ? "Restoring…" : "Confirm Restore"}
              </button>
              <button
                onclick={() => {
                  showRestoreConfirm = false;
                  restoreInput = "";
                  restoreError = "";
                }}
                class="px-4 py-2 text-sm rounded-lg bg-gray-300 dark:bg-gray-700 hover:bg-gray-400 dark:hover:bg-gray-600 text-gray-900 dark:text-gray-100 transition-colors shrink-0"
              >
                Cancel
              </button>
            </div>
          </div>
        {/if}
      {/if}

      {#if restoreError}
        <p class="text-xs text-red-400 mt-2">{restoreError}</p>
      {/if}
    </div>
  </section>

  <section class="mt-10">
    <h2 class="text-lg font-semibold text-gray-900 dark:text-gray-200 mb-2">
      {$t('settings.about')}
    </h2>
    <p class="text-sm text-gray-600 dark:text-gray-400">
      {$t('settings.appVersion')}:
      <span class="text-gray-900 dark:text-gray-100">{appVersion || '…'}</span>
    </p>
  </section>

  <section class="mt-10">
    <h2 class="text-lg font-semibold text-red-400 mb-4">{$t('settings.dangerZone')}</h2>

    <!-- Emergency Export -->
    <div class="border border-amber-600 rounded-lg p-4 mb-4">
      <div class="flex items-start justify-between gap-4">
        <div>
          <p class="text-sm font-medium text-gray-900 dark:text-gray-100">
            {$t('settings.emergencyExport')}
          </p>
          <p class="text-xs text-gray-600 dark:text-gray-400 mt-1">
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
            class="px-4 py-2 text-sm rounded-lg bg-amber-700 hover:bg-amber-600 text-white transition-colors shrink-0"
          >
            {$t('settings.exportData')}
          </button>
        {/if}
      </div>

      {#if showExportConfirm}
        <div class="mt-4 border-t border-amber-700 pt-4">
          <p class="text-sm text-amber-300 mb-3">
            {$t('settings.exportConfirmHint')}
          </p>
          <div class="flex gap-2">
            <input
              type="text"
              bind:value={exportInput}
              placeholder={$t('settings.exportConfirmWord')}
              class="flex-1 px-3 py-2 text-sm rounded-lg bg-gray-200 dark:bg-gray-900 border border-gray-400 dark:border-gray-600 text-gray-900 dark:text-gray-100 placeholder-gray-500 focus:outline-none focus:border-amber-500"
              onkeydown={(e) => {
                if (e.key === 'Enter' && exportInput === $t('settings.exportConfirmWord'))
                  handleExport();
              }}
            />
            <button
              onclick={handleExport}
              disabled={exporting || exportInput !== $t('settings.exportConfirmWord')}
              class="px-4 py-2 text-sm rounded-lg bg-amber-700 hover:bg-amber-600 disabled:opacity-50 disabled:cursor-not-allowed text-white transition-colors shrink-0"
            >
              {exporting ? $t('settings.exporting') : $t('settings.confirmExport')}
            </button>
            <button
              onclick={() => {
                showExportConfirm = false;
                exportInput = '';
                exportError = '';
              }}
              class="px-4 py-2 text-sm rounded-lg bg-gray-300 dark:bg-gray-700 hover:bg-gray-400 dark:hover:bg-gray-600 text-gray-900 dark:text-gray-100 transition-colors shrink-0"
            >
              {$t('common.cancel')}
            </button>
          </div>
          {#if exportError}
            <p class="text-xs text-red-400 mt-2">{exportError}</p>
          {/if}
        </div>
      {/if}
    </div>

    <!-- Factory Reset -->
    <div class="border border-red-800 rounded-lg p-4">
      <div class="flex items-start justify-between gap-4">
        <div>
          <p class="text-sm font-medium text-gray-900 dark:text-gray-100">
            {$t('settings.factoryReset')}
          </p>
          <p class="text-xs text-gray-600 dark:text-gray-400 mt-1">
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
            class="px-4 py-2 text-sm rounded-lg bg-red-700 hover:bg-red-600 text-white transition-colors shrink-0"
          >
            {$t('settings.factoryReset')}
          </button>
        {/if}
      </div>

      {#if showResetConfirm}
        <div class="mt-4 border-t border-red-800 pt-4">
          <p class="text-sm text-red-300 mb-3">
            {$t('settings.resetConfirmHint')}
          </p>
          <div class="flex gap-2">
            <input
              type="text"
              bind:value={resetInput}
              placeholder={$t('settings.resetConfirmWord')}
              class="flex-1 px-3 py-2 text-sm rounded-lg bg-gray-200 dark:bg-gray-900 border border-gray-400 dark:border-gray-600 text-gray-900 dark:text-gray-100 placeholder-gray-500 focus:outline-none focus:border-red-500"
              onkeydown={(e) => {
                if (e.key === 'Enter' && resetInput === $t('settings.resetConfirmWord'))
                  handleReset();
              }}
            />
            <button
              onclick={handleReset}
              disabled={resetting || resetInput !== $t('settings.resetConfirmWord')}
              class="px-4 py-2 text-sm rounded-lg bg-red-700 hover:bg-red-600 disabled:opacity-50 disabled:cursor-not-allowed text-white transition-colors shrink-0"
            >
              {resetting ? $t('settings.resetting') : $t('settings.confirmResetAction')}
            </button>
            <button
              onclick={() => {
                showResetConfirm = false;
                resetInput = '';
                resetError = '';
              }}
              class="px-4 py-2 text-sm rounded-lg bg-gray-300 dark:bg-gray-700 hover:bg-gray-400 dark:hover:bg-gray-600 text-gray-900 dark:text-gray-100 transition-colors shrink-0"
            >
              {$t('common.cancel')}
            </button>
          </div>
          {#if resetError}
            <p class="text-xs text-red-400 mt-2">{resetError}</p>
          {/if}
        </div>
      {/if}
    </div>
  </section>
</div>
