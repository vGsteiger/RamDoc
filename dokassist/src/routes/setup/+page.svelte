<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { initializeApp, parseError } from '$lib/api';
  import { authStatus } from '$lib/stores/auth';
  import { t } from '$lib/translations';
  import MnemonicDisplay from '$lib/components/MnemonicDisplay.svelte';
  import AuthCard from '$lib/components/AuthCard.svelte';
  import { Alert, Button, Field, Input, Spinner } from '$lib/components/ui';

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
      } else if (code === 'KEYCHAIN_ERROR' || code === 'KEYCHAIN_ACCESS_DENIED') {
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

{#if isLoading}
  <AuthCard>
    <div class="flex justify-center py-4">
      <Spinner label={$t('auth.setupGenerating')} />
    </div>
  </AuthCard>
{:else if error}
  <AuthCard title={$t('auth.setupFailed')}>
    <Alert tone="danger">{error}</Alert>
    <div class="mt-6">
      <Button variant="primary" size="lg" full onclick={createRecoveryPhrase}>
        {$t('auth.retry')}
      </Button>
    </div>
    {#snippet footer()}
      {#if errorCode !== 'SETUP_IN_PROGRESS'}
        <a
          href="/reset?from=setup"
          class="text-caption text-fg-muted underline-offset-2 transition-colors duration-150 ease-standard hover:text-danger-fg hover:underline"
        >
          {$t('auth.resetLink')}
        </a>
      {/if}
    {/snippet}
  </AuthCard>
{:else if !showConfirmation}
  <AuthCard title={$t('auth.welcomeToRamDoc')} description={$t('auth.setupIntro')} width="lg">
    <div class="space-y-6">
      <Alert tone="warning">{$t('auth.recoveryPhraseDesc')}</Alert>

      <MnemonicDisplay {words} />

      <div class="flex justify-center">
        <Button variant="primary" size="lg" onclick={startConfirmation}>
          {$t('auth.writtenDown')}
        </Button>
      </div>
    </div>
  </AuthCard>
{:else}
  <AuthCard title={$t('auth.confirmRecoveryPhrase')} description={$t('auth.confirmWordsPrompt')}>
    <div class="space-y-5">
      {#if confirmError}
        <Alert tone="danger">{confirmError}</Alert>
      {/if}

      <div class="space-y-4">
        {#each confirmIndices as index (index)}
          <Field
            label={$t('auth.wordPlaceholder').replace('{number}', String(index + 1))}
            for={`confirm-word-${index}`}
          >
            <Input
              id={`confirm-word-${index}`}
              type="text"
              controlSize="lg"
              value={userInputs[index] ?? ''}
              oninput={(e: Event) =>
                (userInputs[index] = (e.currentTarget as HTMLInputElement).value)}
              placeholder={$t('auth.enterWord')}
              autocomplete="off"
              autocapitalize="off"
              spellcheck="false"
            />
          </Field>
        {/each}
      </div>

      <div class="flex gap-3">
        <Button size="lg" class="flex-1" onclick={() => (showConfirmation = false)}>
          {$t('common.back')}
        </Button>
        <Button variant="primary" size="lg" class="flex-1" onclick={validateConfirmation}>
          {$t('common.continue')}
        </Button>
      </div>
    </div>
  </AuthCard>
{/if}
