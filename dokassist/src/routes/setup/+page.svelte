<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { initializeApp, parseError } from '$lib/api';
  import { authStatus } from '$lib/stores/auth';
  import { t } from '$lib/translations';
  import MnemonicDisplay from '$lib/components/MnemonicDisplay.svelte';
  import { AlertTriangle } from 'lucide-svelte';

  let words = $state<string[]>([]);
  let isLoading = $state(true);
  let error = $state<string | null>(null);
  let errorCode = $state<string | null>(null);
  let showConfirmation = $state(false);
  let confirmIndices = $state<number[]>([]);
  let userInputs = $state<{ [key: number]: string }>({});
  let confirmError = $state<string | null>(null);

  async function createRecoveryPhrase() {
    isLoading = true;
    error = null;
    errorCode = null;
    try {
      const mnemonic = await initializeApp();
      words = mnemonic;
    } catch (err) {
      const { code, message } = parseError(err);
      // The vault already exists — its phrase was shown to whichever page load
      // created it. Setup has nothing left to do; send the user to unlock.
      if (code === 'ALREADY_INITIALIZED') {
        await goto('/', { replaceState: true });
        return;
      }
      errorCode = code;
      if (code === 'SETUP_IN_PROGRESS') {
        error = $t('auth.setupInProgress');
      } else if (code === 'KEYCHAIN_ERROR') {
        error = $t('auth.setupKeychainError');
      } else {
        error = message;
      }
    } finally {
      isLoading = false;
    }
  }

  onMount(() => {
    void createRecoveryPhrase();
  });

  function startConfirmation() {
    const indices: number[] = [];
    while (indices.length < 3) {
      const randomIndex = Math.floor(Math.random() * 24);
      if (!indices.includes(randomIndex)) {
        indices.push(randomIndex);
      }
    }
    confirmIndices = indices.sort((a, b) => a - b);
    userInputs = {};
    confirmError = null;
    showConfirmation = true;
  }

  function validateConfirmation() {
    for (const index of confirmIndices) {
      if (userInputs[index]?.toLowerCase().trim() !== words[index]?.toLowerCase()) {
        confirmError = $t('auth.confirmWordsError');
        return;
      }
    }
    authStatus.set('unlocked');
    goto('/onboarding/step1');
  }
</script>

<div class="min-h-screen bg-surface-sunken text-fg flex items-center justify-center p-8">
  <div class="max-w-4xl w-full">
    {#if isLoading}
      <div class="text-center">
        <div class="animate-spin rounded-full h-12 w-12 border-b-2 border-accent mx-auto"></div>
        <p class="mt-4 text-fg-muted">{$t('auth.setupGenerating')}</p>
      </div>
    {:else if error}
      <div class="bg-danger-subtle border border-danger-line rounded-card p-6 text-center">
        <h2 class="text-title font-semibold text-danger-fg mb-2">{$t('auth.setupFailed')}</h2>
        <p class="text-fg-muted">{error}</p>
        <button
          onclick={createRecoveryPhrase}
          class="mt-5 px-5 py-2 bg-accent hover:bg-accent-hover text-on-accent font-medium rounded-control transition-colors"
        >
          {$t('auth.retry')}
        </button>
        {#if errorCode !== 'SETUP_IN_PROGRESS'}
          <p class="mt-4">
            <a
              href="/reset?from=setup"
              class="text-caption text-fg-muted hover:text-danger-fg transition-colors"
            >
              {$t('auth.resetLink')}
            </a>
          </p>
        {/if}
      </div>
    {:else if !showConfirmation}
      <div class="space-y-6">
        <div class="text-center">
          <h1 class="text-display font-semibold text-fg mb-2">{$t('auth.welcomeToRamDoc')}</h1>
          <p class="text-fg-muted">
            {$t('auth.setupIntro')}
          </p>
        </div>

        <div class="bg-warning-subtle border border-warning rounded-card p-4">
          <p class="text-warning-fg text-body font-medium flex items-center gap-2">
            <AlertTriangle size={16} />
            {$t('auth.recoveryPhraseDesc')}
          </p>
        </div>

        <MnemonicDisplay {words} />

        <div class="flex justify-center">
          <button
            onclick={startConfirmation}
            class="px-6 py-3 bg-accent hover:bg-accent-hover text-on-accent font-medium rounded-control transition-colors"
          >
            {$t('auth.writtenDown')}
          </button>
        </div>
      </div>
    {:else}
      <div class="space-y-6">
        <div class="text-center">
          <h2 class="text-display font-semibold text-fg mb-2">
            {$t('auth.confirmRecoveryPhrase')}
          </h2>
          <p class="text-fg-muted">{$t('auth.confirmWordsPrompt')}</p>
        </div>

        {#if confirmError}
          <div class="bg-danger-subtle border border-danger-line rounded-card p-4">
            <p class="text-danger-fg text-body">{confirmError}</p>
          </div>
        {/if}

        <div class="space-y-4 max-w-md mx-auto">
          {#each confirmIndices as index}
            <div>
              <label for={`confirm-word-${index}`} class="block text-fg-muted mb-2">
                {$t('auth.wordPlaceholder').replace('{number}', String(index + 1))}
              </label>
              <input
                id={`confirm-word-${index}`}
                type="text"
                bind:value={userInputs[index]}
                class="w-full px-4 py-3 bg-surface-raised border border-line rounded-control text-fg focus:outline-none focus:ring-2 focus:ring-accent/30"
                placeholder={$t('auth.enterWord')}
              />
            </div>
          {/each}
        </div>

        <div class="flex justify-center gap-4">
          <button
            onclick={() => (showConfirmation = false)}
            class="px-6 py-3 border border-line bg-surface-raised hover:bg-surface-hover text-fg font-medium rounded-control transition-colors"
          >
            {$t('common.back')}
          </button>
          <button
            onclick={validateConfirmation}
            class="px-6 py-3 bg-accent hover:bg-accent-hover text-on-accent font-medium rounded-control transition-colors"
          >
            {$t('common.continue')}
          </button>
        </div>
      </div>
    {/if}
  </div>
</div>
