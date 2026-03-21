<script lang="ts">
  import { goto } from '$app/navigation';
  import { recoverApp } from '$lib/api';
  import { authStatus } from '$lib/stores/auth';

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
      error = 'Please enter all 24 words';
      return;
    }

    isRecovering = true;
    error = null;

    try {
      const recovered = await recoverApp(words);

      if (!recovered) {
        error = 'Failed to recover account';
        return;
      }

      authStatus.set('unlocked');
      goto('/dashboard');
    } catch (err) {
      error = err instanceof Error ? err.message : 'Failed to recover account';
    } finally {
      isRecovering = false;
    }
  }
</script>

<div class="min-h-screen bg-gray-950 flex items-center justify-center p-8">
  <div class="max-w-4xl w-full space-y-6">
    <div class="text-center">
      <h1 class="text-3xl font-bold text-gray-100 mb-2">Recover Your Account</h1>
      <p class="text-gray-400">Enter your 24-word recovery phrase</p>
    </div>

    {#if error}
      <div class="bg-red-900/20 border border-red-500 rounded-lg p-4">
        <p class="text-red-500 text-sm">{error}</p>
      </div>
    {/if}

    <div class="grid grid-cols-4 gap-3">
      {#each Array(24) as _, i}
        <div class="flex flex-col">
          <label class="text-gray-400 text-xs mb-1">{i + 1}.</label>
          <input
            type="text"
            value={words[i]}
            oninput={(e) => handleInput(i, (e.target as HTMLInputElement).value)}
            class="px-3 py-2 bg-gray-800 border border-gray-700 rounded-lg text-gray-100 focus:outline-none focus:ring-2 focus:ring-blue-500"
            placeholder={`Word ${i + 1}`}
          />
        </div>
      {/each}
    </div>

    <div class="flex justify-center gap-4">
      <a
        href="/unlock"
        class="px-6 py-3 bg-gray-700 hover:bg-gray-600 text-white font-medium rounded-lg transition-colors"
      >
        Back
      </a>
      <button
        onclick={handleRecover}
        disabled={isRecovering}
        class="px-6 py-3 bg-blue-600 hover:bg-blue-700 disabled:bg-gray-700 disabled:cursor-not-allowed text-white font-medium rounded-lg transition-colors flex items-center gap-2"
      >
        {#if isRecovering}
          <div class="animate-spin rounded-full h-5 w-5 border-b-2 border-white"></div>
          <span>Recovering...</span>
        {:else}
          <span>Recover Account</span>
        {/if}
      </button>
    </div>
  </div>
</div>
