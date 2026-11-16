<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { resolve } from '$app/paths';
  import { checkAuth, unlockApp, unlockAppAcceptingAuditCheckpoint, parseError } from '$lib/api';
  import { authStatus } from '$lib/stores/auth';
  import { t } from '$lib/translations';
  import { classifyUnlockError } from '$lib/unlock-errors';
  import { Fingerprint, KeyRound } from 'lucide-svelte';

  let isUnlocking = $state(false);
  let error = $state<string | null>(null);
  let errorDetails = $state<string | null>(null);
  let suggestRecovery = $state(false);
  /** Backend description of an audit checkpoint mismatch awaiting confirmation. */
  let checkpointMismatch = $state<string | null>(null);

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

  async function attemptUnlock(unlock: () => Promise<boolean>) {
    if (isUnlocking) return;
    isUnlocking = true;
    error = null;
    errorDetails = null;
    suggestRecovery = false;

    try {
      const unlocked = await unlock();
      if (!unlocked) {
        error = $t('auth.unlockFailed');
        return;
      }
      checkpointMismatch = null;
      authStatus.set('unlocked');
      goto('/dashboard');
    } catch (err) {
      const { code, message, ref } = parseError(err);
      const outcome = classifyUnlockError(code);
      switch (outcome.kind) {
        case 'ignore':
          return;
        case 'recovery_required':
          // A missing or invalidated master-key item cannot be restored by trying
          // Touch ID again. The backend has already moved this session to recovery.
          authStatus.set('recovery_required');
          await goto('/recover', { replaceState: true });
          return;
        case 'checkpoint_mismatch':
          checkpointMismatch = message;
          return;
        case 'error':
          checkpointMismatch = null;
          error = outcome.messageKey ? $t(outcome.messageKey) : message || $t('auth.unlockFailed');
          errorDetails = outcome.showDetails ? `${message} (${ref})` : null;
          suggestRecovery = outcome.suggestRecovery;
      }
    } finally {
      isUnlocking = false;
    }
  }

  const handleUnlock = () => attemptUnlock(unlockApp);
  const handleAcceptCheckpoint = () => attemptUnlock(unlockAppAcceptingAuditCheckpoint);
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

    {#if checkpointMismatch}
      <div
        class="rounded-card border border-danger-line/50 bg-danger-subtle/40 p-4 text-left space-y-3"
        role="alert"
      >
        <p class="text-body font-semibold text-danger-fg">{$t('auth.checkpointMismatchTitle')}</p>
        <p class="text-body text-fg">{$t('auth.checkpointMismatchBody')}</p>
        <details class="text-caption text-fg-muted">
          <summary class="cursor-pointer">{$t('auth.technicalDetails')}</summary>
          <p class="mt-1 break-words">{checkpointMismatch}</p>
        </details>
        <button
          onclick={handleAcceptCheckpoint}
          disabled={isUnlocking}
          class="w-full px-4 py-3 border border-danger-line text-danger-fg hover:bg-danger-subtle/60 focus:outline-none focus:ring-2 focus:ring-danger-line/40 disabled:cursor-not-allowed disabled:opacity-60 font-medium rounded-card transition-colors"
        >
          {$t('auth.checkpointMismatchContinue')}
        </button>
      </div>
    {:else if error}
      <div
        class="rounded-card border border-danger-line/50 bg-danger-subtle/40 p-4 text-left space-y-2"
        role="alert"
      >
        <p class="text-body text-danger-fg">{error}</p>
        {#if errorDetails}
          <details class="text-caption text-fg-muted">
            <summary class="cursor-pointer">{$t('auth.technicalDetails')}</summary>
            <p class="mt-1 break-words">{errorDetails}</p>
          </details>
        {/if}
        {#if suggestRecovery}
          <a href={resolve('/recover')} class="block text-body font-medium text-accent-fg hover:underline">
            {$t('auth.useRecoveryPhrase')}
          </a>
        {/if}
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
