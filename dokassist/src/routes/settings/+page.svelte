<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { listen, type UnlistenFn } from "@tauri-apps/api/event";
  import { getVersion } from "@tauri-apps/api/app";
  import { goto } from "$app/navigation";
  import {
    getEngineStatus,
    getRecommendedModel,
    downloadModel,
    loadModel,
    resetApp,
    parseError,
    checkForUpdates,
    installUpdate,
    getAppVersion,
    exportAllPatientData,
    type LlmEngineStatus,
    type ModelChoice,
    type UpdateInfo,
  } from "$lib/api";

  let status = $state<LlmEngineStatus | null>(null);
  let recommended = $state<ModelChoice | null>(null);
  let downloadProgress = $state<number | null>(null);
  let phase = $state<"idle" | "downloading" | "loading" | "done" | "error">(
    "idle",
  );
  let errorMsg = $state("");
  let unlisten: UnlistenFn | null = null;
  let appVersion = $state("");

  // Update state
  let updateInfo = $state<UpdateInfo | null>(null);
  let checkingUpdate = $state(false);
  let installingUpdate = $state(false);
  let updateProgress = $state<number>(0);
  let updateError = $state("");
  let updateUnlisten: UnlistenFn | null = null;

  onMount(async () => {
    [status, recommended, appVersion] = await Promise.all([
      getEngineStatus(),
      getRecommendedModel(),
      getVersion().catch(() => "Unknown"),
    ]);
    if (status.is_loaded) phase = "done";
  });

  onDestroy(() => {
    unlisten?.();
    updateUnlisten?.();
  });

  async function handleCheckForUpdates() {
    checkingUpdate = true;
    updateError = "";
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
    updateError = "";
    updateProgress = 0;

    // Listen for download progress events
    updateUnlisten = await listen<number>("updater-download-progress", (e) => {
      updateProgress = Math.round(e.payload * 100);
    });

    const completeUnsub = await listen("updater-download-complete", () => {
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
    phase = "downloading";
    downloadProgress = 0;
    errorMsg = "";

    unlisten = await listen<number>("model-download-progress", (e) => {
      downloadProgress = Math.round(e.payload * 100);
    });

    const doneUnsub = await listen("model-download-done", () => {
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
      phase = "error";
      errorMsg = parseError(e).message;
    }
  }

  async function handleLoad() {
    if (!recommended) return;
    phase = "loading";
    errorMsg = "";
    try {
      await loadModel(recommended.filename);
      status = await getEngineStatus();
      phase = "done";
    } catch (e) {
      phase = "error";
      errorMsg = parseError(e).message;
    }
  }

  let showResetConfirm = $state(false);
  let resetInput = $state("");
  let resetting = $state(false);
  let resetError = $state("");

  // Export state
  let showExportConfirm = $state(false);
  let exportInput = $state("");
  let exporting = $state(false);
  let exportError = $state("");

  async function handleReset() {
    resetting = true;
    resetError = "";
    try {
      await resetApp();
      goto("/");
    } catch (e) {
      resetError = parseError(e).message;
      resetting = false;
    }
  }

  async function handleExport() {
    exporting = true;
    exportError = "";
    try {
      const zipData = await exportAllPatientData();

      // Convert number array to Uint8Array
      const blob = new Blob([new Uint8Array(zipData)], {
        type: "application/zip"
      });

      // Create download link
      const url = URL.createObjectURL(blob);
      const a = document.createElement("a");
      a.href = url;
      a.download = `DokAssist_Export_${new Date().toISOString().split('T')[0]}.zip`;
      document.body.appendChild(a);
      a.click();
      document.body.removeChild(a);
      URL.revokeObjectURL(url);

      // Reset the form
      showExportConfirm = false;
      exportInput = "";
    } catch (e) {
      exportError = parseError(e).message;
    } finally {
      exporting = false;
    }
  }
</script>

<div class="p-8 max-w-xl">
  <h1 class="text-2xl font-bold text-gray-100 mb-6">Settings</h1>

  <section class="mb-10">
    <h2 class="text-lg font-semibold text-gray-200 mb-4">
      Application Updates
    </h2>

    <div class="bg-gray-800 rounded-lg p-4 mb-4">
      <div class="flex items-center justify-between mb-3">
        <div>
          <p class="text-sm font-medium text-gray-100">Current Version</p>
          <p class="text-xs text-gray-400 mt-1">{appVersion || "Loading..."}</p>
        </div>
        <button
          onclick={handleCheckForUpdates}
          disabled={checkingUpdate || installingUpdate}
          class="px-4 py-2 text-sm rounded-lg bg-blue-600 hover:bg-blue-500 disabled:opacity-50 disabled:cursor-not-allowed text-white transition-colors"
        >
          {checkingUpdate ? "Checking..." : "Check for Updates"}
        </button>
      </div>

      {#if updateInfo}
        {#if updateInfo.update_available}
          <div class="border-t border-gray-700 pt-3 mt-3">
            <div class="flex items-start justify-between gap-4 mb-2">
              <div>
                <p class="text-sm font-medium text-green-400">
                  Update Available
                </p>
                <p class="text-xs text-gray-400 mt-1">
                  Version {updateInfo.latest_version} is now available
                </p>
              </div>
            </div>

            {#if updateInfo.body}
              <div
                class="text-xs text-gray-400 mb-3 max-h-32 overflow-y-auto bg-gray-900 rounded p-2"
              >
                <p class="font-medium mb-1">Release Notes:</p>
                <div class="whitespace-pre-wrap">{updateInfo.body}</div>
              </div>
            {/if}

            {#if installingUpdate}
              <div class="mb-3">
                <div class="flex justify-between text-xs text-gray-400 mb-1">
                  <span>Downloading and installing update...</span>
                  <span>{updateProgress}%</span>
                </div>
                <div class="w-full bg-gray-700 rounded-full h-2">
                  <div
                    class="bg-blue-500 h-2 rounded-full transition-all"
                    style="width: {updateProgress}%"
                  ></div>
                </div>
                <p class="text-xs text-gray-400 mt-2">
                  The app will restart automatically after installation.
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
                Install Update
              </button>
            {/if}
          </div>
        {:else}
          <div class="border-t border-gray-700 pt-3 mt-3">
            <p class="text-sm text-green-400">You're up to date!</p>
          </div>
        {/if}
      {/if}

      {#if updateError && !updateInfo}
        <div class="border-t border-gray-700 pt-3 mt-3">
          <p class="text-xs text-red-400">{updateError}</p>
        </div>
      {/if}
    </div>
  </section>

  <section>
    <h2 class="text-lg font-semibold text-gray-200 mb-4">LLM Model</h2>

    <!-- Current status -->
    <div class="bg-gray-800 rounded-lg p-4 mb-6 flex items-center gap-3">
      <div
        class="w-3 h-3 rounded-full shrink-0 {status?.is_loaded
          ? 'bg-green-500'
          : status?.is_downloaded
            ? 'bg-amber-400'
            : 'bg-red-500'}"
      ></div>
      <div>
        {#if status?.is_loaded}
          <p class="text-sm text-gray-100 font-medium">{status.model_name}</p>
          <p class="text-xs text-gray-400">
            Loaded · {formatBytes(status.total_ram_bytes)} system RAM
          </p>
        {:else if status?.is_downloaded}
          <p class="text-sm text-gray-100 font-medium">
            Model downloaded, not loaded
          </p>
          <p class="text-xs text-gray-400">
            {status.downloaded_filename} · {formatBytes(status.total_ram_bytes)}
            RAM available
          </p>
        {:else}
          <p class="text-sm text-gray-100 font-medium">No model downloaded</p>
          {#if status}
            <p class="text-xs text-gray-400">
              {formatBytes(status.total_ram_bytes)} system RAM available
            </p>
          {/if}
        {/if}
      </div>
    </div>

    <!-- Recommended model card -->
    {#if recommended && !status?.is_loaded}
      <div class="bg-gray-800 border border-gray-700 rounded-lg p-4 mb-4">
        <div class="flex items-start justify-between gap-4 mb-1">
          <p class="text-sm font-medium text-gray-100">{recommended.name}</p>
          <span class="text-xs text-gray-400 shrink-0"
            >{formatBytes(recommended.size_bytes)}</span
          >
        </div>
        <p class="text-xs text-gray-400 mb-4">{recommended.reason}</p>

        {#if phase === "downloading"}
          <div class="mb-3">
            <div class="flex justify-between text-xs text-gray-400 mb-1">
              <span>Downloading…</span>
              <span>{downloadProgress ?? 0}%</span>
            </div>
            <div class="w-full bg-gray-700 rounded-full h-2">
              <div
                class="bg-blue-500 h-2 rounded-full transition-all"
                style="width: {downloadProgress ?? 0}%"
              ></div>
            </div>
          </div>
        {:else if phase === "loading"}
          <p class="text-xs text-blue-400 mb-3">Loading model into memory…</p>
        {/if}

        {#if phase === "error"}
          <p class="text-xs text-red-400 mb-3">{errorMsg}</p>
        {/if}

        {#if status?.is_downloaded}
          <div class="flex gap-2">
            <button
              onclick={handleLoad}
              disabled={phase === "downloading" || phase === "loading"}
              class="px-4 py-2 text-sm rounded-lg bg-blue-600 hover:bg-blue-500 disabled:opacity-50 disabled:cursor-not-allowed text-white transition-colors"
            >
              {phase === "loading" ? "Loading…" : "Load model"}
            </button>
            <button
              onclick={handleDownload}
              disabled={phase === "downloading" || phase === "loading"}
              class="px-4 py-2 text-sm rounded-lg bg-gray-700 hover:bg-gray-600 disabled:opacity-50 disabled:cursor-not-allowed text-gray-100 transition-colors"
            >
              {phase === "downloading" ? "Downloading…" : "Re-download"}
            </button>
          </div>
        {:else}
          <div class="flex gap-2">
            <button
              onclick={handleDownload}
              disabled={phase === "downloading" || phase === "loading"}
              class="px-4 py-2 text-sm rounded-lg bg-blue-600 hover:bg-blue-500 disabled:opacity-50 disabled:cursor-not-allowed text-white transition-colors"
            >
              {phase === "downloading" ? "Downloading…" : "Download & Load"}
            </button>
            <button
              onclick={handleLoad}
              disabled={phase === "downloading" || phase === "loading"}
              class="px-4 py-2 text-sm rounded-lg bg-gray-700 hover:bg-gray-600 disabled:opacity-50 disabled:cursor-not-allowed text-gray-100 transition-colors"
            >
              {phase === "loading" ? "Loading…" : "Load existing"}
            </button>
          </div>
          <p class="text-xs text-gray-500 mt-2">
            "Load existing" if the model file is already downloaded.
          </p>
        {/if}
      </div>
    {/if}

    {#if phase === "done" && status?.is_loaded}
      <p class="text-sm text-green-400">
        Model ready. Reports and metadata extraction are available.
      </p>
    {/if}
  </section>

  <section class="mt-10">
    <h2 class="text-lg font-semibold text-gray-200 mb-2">About</h2>
    <p class="text-sm text-gray-400">
      App Version: <span class="text-gray-100">{appVersion || "…"}</span>
    </p>
  </section>

  <section class="mt-10">
    <h2 class="text-lg font-semibold text-red-400 mb-4">Danger Zone</h2>

    <!-- Emergency Export -->
    <div class="border border-amber-600 rounded-lg p-4 mb-4">
      <div class="flex items-start justify-between gap-4">
        <div>
          <p class="text-sm font-medium text-gray-100">Emergency Export</p>
          <p class="text-xs text-gray-400 mt-1">
            Export all patient data to a ZIP file. Use this if you need to
            migrate to another system or create a complete backup.
          </p>
        </div>
        {#if !showExportConfirm}
          <button
            onclick={() => {
              showExportConfirm = true;
              exportInput = "";
              exportError = "";
            }}
            class="px-4 py-2 text-sm rounded-lg bg-amber-700 hover:bg-amber-600 text-white transition-colors shrink-0"
          >
            Export All Data
          </button>
        {/if}
      </div>

      {#if showExportConfirm}
        <div class="mt-4 border-t border-amber-700 pt-4">
          <p class="text-sm text-amber-300 mb-3">
            Type <strong>EXPORT</strong> to confirm. This will create a ZIP file
            with all patient data including decrypted files.
          </p>
          <div class="flex gap-2">
            <input
              type="text"
              bind:value={exportInput}
              placeholder="EXPORT"
              class="flex-1 px-3 py-2 text-sm rounded-lg bg-gray-900 border border-gray-600 text-gray-100 placeholder-gray-500 focus:outline-none focus:border-amber-500"
              onkeydown={(e) => {
                if (e.key === "Enter" && exportInput === "EXPORT")
                  handleExport();
              }}
            />
            <button
              onclick={handleExport}
              disabled={exporting || exportInput !== "EXPORT"}
              class="px-4 py-2 text-sm rounded-lg bg-amber-700 hover:bg-amber-600 disabled:opacity-50 disabled:cursor-not-allowed text-white transition-colors shrink-0"
            >
              {exporting ? "Exporting…" : "Confirm Export"}
            </button>
            <button
              onclick={() => {
                showExportConfirm = false;
                exportInput = "";
                exportError = "";
              }}
              class="px-4 py-2 text-sm rounded-lg bg-gray-700 hover:bg-gray-600 text-gray-100 transition-colors shrink-0"
            >
              Cancel
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
          <p class="text-sm font-medium text-gray-100">Factory Reset</p>
          <p class="text-xs text-gray-400 mt-1">
            Deletes all patient data, encryption keys, and model files. This
            cannot be undone.
          </p>
        </div>
        {#if !showResetConfirm}
          <button
            onclick={() => {
              showResetConfirm = true;
              resetInput = "";
              resetError = "";
            }}
            class="px-4 py-2 text-sm rounded-lg bg-red-700 hover:bg-red-600 text-white transition-colors shrink-0"
          >
            Factory Reset
          </button>
        {/if}
      </div>

      {#if showResetConfirm}
        <div class="mt-4 border-t border-red-800 pt-4">
          <p class="text-sm text-red-300 mb-3">
            Type <strong>RESET</strong> to confirm, or click the button. This action
            is irreversible.
          </p>
          <div class="flex gap-2">
            <input
              type="text"
              bind:value={resetInput}
              placeholder="RESET"
              class="flex-1 px-3 py-2 text-sm rounded-lg bg-gray-900 border border-gray-600 text-gray-100 placeholder-gray-500 focus:outline-none focus:border-red-500"
              onkeydown={(e) => {
                if (e.key === "Enter" && resetInput === "RESET") handleReset();
              }}
            />
            <button
              onclick={handleReset}
              disabled={resetting}
              class="px-4 py-2 text-sm rounded-lg bg-red-700 hover:bg-red-600 disabled:opacity-50 disabled:cursor-not-allowed text-white transition-colors shrink-0"
            >
              {resetting ? "Resetting…" : "Confirm Reset"}
            </button>
            <button
              onclick={() => {
                showResetConfirm = false;
                resetInput = "";
                resetError = "";
              }}
              class="px-4 py-2 text-sm rounded-lg bg-gray-700 hover:bg-gray-600 text-gray-100 transition-colors shrink-0"
            >
              Cancel
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
