<script lang="ts">
  import { errorText } from '$lib/translations/labels';
  import { goto } from '$app/navigation';
  import { page } from '$app/stores';
  import { checkAuth, resetApp } from '$lib/api';
  import { cancelResetPath } from '$lib/auth-routes';
  import { t } from '$lib/translations';
  import { AlertTriangle, RotateCcw } from 'lucide-svelte';

  let confirmation = $state('');
  let isResetting = $state(false);
  let error = $state<string | null>(null);

  async function handleCancel() {
    let status = 'locked';
    try {
      status = await checkAuth();
    } catch {
      // Prefer the originating route over a stranded unlock screen.
      status = $page.url.searchParams.get('from') === 'setup' ? 'first_run' : 'locked';
    }
    await goto(cancelResetPath(status));
  }

  async function handleReset() {
    if (confirmation !== 'RESET' || isResetting) return;

    isResetting = true;
    error = null;
    try {
      await resetApp();
      await goto('/setup', { replaceState: true });
    } catch (err) {
      error = $errorText(err, $t('auth.resetFailed'));
    } finally {
      isResetting = false;
    }
  }
</script>

<div class="min-h-screen bg-surface-sunken text-fg flex items-center justify-center p-8">
  <main
    class="max-w-md w-full rounded-card border border-danger-line bg-surface-raised p-8 shadow-modal space-y-6"
  >
    <div class="text-center space-y-3">
      <div
        class="mx-auto flex h-12 w-12 items-center justify-center rounded-full bg-danger-subtle text-danger-fg"
      >
        <AlertTriangle size={24} aria-hidden="true" />
      </div>
      <h1 class="text-display font-semibold">{$t('auth.resetTitle')}</h1>
      <p class="text-body text-fg-muted">{$t('auth.resetDescription')}</p>
    </div>

    <div
      class="rounded-card border border-danger-line bg-danger-subtle p-4 text-body text-danger-fg"
    >
      {$t('auth.resetWarning')}
    </div>

    {#if error}
      <p
        class="rounded-card border border-danger-line/50 bg-danger-subtle p-4 text-body text-danger-fg"
        role="alert"
      >
        {error}
      </p>
    {/if}

    <label for="reset-confirmation" class="block text-body font-medium text-fg-muted">
      {$t('auth.resetPrompt')}
    </label>
    <input
      id="reset-confirmation"
      bind:value={confirmation}
      autocomplete="off"
      class="mt-2 w-full rounded-card border border-line bg-surface-raised px-4 py-3 text-fg outline-none focus:ring-2 focus:ring-danger/30"
      placeholder="RESET"
    />

    <div class="flex gap-3">
      <button
        type="button"
        onclick={handleCancel}
        class="inline-flex items-center justify-center flex-1 rounded-card border border-line h-8 px-3 text-center text-body font-medium text-fg-muted hover:bg-surface-hover"
      >
        {$t('common.cancel')}
      </button>
      <button
        onclick={handleReset}
        disabled={confirmation !== 'RESET' || isResetting}
        class="flex-1 rounded-card bg-danger h-8 px-3 text-body font-medium text-on-danger hover:bg-danger disabled:cursor-not-allowed disabled:bg-surface-selected"
      >
        {#if isResetting}
          {$t('auth.resetting')}
        {:else}
          <span class="inline-flex items-center gap-2"
            ><RotateCcw size={16} aria-hidden="true" />{$t('auth.resetAction')}</span
          >
        {/if}
      </button>
    </div>
  </main>
</div>
