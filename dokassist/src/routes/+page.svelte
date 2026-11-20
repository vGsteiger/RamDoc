<script lang="ts">
  import { errorText } from '$lib/translations/labels';
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { checkAuth, getSettings } from '$lib/api';
  import { authStatus, isLoading } from '$lib/stores/auth';
  import { t } from '$lib/translations';
  import { Alert, BrandMark, Button, Spinner } from '$lib/components/ui';

  let error = $state<string | null>(null);

  async function checkAndRedirect() {
    try {
      const status = await checkAuth();
      authStatus.set(status);

      if (status === 'first_run' || status === 'initializing') {
        // A setup run started by an earlier page load may still be generating
        // keys; /setup reports that rather than starting a second one.
        goto('/setup');
      } else if (status === 'locked') {
        goto('/unlock');
      } else if (status === 'recovery_required') {
        goto('/recover');
      } else if (status === 'unlocked') {
        try {
          const settings = await getSettings();
          if (!settings.onboarding_completed) {
            goto('/onboarding/step1');
          } else {
            // The conversation workspace is the daily starting point (D3: the
            // dashboard's "Today" content lives on the new-chat screen).
            goto('/chat');
          }
        } catch (err) {
          console.error('Failed to check onboarding status:', err);
          goto('/onboarding/step1');
        }
      }
    } catch (err) {
      console.error('Failed to check auth:', err);
      error = $errorText(err);
    } finally {
      isLoading.set(false);
    }
  }

  onMount(() => {
    checkAndRedirect();
  });

  function handleRetry() {
    error = null;
    isLoading.set(true);
    checkAndRedirect();
  }
</script>

<div class="flex min-h-screen items-center justify-center bg-surface p-8 text-fg">
  <div class="flex w-full max-w-sm flex-col items-center gap-4 text-center">
    <BrandMark size={32} />
    {#if error}
      <Alert tone="danger" title={$t('auth.authError')} class="w-full text-left">{error}</Alert>
      <Button size="lg" onclick={handleRetry}>{$t('auth.retry')}</Button>
    {:else}
      <Spinner size={20} label={$t('auth.loading')} />
    {/if}
  </div>
</div>
