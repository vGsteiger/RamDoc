<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { listen, type UnlistenFn } from '@tauri-apps/api/event';
  import {
    checkForUpdates,
    installUpdate,
    parseError,
    type UpdateInfo,
  } from '$lib/api';

  let updateInfo = $state<UpdateInfo | null>(null);
  let checking = $state(false);
  let installing = $state(false);
  let downloadProgress = $state<number>(0);
  let errorMsg = $state('');
  let showNotification = $state(false);
  let unlisten: UnlistenFn | null = null;

  export let autoCheck = true;

  onMount(async () => {
    if (autoCheck) {
      await handleCheckForUpdates();
    }
  });

  onDestroy(() => {
    unlisten?.();
  });

  async function handleCheckForUpdates() {
    checking = true;
    errorMsg = '';
    try {
      updateInfo = await checkForUpdates();
      if (updateInfo.update_available) {
        showNotification = true;
      }
    } catch (e) {
      errorMsg = parseError(e).message;
    } finally {
      checking = false;
    }
  }

  async function handleInstallUpdate() {
    if (!updateInfo?.update_available) return;

    installing = true;
    errorMsg = '';
    downloadProgress = 0;

    // Listen for download progress events
    unlisten = await listen<number>('updater-download-progress', (e) => {
      downloadProgress = Math.round(e.payload * 100);
    });

    const completeUnsub = await listen('updater-download-complete', () => {
      completeUnsub();
    });

    try {
      await installUpdate();
      // After successful install, the app will restart automatically
    } catch (e) {
      unlisten?.();
      unlisten = null;
      installing = false;
      errorMsg = parseError(e).message;
    }
  }

  function dismiss() {
    showNotification = false;
  }
</script>

{#if showNotification && updateInfo?.update_available}
  <div class="fixed top-4 right-4 z-50 bg-gray-800 border border-gray-700 rounded-lg shadow-lg p-4 max-w-md">
    <div class="flex items-start justify-between gap-4">
      <div class="flex-1">
        <h3 class="text-sm font-semibold text-gray-100 mb-1">Update Available</h3>
        <p class="text-xs text-gray-400 mb-2">
          Version {updateInfo.latest_version} is now available. You are currently on version {updateInfo.current_version}.
        </p>
        {#if updateInfo.body}
          <div class="text-xs text-gray-400 mb-3 max-h-24 overflow-y-auto">
            <p class="font-medium mb-1">What's new:</p>
            <div class="whitespace-pre-wrap">{updateInfo.body}</div>
          </div>
        {/if}

        {#if installing}
          <div class="mb-3">
            <div class="flex justify-between text-xs text-gray-400 mb-1">
              <span>Downloading update...</span>
              <span>{downloadProgress}%</span>
            </div>
            <div class="w-full bg-gray-700 rounded-full h-2">
              <div
                class="bg-blue-500 h-2 rounded-full transition-all"
                style="width: {downloadProgress}%"
              ></div>
            </div>
          </div>
        {/if}

        {#if errorMsg}
          <p class="text-xs text-red-400 mb-3">{errorMsg}</p>
        {/if}

        <div class="flex gap-2">
          <button
            onclick={handleInstallUpdate}
            disabled={installing}
            class="px-3 py-1.5 text-xs rounded-lg bg-blue-600 hover:bg-blue-500 disabled:opacity-50 disabled:cursor-not-allowed text-white transition-colors"
          >
            {installing ? 'Installing...' : 'Install Update'}
          </button>
          <button
            onclick={dismiss}
            disabled={installing}
            class="px-3 py-1.5 text-xs rounded-lg bg-gray-700 hover:bg-gray-600 disabled:opacity-50 disabled:cursor-not-allowed text-gray-100 transition-colors"
          >
            Later
          </button>
        </div>
      </div>
      <button
        onclick={dismiss}
        disabled={installing}
        class="text-gray-400 hover:text-gray-200 transition-colors disabled:opacity-50"
      >
        <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
        </svg>
      </button>
    </div>
  </div>
{/if}
