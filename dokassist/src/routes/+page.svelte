<script lang="ts">
  import { errorText } from '$lib/translations/labels';
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { checkAuth, getSettings } from '$lib/api';
  import { authStatus, isLoading } from '$lib/stores/auth';
  import { t } from '$lib/translations';

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
            // The conversation workspace is the daily starting point. Dashboard
            // remains available from navigation for clinicians who need its overview.
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

<div class="min-h-screen bg-surface-sunken text-fg flex items-center justify-center p-8">
  {#if error}
    <div class="text-center space-y-4 max-w-md">
      <div class="bg-danger-subtle border border-danger-line rounded-card p-6">
        <h2 class="text-title font-semibold text-danger-fg mb-2">{$t('auth.authError')}</h2>
        <p class="text-fg-muted">{error}</p>
      </div>
      <button
        onclick={handleRetry}
        class="px-6 py-3 bg-accent hover:bg-accent-hover text-on-accent font-medium rounded-control transition-colors"
      >
        {$t('auth.retry')}
      </button>
    </div>
  {:else}
    <div class="text-center">
      <div class="animate-spin rounded-full h-12 w-12 border-b-2 border-accent mx-auto"></div>
      <p class="mt-4 text-fg-muted">{$t('auth.loading')}</p>
    </div>
  {/if}
</div>
