<script lang="ts">
  import { goto } from '$app/navigation';
  import { onMount, onDestroy } from 'svelte';
  import { listen, type UnlistenFn } from '@tauri-apps/api/event';
  import {
    getRecommendedModel,
    downloadAndRegisterModel,
    parseError,
    type ModelChoice,
  } from '$lib/api';
  import { ChevronRight, ChevronLeft, Download, Check, Zap, Brain, Gauge } from 'lucide-svelte';

  let recommended = $state<ModelChoice | null>(null);
  let error = $state<string | null>(null);
  let isLoading = $state(true);
  let isDownloading = $state(false);
  let downloadProgress = $state<number>(0);
  let isComplete = $state(false);

  let unlisten: UnlistenFn | null = null;
  let doneUnsubscribe: UnlistenFn | null = null;

  onMount(async () => {
    try {
      recommended = await getRecommendedModel();
    } catch (err) {
      error = parseError(err).message;
    } finally {
      isLoading = false;
    }
  });

  onDestroy(() => {
    unlisten?.();
    doneUnsubscribe?.();
  });

  async function handleDownload() {
    if (!recommended) return;

    isDownloading = true;
    downloadProgress = 0;
    error = null;

    try {
      unlisten = await listen<number>('model-download-progress', (e) => {
        downloadProgress = Math.round(e.payload * 100);
      });

      doneUnsubscribe = await listen('model-download-done', () => {
        isComplete = true;
      });

      await downloadAndRegisterModel(recommended);
    } catch (err) {
      error = parseError(err).message;
    } finally {
      doneUnsubscribe?.();
      unlisten?.();
      doneUnsubscribe = null;
      unlisten = null;
      isDownloading = false;
    }
  }

  function handleContinue() {
    goto('/onboarding/step3');
  }

  function handleBack() {
    goto('/onboarding/step1');
  }

  function formatBytes(bytes: number): string {
    const gb = bytes / 1024 ** 3;
    return `${gb.toFixed(1)} GB`;
  }
</script>

<div class="min-h-screen bg-gray-950 flex items-center justify-center p-8">
  <div class="max-w-3xl w-full">
    <div class="mb-8 text-center">
      <h1 class="text-3xl font-bold text-gray-100 mb-2">Configure AI Model</h1>
      <p class="text-gray-400">
        Download a language model for AI-powered features like session summaries and report
        generation.
      </p>
      <div class="flex items-center justify-center gap-2 mt-4">
        <div class="h-2 w-16 bg-blue-600 rounded-full"></div>
        <div class="h-2 w-16 bg-blue-600 rounded-full"></div>
        <div class="h-2 w-16 bg-gray-700 rounded-full"></div>
        <div class="h-2 w-16 bg-gray-700 rounded-full"></div>
      </div>
    </div>

    {#if error}
      <div class="bg-red-900/20 border border-red-500 rounded-lg p-4 mb-6">
        <p class="text-red-500 text-sm">{error}</p>
      </div>
    {/if}

    {#if isLoading}
      <div class="text-center py-12">
        <div class="animate-spin rounded-full h-12 w-12 border-b-2 border-blue-500 mx-auto"></div>
        <p class="mt-4 text-gray-400">Loading model recommendations...</p>
      </div>
    {:else if recommended}
      <div class="bg-gray-900 border border-gray-800 rounded-lg p-8 space-y-6">
        <div class="text-center">
          <div class="inline-block p-4 bg-blue-900/20 rounded-full mb-4">
            <Brain size={48} class="text-blue-500" />
          </div>
          <h2 class="text-2xl font-bold text-gray-100 mb-2">{recommended.name}</h2>
          <p class="text-gray-400">Recommended for your system</p>
        </div>

        <div class="grid grid-cols-1 md:grid-cols-3 gap-4">
          <div class="bg-gray-800 rounded-lg p-4 text-center">
            <Gauge size={24} class="text-gray-400 mx-auto mb-2" />
            <p class="text-gray-400 text-sm mb-1">Size</p>
            <p class="text-gray-100 font-semibold">{formatBytes(recommended.size_bytes)}</p>
          </div>

          <div class="bg-gray-800 rounded-lg p-4 text-center">
            <Zap size={24} class="text-gray-400 mx-auto mb-2" />
            <p class="text-gray-400 text-sm mb-1">Speed</p>
            <p class="text-gray-100 font-semibold">{recommended.tier === '30B' ? 'Good' : 'Fast'}</p>
          </div>

          <div class="bg-gray-800 rounded-lg p-4 text-center">
            <Brain size={24} class="text-gray-400 mx-auto mb-2" />
            <p class="text-gray-400 text-sm mb-1">Quality</p>
            <p class="text-gray-100 font-semibold">{recommended.tier === '30B' ? 'Excellent' : 'Very Good'}</p>
          </div>
        </div>

        <div class="bg-blue-900/20 border border-blue-800 rounded-lg p-4">
          <p class="text-blue-400 text-sm">
            <strong>About this model:</strong> This model provides a balance between quality and
            performance. It will run locally on your machine, ensuring your patient data remains
            private and secure.
          </p>
        </div>

        {#if isDownloading}
          <div class="space-y-4">
            <div class="flex items-center justify-between text-sm text-gray-400">
              <span>Downloading model...</span>
              <span>{downloadProgress}%</span>
            </div>
            <div class="w-full bg-gray-800 rounded-full h-3 overflow-hidden">
              <div
                class="bg-blue-600 h-full transition-all duration-300 ease-out"
                style="width: {downloadProgress}%"
              ></div>
            </div>
            <p class="text-center text-gray-500 text-sm">
              This may take a few minutes depending on your internet connection.
            </p>
          </div>
        {:else if isComplete}
          <div class="bg-green-900/20 border border-green-800 rounded-lg p-4 flex items-center gap-3">
            <div class="flex-shrink-0 w-10 h-10 bg-green-600 rounded-full flex items-center justify-center">
              <Check size={24} class="text-white" />
            </div>
            <div>
              <p class="text-green-400 font-semibold">Download Complete!</p>
              <p class="text-gray-400 text-sm">The model is ready to use.</p>
            </div>
          </div>
        {/if}
      </div>

      <div class="flex justify-between items-center mt-8">
        <button
          onclick={handleBack}
          disabled={isDownloading}
          class="px-6 py-3 bg-gray-700 hover:bg-gray-600 disabled:bg-gray-800 disabled:cursor-not-allowed text-white font-medium rounded-lg transition-colors flex items-center gap-2"
        >
          <ChevronLeft size={20} />
          Back
        </button>

        <div class="flex gap-3">
          {#if !isComplete && !isDownloading}
            <button
              onclick={handleContinue}
              class="px-6 py-3 text-gray-400 hover:text-gray-300 font-medium transition-colors"
            >
              Skip for now
            </button>

            <button
              onclick={handleDownload}
              class="px-6 py-3 bg-blue-600 hover:bg-blue-700 text-white font-medium rounded-lg transition-colors flex items-center gap-2"
            >
              <Download size={20} />
              Download Model
            </button>
          {:else if isComplete}
            <button
              onclick={handleContinue}
              class="px-6 py-3 bg-blue-600 hover:bg-blue-700 text-white font-medium rounded-lg transition-colors flex items-center gap-2"
            >
              Continue
              <ChevronRight size={20} />
            </button>
          {/if}
        </div>
      </div>
    {/if}
  </div>
</div>
