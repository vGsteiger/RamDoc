<script lang="ts">
  import { errorText } from '$lib/translations/labels';
  import { goto } from '$app/navigation';
  import { recoverApp } from '$lib/api';
  import { authStatus } from '$lib/stores/auth';
  import { t } from '$lib/translations';
  import { LoaderCircle } from 'lucide-svelte';
  import AuthCard from '$lib/components/AuthCard.svelte';
  import { Alert, Button, Input } from '$lib/components/ui';

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

<AuthCard title={$t('auth.recoveryTitle')} description={$t('auth.recoverySubtitle')} width="lg">
  <div class="space-y-6">
    {#if error}
      <Alert tone="danger">{error}</Alert>
    {/if}

    <Alert tone="info" title={$t('auth.recoveryNoticeTitle')}>
      {$t('auth.recoveryNotice')}
    </Alert>

    <div class="grid grid-cols-2 gap-x-3 gap-y-2.5 sm:grid-cols-3 md:grid-cols-4">
      {#each Array(24) as _, i (i)}
        <div class="flex min-w-0 items-center gap-2">
          <label
            for={`word-${i}`}
            class="w-5 shrink-0 text-right text-caption text-fg-subtle"
            data-numeric>{i + 1}.</label
          >
          <Input
            id={`word-${i}`}
            type="text"
            controlSize="lg"
            value={words[i]}
            oninput={(e: Event) => handleInput(i, (e.target as HTMLInputElement).value)}
            placeholder={$t('auth.wordPlaceholder').replace('{number}', String(i + 1))}
            autocomplete="off"
            autocapitalize="off"
            spellcheck="false"
            class="font-mono"
          />
        </div>
      {/each}
    </div>

    <div class="flex justify-center">
      <Button variant="primary" size="lg" onclick={handleRecover} disabled={isRecovering}>
        {#if isRecovering}
          <LoaderCircle size={16} class="animate-spin" aria-hidden="true" />
          <span>{$t('auth.recoveryInProgress')}</span>
        {:else}
          <span>{$t('auth.recoverAccount')}</span>
        {/if}
      </Button>
    </div>
  </div>

  <!-- Without the phrase there is no way back in; offer the same
       RESET-confirmed factory reset the unlock screen links to. -->
  {#snippet footer()}
    <a
      href="/reset?from=recover"
      class="text-caption text-fg-muted underline-offset-2 transition-colors duration-150 ease-standard hover:text-danger-fg hover:underline"
    >
      {$t('auth.resetLink')}
    </a>
  {/snippet}
</AuthCard>
