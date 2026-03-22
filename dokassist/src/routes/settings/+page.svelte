<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { listen, type UnlistenFn } from '@tauri-apps/api/event';
  import { getVersion } from '@tauri-apps/api/app';
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
    type LlmEngineStatus,
    type ModelChoice,
    type UpdateInfo,
    type EmbedStatus,
  } from '$lib/api';
  import { themePreference } from '$lib/stores/theme';
  import { language } from '$lib/stores/language';
  import { t } from '$lib/translations';

  let status = $state<LlmEngineStatus | null>(null);
  let recommended = $state<ModelChoice | null>(null);
  let downloadProgress = $state<number | null>(null);
  let phase = $state<'idle' | 'downloading' | 'loading' | 'done' | 'error'>('idle');
  let errorMsg = $state('');
  let unlisten: UnlistenFn | null = null;
  let appVersion = $state('');

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
  });

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
                <div class="whitespace-pre-wrap">{updateInfo.body}</div>
              </div>
            {/if}

            {#if installingUpdate}
              <div class="mb-3">
                <div class="flex justify-between text-xs text-gray-600 dark:text-gray-400 mb-1">
                  <span>{$t('settings.downloading')} {$t('settings.downloadingAndInstalling')}</span
                  >
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

  <section>
    <h2 class="text-lg font-semibold text-gray-900 dark:text-gray-200 mb-4">
      {$t('settings.llmModelSection')}
    </h2>

    <!-- Current status -->
    <div class="bg-gray-100 dark:bg-gray-800 rounded-lg p-4 mb-6 flex items-center gap-3">
      <div
        class="w-3 h-3 rounded-full shrink-0 {status?.is_loaded
          ? 'bg-green-500'
          : status?.is_downloaded
            ? 'bg-amber-400'
            : 'bg-red-500'}"
      ></div>
      <div>
        {#if status?.is_loaded}
          <p class="text-sm text-gray-900 dark:text-gray-100 font-medium">{status.model_name}</p>
          <p class="text-xs text-gray-600 dark:text-gray-400">
            {$t('settings.modelLoadedInfo').replace('{ram}', formatBytes(status.total_ram_bytes))}
          </p>
        {:else if status?.is_downloaded}
          <p class="text-sm text-gray-900 dark:text-gray-100 font-medium">
            {$t('settings.modelDownloadedNotLoaded')}
          </p>
          <p class="text-xs text-gray-600 dark:text-gray-400">
            {$t('settings.modelDownloadedInfo')
              .replace('{name}', status.downloaded_filename ?? '')
              .replace('{ram}', formatBytes(status.total_ram_bytes))}
          </p>
        {:else}
          <p class="text-sm text-gray-900 dark:text-gray-100 font-medium">
            {$t('settings.noModelDownloaded')}
          </p>
          {#if status}
            <p class="text-xs text-gray-600 dark:text-gray-400">
              {$t('settings.ramAvailable').replace('{ram}', formatBytes(status.total_ram_bytes))}
            </p>
          {/if}
        {/if}
      </div>
    </div>

    <!-- Recommended model card -->
    {#if recommended && !status?.is_loaded}
      <div
        class="bg-gray-100 dark:bg-gray-800 border border-gray-300 dark:border-gray-700 rounded-lg p-4 mb-4"
      >
        <div class="flex items-start justify-between gap-4 mb-1">
          <p class="text-sm font-medium text-gray-900 dark:text-gray-100">{recommended.name}</p>
          <span class="text-xs text-gray-600 dark:text-gray-400 shrink-0"
            >{formatBytes(recommended.size_bytes)}</span
          >
        </div>
        <p class="text-xs text-gray-600 dark:text-gray-400 mb-4">{recommended.reason}</p>

        {#if phase === 'downloading'}
          <div class="mb-3">
            <div class="flex justify-between text-xs text-gray-600 dark:text-gray-400 mb-1">
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
        {:else if phase === 'loading'}
          <p class="text-xs text-blue-400 mb-3">{$t('settings.loadingModelLabel')}</p>
        {/if}

        {#if phase === 'error'}
          <p class="text-xs text-red-400 mb-3">{errorMsg}</p>
        {/if}

        {#if status?.is_downloaded}
          <div class="flex gap-2">
            <button
              onclick={handleLoad}
              disabled={phase === 'downloading' || phase === 'loading'}
              class="px-4 py-2 text-sm rounded-lg bg-blue-600 hover:bg-blue-500 disabled:opacity-50 disabled:cursor-not-allowed text-white transition-colors"
            >
              {phase === 'loading' ? $t('common.loading') : $t('settings.loadModel')}
            </button>
            <button
              onclick={handleDownload}
              disabled={phase === 'downloading' || phase === 'loading'}
              class="px-4 py-2 text-sm rounded-lg bg-gray-300 dark:bg-gray-700 hover:bg-gray-400 dark:hover:bg-gray-600 disabled:opacity-50 disabled:cursor-not-allowed text-gray-900 dark:text-gray-100 transition-colors"
            >
              {phase === 'downloading'
                ? $t('settings.downloadingLabel')
                : $t('settings.redownload')}
            </button>
          </div>
        {:else}
          <div class="flex gap-2">
            <button
              onclick={handleDownload}
              disabled={phase === 'downloading' || phase === 'loading'}
              class="px-4 py-2 text-sm rounded-lg bg-blue-600 hover:bg-blue-500 disabled:opacity-50 disabled:cursor-not-allowed text-white transition-colors"
            >
              {phase === 'downloading'
                ? $t('settings.downloadingLabel')
                : $t('settings.downloadAndLoad')}
            </button>
            <button
              onclick={handleLoad}
              disabled={phase === 'downloading' || phase === 'loading'}
              class="px-4 py-2 text-sm rounded-lg bg-gray-300 dark:bg-gray-700 hover:bg-gray-400 dark:hover:bg-gray-600 disabled:opacity-50 disabled:cursor-not-allowed text-gray-900 dark:text-gray-100 transition-colors"
            >
              {phase === 'loading' ? $t('common.loading') : $t('settings.loadExisting')}
            </button>
          </div>
          <p class="text-xs text-gray-600 dark:text-gray-400 mt-2">
            {$t('settings.loadExistingHint')}
          </p>
        {/if}
      </div>
    {/if}

    {#if phase === 'done' && status?.is_loaded}
      <p class="text-sm text-green-400">
        {$t('settings.modelReadyMsg')}
      </p>
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
