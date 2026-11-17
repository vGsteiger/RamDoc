<script lang="ts">
  import { errorText } from '$lib/translations/labels';
  import { goto } from '$app/navigation';
  import { recoverApp } from '$lib/api';
  import { authStatus } from '$lib/stores/auth';
  import { t } from '$lib/translations';
  import { ShieldCheck } from 'lucide-svelte';

  let words = $state<string[]>(Array(24).fill(''));
  let isRecovering = $state(false);
  let error = $state<string | null>(null);

  function handleInput(index: number, value: string) {
    words[index] = value.toLowerCase().trim();
  }

  async function handleRecover() {
    if (isRecovering) return;

    const filledWords = words.filter((w) => w.length > 0);
    if (filledWords.length !== 24) {
      error = $t('auth.recoveryMissingWords');
      return;
    }

    isRecovering = true;
    error = null;

    try {
      const recovered = await recoverApp(words);

      if (!recovered) {
        error = $t('auth.recoveryFailed');
        return;
      }

      authStatus.set('unlocked');
      goto('/dashboard');
    } catch (err) {
      error = $errorText(err, $t('auth.recoveryFailed'));
    } finally {
      isRecovering = false;
    }
  }
</script>

<div class="min-h-screen bg-surface-sunken text-fg flex items-center justify-center p-8">
  <main
    class="max-w-4xl w-full rounded-card border border-line bg-surface-raised p-6 sm:p-8 space-y-6 shadow-modal"
  >
    <div class="text-center space-y-3">
      <div
        class="mx-auto flex h-12 w-12 items-center justify-center rounded-full bg-success-subtle/15 text-success-fg"
      >
        <ShieldCheck size={24} aria-hidden="true" />
      </div>
      <h1 class="text-display font-semibold text-fg">{$t('auth.recoveryTitle')}</h1>
      <p class="mx-auto max-w-2xl text-fg-muted">{$t('auth.recoverySubtitle')}</p>
    </div>

    {#if error}
      <div class="rounded-card border border-danger-line/50 bg-danger-subtle/40 p-4" role="alert">
        <p class="text-body text-danger-fg">{error}</p>
      </div>
    {/if}

    <div
      class="rounded-card border border-success-line/20 bg-success-subtle/5 p-4 text-body text-success-fg"
    >
      <p class="font-medium">{$t('auth.recoveryNoticeTitle')}</p>
      <p class="mt-1 text-success-fg/75">{$t('auth.recoveryNotice')}</p>
    </div>

    <div class="grid grid-cols-4 gap-3">
      {#each Array(24) as _, i}
        <div class="flex flex-col">
          <label for={`word-${i}`} class="text-fg-muted text-caption mb-1">{i + 1}.</label>
          <input
            id={`word-${i}`}
            type="text"
            value={words[i]}
            oninput={(e) => handleInput(i, (e.target as HTMLInputElement).value)}
            class="px-3 py-2 bg-surface-raised border border-line rounded-control text-fg focus:outline-none focus:ring-2 focus:ring-accent/30"
            placeholder={$t('auth.wordPlaceholder').replace('{number}', String(i + 1))}
          />
        </div>
      {/each}
    </div>

    <div class="flex justify-center">
      <button
        onclick={handleRecover}
        disabled={isRecovering}
        class="px-6 py-3 bg-accent hover:bg-accent-hover focus:outline-none focus:ring-2 focus:ring-accent/30 focus:ring-offset-2 focus:ring-offset-surface-raised disabled:bg-surface-selected disabled:cursor-not-allowed text-on-accent font-medium rounded-card transition-colors flex items-center gap-2"
      >
        {#if isRecovering}
          <div class="animate-spin rounded-full h-5 w-5 border-b-2 border-on-accent"></div>
          <span>{$t('auth.recoveryInProgress')}</span>
        {:else}
          <span>{$t('auth.recoverAccount')}</span>
        {/if}
      </button>
    </div>

    <!-- Without the phrase there is no way back in; offer the same
         RESET-confirmed factory reset the unlock screen links to. -->
    <div class="border-t border-line pt-6">
      <a
        href="/reset?from=recover"
        class="text-caption text-fg-muted hover:text-danger-fg transition-colors"
      >
        {$t('auth.resetLink')}
      </a>
    </div>
  </main>
</div>
