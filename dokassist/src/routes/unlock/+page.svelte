<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { resolve } from '$app/paths';
  import { checkAuth, unlockApp, unlockAppAcceptingAuditCheckpoint, parseError } from '$lib/api';
  import { authStatus } from '$lib/stores/auth';
  import { t } from '$lib/translations';
  import { classifyUnlockError } from '$lib/unlock-errors';
  import { Fingerprint, LoaderCircle } from 'lucide-svelte';
  import AuthCard from '$lib/components/AuthCard.svelte';
  import { Alert, Button } from '$lib/components/ui';

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

<AuthCard title={$t('auth.welcomeBack')} description={$t('auth.unlockSubtitle')}>
  <div class="space-y-5">
    {#if checkpointMismatch}
      <Alert tone="danger" title={$t('auth.checkpointMismatchTitle')}>
        <p class="text-fg">{$t('auth.checkpointMismatchBody')}</p>
        <details class="mt-2 text-caption text-fg-muted">
          <summary class="cursor-pointer">{$t('auth.technicalDetails')}</summary>
          <p class="mt-1 break-words">{checkpointMismatch}</p>
        </details>
        <div class="mt-3">
          <Button
            variant="danger"
            size="lg"
            full
            onclick={handleAcceptCheckpoint}
            disabled={isUnlocking}
          >
            {$t('auth.checkpointMismatchContinue')}
          </Button>
        </div>
      </Alert>
    {:else if error}
      <Alert tone="danger">
        <p>{error}</p>
        {#if errorDetails}
          <details class="mt-2 text-caption text-fg-muted">
            <summary class="cursor-pointer">{$t('auth.technicalDetails')}</summary>
            <p class="mt-1 break-words">{errorDetails}</p>
          </details>
        {/if}
        {#if suggestRecovery}
          <a
            href={resolve('/recover')}
            class="mt-2 block font-medium text-link underline underline-offset-2 hover:text-link-hover"
          >
            {$t('auth.useRecoveryPhrase')}
          </a>
        {/if}
      </Alert>
    {/if}

    <Button variant="primary" size="lg" full onclick={handleUnlock} disabled={isUnlocking}>
      {#if isUnlocking}
        <LoaderCircle size={16} class="animate-spin" aria-hidden="true" />
        <span>{$t('auth.unlocking')}</span>
      {:else}
        <Fingerprint size={18} aria-hidden="true" />
        <span>{$t('auth.unlockWithTouchID')}</span>
      {/if}
    </Button>

    <p class="text-center">
      <a
        href="/recover"
        class="text-body text-link underline-offset-2 hover:text-link-hover hover:underline"
      >
        {$t('auth.recoveryLink')}
      </a>
    </p>
  </div>

  {#snippet footer()}
    <a
      href="/reset"
      class="text-caption text-fg-muted underline-offset-2 transition-colors duration-150 ease-standard hover:text-danger-fg hover:underline"
    >
      {$t('auth.resetLink')}
    </a>
  {/snippet}
</AuthCard>
