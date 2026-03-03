<script lang="ts">
  import { goto } from '$app/navigation';
  import { unlockApp, resetApp, parseError } from '$lib/api';
  import { authStatus } from '$lib/stores/auth';

  let isUnlocking = $state(false);
  let isResetting = $state(false);
  let resetConfirm = $state(false);
  let error = $state<string | null>(null);

  function friendlyError(err: unknown): string {
    const { code, message } = parseError(err);
    switch (code) {
      case 'KEYCHAIN_ERROR':
        return `Keychain error — ${message}. Check System Settings → Privacy & Security.`;
      case 'DATABASE_ERROR':
        return `Database error — ${message}. The database may be corrupt; consider a factory reset.`;
      case 'FILESYSTEM_ERROR':
        return `Filesystem error — ${message}.`;
      case 'AUTH_REQUIRED':
        return 'Authentication required. Please unlock the app.';
      default:
        return message || 'Failed to unlock';
    }
  }

  async function handleUnlock() {
    if (isUnlocking) return;
    isUnlocking = true;
    error = null;

    try {
      const unlocked = await unlockApp();
      if (!unlocked) {
        error = 'Failed to unlock';
        return;
      }
      authStatus.set('unlocked');
      goto('/patients');
    } catch (err) {
      error = friendlyError(err);
    } finally {
      isUnlocking = false;
    }
  }

  async function handleReset() {
    if (isResetting) return;
    isResetting = true;
    error = null;

    try {
      await resetApp();
      authStatus.set('first_run');
      goto('/setup');
    } catch (err) {
      error = friendlyError(err);
    } finally {
      isResetting = false;
      resetConfirm = false;
    }
  }
</script>

<div class="min-h-screen bg-gray-950 flex items-center justify-center p-8">
  <div class="max-w-md w-full text-center space-y-8">
    <div>
      <h1 class="text-3xl font-bold text-gray-100 mb-2">Welcome Back</h1>
      <p class="text-gray-400">Unlock DokAssist to continue</p>
    </div>

    {#if error}
      <div class="bg-red-900/20 border border-red-500 rounded-lg p-4 text-left">
        <p class="text-red-400 text-sm font-mono">{error}</p>
      </div>
    {/if}

    <div class="space-y-4">
      <button
        onclick={handleUnlock}
        disabled={isUnlocking}
        class="w-full px-6 py-4 bg-blue-600 hover:bg-blue-700 disabled:bg-gray-700 disabled:cursor-not-allowed text-white font-medium rounded-lg transition-colors flex items-center justify-center gap-3"
      >
        {#if isUnlocking}
          <div class="animate-spin rounded-full h-5 w-5 border-b-2 border-white"></div>
          <span>Unlocking...</span>
        {:else}
          <span class="text-2xl">🔓</span>
          <span>Unlock with Touch ID</span>
        {/if}
      </button>

      <a
        href="/recover"
        class="block text-sm text-blue-400 hover:text-blue-300 transition-colors"
      >
        I've lost access — use recovery phrase
      </a>
    </div>

    <!-- Factory Reset -->
    <div class="border-t border-gray-800 pt-6">
      {#if !resetConfirm}
        <button
          onclick={() => { resetConfirm = true; error = null; }}
          class="text-xs text-gray-600 hover:text-red-500 transition-colors"
        >
          Factory Reset…
        </button>
      {:else}
        <div class="bg-red-950/40 border border-red-800 rounded-lg p-4 space-y-3 text-left">
          <p class="text-red-400 text-sm font-semibold">⚠ Destructive — this cannot be undone</p>
          <p class="text-gray-400 text-xs leading-relaxed">
            All patient data, the encrypted vault, database, and stored keys will be
            <strong class="text-gray-200">permanently deleted</strong>.
            The app will restart from the initial setup screen.
          </p>
          <div class="flex gap-2 pt-1">
            <button
              onclick={handleReset}
              disabled={isResetting}
              class="flex-1 px-3 py-2 bg-red-700 hover:bg-red-600 disabled:bg-gray-700 disabled:cursor-not-allowed text-white text-xs font-medium rounded transition-colors"
            >
              {isResetting ? 'Wiping…' : 'Yes, wipe everything'}
            </button>
            <button
              onclick={() => resetConfirm = false}
              disabled={isResetting}
              class="flex-1 px-3 py-2 bg-gray-700 hover:bg-gray-600 disabled:cursor-not-allowed text-gray-300 text-xs font-medium rounded transition-colors"
            >
              Cancel
            </button>
          </div>
        </div>
      {/if}
    </div>
  </div>
</div>
