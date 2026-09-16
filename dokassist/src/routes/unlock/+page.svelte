<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { checkAuth, unlockApp, parseError } from '$lib/api';
  import { authStatus } from '$lib/stores/auth';
  import { t } from '$lib/translations';
  import { Fingerprint, KeyRound } from 'lucide-svelte';

  let isUnlocking = $state(false);
  let error = $state<string | null>(null);

  // The browser history (or a manually entered URL) can reach this page even
  // though recovery is required. Do not offer a locked-only unlock action in
  // that state.
  onMount(() => {
    void (async () => {
      const status = await checkAuth();
      if (status === 'recovery_required') {
        authStatus.set('recovery_required');
        await goto('/recover', { replaceState: true });
      } else if (status === 'first_run' || status === 'initializing') {
        authStatus.set(status);
        await goto('/setup', { replaceState: true });
      }
    })();
  });

  function friendlyError(err: unknown): string {
    const { code, message } = parseError(err);
    switch (code) {
      case 'KEYCHAIN_ERROR':
        return $t('auth.keychainAccessError');
      case 'DATABASE_ERROR':
        return $t('auth.databaseAccessError');
      case 'FILESYSTEM_ERROR':
        return $t('auth.filesystemAccessError');
      case 'AUTH_REQUIRED':
        return $t('auth.unlockFailed');
      default:
        return message || $t('auth.unlockFailed');
    }
  }

  async function handleUnlock() {
    if (isUnlocking) return;
    isUnlocking = true;
    error = null;

    try {
      const unlocked = await unlockApp();
      if (!unlocked) {
        error = $t('auth.unlockFailed');
        return;
      }
      authStatus.set('unlocked');
      goto('/dashboard');
    } catch (err) {
      const { code } = parseError(err);
      // User dismissed the Touch ID sheet — not an error, just stay on screen.
      if (code === 'BIOMETRIC_CANCELLED') return;
      // A missing or invalidated master-key item cannot be restored by trying
      // Touch ID again. The backend has already moved this session to recovery.
      if (code === 'RECOVERY_REQUIRED') {
        authStatus.set('recovery_required');
        await goto('/recover', { replaceState: true });
        return;
      }
      error = friendlyError(err);
    } finally {
      isUnlocking = false;
    }
  }
</script>

<div class="min-h-screen bg-surface-sunken text-fg flex items-center justify-center p-8">
  <main
    class="max-w-md w-full rounded-card border border-line bg-surface-raised p-8 text-center shadow-modal space-y-8"
  >
    <div class="space-y-3">
      <div
        class="mx-auto flex h-12 w-12 items-center justify-center rounded-full bg-accent-subtle/15 text-accent-fg"
      >
        <KeyRound size={24} aria-hidden="true" />
      </div>
      <h1 class="text-display font-semibold text-fg">{$t('auth.welcomeBack')}</h1>
      <p class="text-fg-muted">{$t('auth.unlockSubtitle')}</p>
    </div>

    {#if error}
      <div
        class="rounded-card border border-danger-line/50 bg-danger-subtle/40 p-4 text-left"
        role="alert"
      >
        <p class="text-body text-danger-fg">{error}</p>
      </div>
    {/if}

    <div class="space-y-4">
      <button
        onclick={handleUnlock}
        disabled={isUnlocking}
        class="w-full px-6 py-4 bg-accent hover:bg-accent-hover focus:outline-none focus:ring-2 focus:ring-accent/30 focus:ring-offset-2 focus:ring-offset-surface-raised disabled:bg-surface-selected disabled:cursor-not-allowed text-on-accent font-medium rounded-card transition-colors flex items-center justify-center gap-3"
      >
        {#if isUnlocking}
          <div class="animate-spin rounded-full h-5 w-5 border-b-2 border-on-accent"></div>
          <span>{$t('auth.unlocking')}</span>
        {:else}
          <Fingerprint size={22} aria-hidden="true" />
          <span>{$t('auth.unlockWithTouchID')}</span>
        {/if}
      </button>

      <a
        href="/recover"
        class="block text-body text-accent-fg hover:text-accent-fg transition-colors"
      >
        {$t('auth.recoveryLink')}
      </a>
    </div>

    <div class="border-t border-line pt-6">
      <a href="/reset" class="text-caption text-fg-muted hover:text-danger-fg transition-colors">
        {$t('auth.resetLink')}
      </a>
    </div>
  </main>
</div>
