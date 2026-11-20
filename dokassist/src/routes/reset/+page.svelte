<script lang="ts">
  import { errorText } from '$lib/translations/labels';
  import { goto } from '$app/navigation';
  import { page } from '$app/state';
  import { checkAuth, resetApp } from '$lib/api';
  import { cancelResetPath } from '$lib/auth-routes';
  import { t } from '$lib/translations';
  import { RotateCcw } from 'lucide-svelte';
  import AuthCard from '$lib/components/AuthCard.svelte';
  import { Alert, Button, Field, Input } from '$lib/components/ui';

  let confirmation = $state('');
  let isResetting = $state(false);
  let error = $state<string | null>(null);

  async function handleCancel() {
    let status = 'locked';
    try {
      status = await checkAuth();
    } catch {
      // Prefer the originating route over a stranded unlock screen.
      status = page.url.searchParams.get('from') === 'setup' ? 'first_run' : 'locked';
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

<AuthCard title={$t('auth.resetTitle')} description={$t('auth.resetDescription')} tone="danger">
  <div class="space-y-5">
    <Alert tone="danger">{$t('auth.resetWarning')}</Alert>

    {#if error}
      <Alert tone="danger">{error}</Alert>
    {/if}

    <Field label={$t('auth.resetPrompt')} for="reset-confirmation">
      <Input
        id="reset-confirmation"
        controlSize="lg"
        bind:value={confirmation}
        autocomplete="off"
        autocapitalize="off"
        spellcheck="false"
        placeholder="RESET"
        class="font-mono"
      />
    </Field>

    <div class="flex gap-3">
      <Button size="lg" class="flex-1" onclick={handleCancel}>
        {$t('common.cancel')}
      </Button>
      <Button
        variant="danger-solid"
        size="lg"
        class="flex-1"
        onclick={handleReset}
        disabled={confirmation !== 'RESET' || isResetting}
      >
        {#if isResetting}
          {$t('auth.resetting')}
        {:else}
          <RotateCcw size={16} aria-hidden="true" />{$t('auth.resetAction')}
        {/if}
      </Button>
    </div>
  </div>
</AuthCard>
