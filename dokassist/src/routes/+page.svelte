<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { checkAuth, getSettings } from '$lib/api';
  import { authStatus, isLoading } from '$lib/stores/auth';
  import { t } from '$lib/translations';

  let error = $state<string | null>(null);

  onMount(async () => {
    try {
      const status = await checkAuth();
      authStatus.set(status);

      if (status === 'first_run') {
        goto('/setup');
      } else if (status === 'locked') {
        goto('/unlock');
      } else if (status === 'recovery_required') {
        goto('/recover');
      } else if (status === 'unlocked') {
        // Check if onboarding is completed
        try {
          const settings = await getSettings();
          if (!settings.onboarding_completed) {
            goto('/onboarding/step1');
          } else {
            goto('/dashboard');
          }
        } catch (err) {
          // If settings fetch fails, go to dashboard anyway
          console.error('Failed to check onboarding status:', err);
          goto('/dashboard');
        }
      }
    } catch (err) {
      console.error('Failed to check auth:', err);
      error = err instanceof Error ? err.message : 'Failed to check authentication status';
    } finally {
      isLoading.set(false);
    }
  });

  function handleRetry() {
    error = null;
    isLoading.set(true);
    onMount(async () => {
      try {
        const status = await checkAuth();
        authStatus.set(status);

        if (status === 'first_run') {
          goto('/setup');
        } else if (status === 'locked') {
          goto('/unlock');
        } else if (status === 'recovery_required') {
          goto('/recover');
        } else if (status === 'unlocked') {
          // Check if onboarding is completed
          try {
            const settings = await getSettings();
            if (!settings.onboarding_completed) {
              goto('/onboarding/step1');
            } else {
              goto('/dashboard');
            }
          } catch (err) {
            // If settings fetch fails, go to dashboard anyway
            console.error('Failed to check onboarding status:', err);
            goto('/dashboard');
          }
        }
      } catch (err) {
        console.error('Failed to check auth:', err);
        error = err instanceof Error ? err.message : 'Failed to check authentication status';
      } finally {
        isLoading.set(false);
      }
    });
  }
</script>

<div class="min-h-screen bg-gray-950 flex items-center justify-center p-8">
  {#if error}
    <div class="text-center space-y-4 max-w-md">
      <div class="bg-red-900/20 border border-red-500 rounded-lg p-6">
        <h2 class="text-xl font-bold text-red-500 mb-2">{$t('auth.authError')}</h2>
        <p class="text-gray-300">{error}</p>
      </div>
      <button
        onclick={handleRetry}
        class="px-6 py-3 bg-blue-600 hover:bg-blue-700 text-white font-medium rounded-lg transition-colors"
      >
        {$t('auth.retry')}
      </button>
    </div>
  {:else}
    <div class="text-center">
      <div class="animate-spin rounded-full h-12 w-12 border-b-2 border-blue-500 mx-auto"></div>
      <p class="mt-4 text-gray-400">{$t('auth.loading')}</p>
    </div>
  {/if}
</div>
