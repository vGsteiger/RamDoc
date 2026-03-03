<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { listen, type UnlistenFn } from '@tauri-apps/api/event';
  import { goto } from '$app/navigation';
  import {
    getEngineStatus,
    getRecommendedModel,
    downloadModel,
    loadModel,
    resetApp,
    parseError,
    type LlmEngineStatus,
    type ModelChoice,
  } from '$lib/api';

  let status = $state<LlmEngineStatus | null>(null);
  let recommended = $state<ModelChoice | null>(null);
  let downloadProgress = $state<number | null>(null);
  let phase = $state<'idle' | 'downloading' | 'loading' | 'done' | 'error'>('idle');
  let errorMsg = $state('');
  let unlisten: UnlistenFn | null = null;

  onMount(async () => {
    [status, recommended] = await Promise.all([getEngineStatus(), getRecommendedModel()]);
    if (status.is_loaded) phase = 'done';
  });

  onDestroy(() => {
    unlisten?.();
  });

  function formatBytes(bytes: number): string {
    const gb = bytes / (1024 ** 3);
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
</script>

<div class="p-8 max-w-xl">
  <h1 class="text-2xl font-bold text-gray-100 mb-6">Settings</h1>

  <section>
    <h2 class="text-lg font-semibold text-gray-200 mb-4">LLM Model</h2>

    <!-- Current status -->
    <div class="bg-gray-800 rounded-lg p-4 mb-6 flex items-center gap-3">
      <div class="w-3 h-3 rounded-full shrink-0 {status?.is_loaded ? 'bg-green-500' : status?.is_downloaded ? 'bg-amber-400' : 'bg-red-500'}"></div>
      <div>
        {#if status?.is_loaded}
          <p class="text-sm text-gray-100 font-medium">{status.model_name}</p>
          <p class="text-xs text-gray-400">Loaded · {formatBytes(status.total_ram_bytes)} system RAM</p>
        {:else if status?.is_downloaded}
          <p class="text-sm text-gray-100 font-medium">Model downloaded, not loaded</p>
          <p class="text-xs text-gray-400">{status.downloaded_filename} · {formatBytes(status.total_ram_bytes)} RAM available</p>
        {:else}
          <p class="text-sm text-gray-100 font-medium">No model downloaded</p>
          {#if status}
            <p class="text-xs text-gray-400">{formatBytes(status.total_ram_bytes)} system RAM available</p>
          {/if}
        {/if}
      </div>
    </div>

    <!-- Recommended model card -->
    {#if recommended && !status?.is_loaded}
      <div class="bg-gray-800 border border-gray-700 rounded-lg p-4 mb-4">
        <div class="flex items-start justify-between gap-4 mb-1">
          <p class="text-sm font-medium text-gray-100">{recommended.name}</p>
          <span class="text-xs text-gray-400 shrink-0">{formatBytes(recommended.size_bytes)}</span>
        </div>
        <p class="text-xs text-gray-400 mb-4">{recommended.reason}</p>

        {#if phase === 'downloading'}
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
        {:else if phase === 'loading'}
          <p class="text-xs text-blue-400 mb-3">Loading model into memory…</p>
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
              {phase === 'loading' ? 'Loading…' : 'Load model'}
            </button>
            <button
              onclick={handleDownload}
              disabled={phase === 'downloading' || phase === 'loading'}
              class="px-4 py-2 text-sm rounded-lg bg-gray-700 hover:bg-gray-600 disabled:opacity-50 disabled:cursor-not-allowed text-gray-100 transition-colors"
            >
              {phase === 'downloading' ? 'Downloading…' : 'Re-download'}
            </button>
          </div>
        {:else}
          <div class="flex gap-2">
            <button
              onclick={handleDownload}
              disabled={phase === 'downloading' || phase === 'loading'}
              class="px-4 py-2 text-sm rounded-lg bg-blue-600 hover:bg-blue-500 disabled:opacity-50 disabled:cursor-not-allowed text-white transition-colors"
            >
              {phase === 'downloading' ? 'Downloading…' : 'Download & Load'}
            </button>
            <button
              onclick={handleLoad}
              disabled={phase === 'downloading' || phase === 'loading'}
              class="px-4 py-2 text-sm rounded-lg bg-gray-700 hover:bg-gray-600 disabled:opacity-50 disabled:cursor-not-allowed text-gray-100 transition-colors"
            >
              {phase === 'loading' ? 'Loading…' : 'Load existing'}
            </button>
          </div>
          <p class="text-xs text-gray-500 mt-2">"Load existing" if the model file is already downloaded.</p>
        {/if}
      </div>
    {/if}

    {#if phase === 'done' && status?.is_loaded}
      <p class="text-sm text-green-400">Model ready. Reports and metadata extraction are available.</p>
    {/if}
  </section>

  <section class="mt-10">
    <h2 class="text-lg font-semibold text-red-400 mb-4">Danger Zone</h2>
    <div class="border border-red-800 rounded-lg p-4">
      <div class="flex items-start justify-between gap-4">
        <div>
          <p class="text-sm font-medium text-gray-100">Factory Reset</p>
          <p class="text-xs text-gray-400 mt-1">Deletes all patient data, encryption keys, and model files. This cannot be undone.</p>
        </div>
        {#if !showResetConfirm}
          <button
            onclick={() => { showResetConfirm = true; resetInput = ''; resetError = ''; }}
            class="px-4 py-2 text-sm rounded-lg bg-red-700 hover:bg-red-600 text-white transition-colors shrink-0"
          >
            Factory Reset
          </button>
        {/if}
      </div>

      {#if showResetConfirm}
        <div class="mt-4 border-t border-red-800 pt-4">
          <p class="text-sm text-red-300 mb-3">Type <strong>RESET</strong> to confirm, or click the button. This action is irreversible.</p>
          <div class="flex gap-2">
            <input
              type="text"
              bind:value={resetInput}
              placeholder="RESET"
              class="flex-1 px-3 py-2 text-sm rounded-lg bg-gray-900 border border-gray-600 text-gray-100 placeholder-gray-500 focus:outline-none focus:border-red-500"
              onkeydown={(e) => { if (e.key === 'Enter' && resetInput === 'RESET') handleReset(); }}
            />
            <button
              onclick={handleReset}
              disabled={resetting}
              class="px-4 py-2 text-sm rounded-lg bg-red-700 hover:bg-red-600 disabled:opacity-50 disabled:cursor-not-allowed text-white transition-colors shrink-0"
            >
              {resetting ? 'Resetting…' : 'Confirm Reset'}
            </button>
            <button
              onclick={() => { showResetConfirm = false; resetInput = ''; resetError = ''; }}
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
