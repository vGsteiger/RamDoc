<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { initializeApp } from '$lib/api';
  import { authStatus } from '$lib/stores/auth';
  import MnemonicDisplay from '$lib/components/MnemonicDisplay.svelte';

  let words = $state<string[]>([]);
  let isLoading = $state(true);
  let error = $state<string | null>(null);
  let showConfirmation = $state(false);
  let confirmIndices = $state<number[]>([]);
  let userInputs = $state<{ [key: number]: string }>({});
  let confirmError = $state<string | null>(null);

  onMount(async () => {
    try {
      const mnemonic = await initializeApp();
      words = mnemonic;
      isLoading = false;
    } catch (err) {
      error = err instanceof Error ? err.message : 'Failed to initialize app';
      isLoading = false;
    }
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
        confirmError = 'One or more words are incorrect. Please try again.';
        return;
      }
    }
    authStatus.set('unlocked');
    goto('/patients');
  }
</script>

<div class="min-h-screen bg-gray-950 flex items-center justify-center p-8">
  <div class="max-w-4xl w-full">
    {#if isLoading}
      <div class="text-center">
        <div class="animate-spin rounded-full h-12 w-12 border-b-2 border-blue-500 mx-auto"></div>
        <p class="mt-4 text-gray-400">Generating keys...</p>
      </div>
    {:else if error}
      <div class="bg-red-900/20 border border-red-500 rounded-lg p-6 text-center">
        <h2 class="text-xl font-bold text-red-500 mb-2">Setup Failed</h2>
        <p class="text-gray-300">{error}</p>
      </div>
    {:else if !showConfirmation}
      <div class="space-y-6">
        <div class="text-center">
          <h1 class="text-3xl font-bold text-gray-100 mb-2">Welcome to DokAssist</h1>
          <p class="text-gray-400">
            Please write down these 24 words in order. You'll need them to recover your account.
          </p>
        </div>

        <div class="bg-yellow-900/20 border border-yellow-600 rounded-lg p-4">
          <p class="text-yellow-500 text-sm font-medium">
            ⚠️ Store these words safely. They cannot be recovered if lost.
          </p>
        </div>

        <MnemonicDisplay {words} />

        <div class="flex justify-center">
          <button
            onclick={startConfirmation}
            class="px-6 py-3 bg-blue-600 hover:bg-blue-700 text-white font-medium rounded-lg transition-colors"
          >
            I've written them down
          </button>
        </div>
      </div>
    {:else}
      <div class="space-y-6">
        <div class="text-center">
          <h2 class="text-2xl font-bold text-gray-100 mb-2">Confirm Your Recovery Phrase</h2>
          <p class="text-gray-400">Please enter the following words to confirm:</p>
        </div>

        {#if confirmError}
          <div class="bg-red-900/20 border border-red-500 rounded-lg p-4">
            <p class="text-red-500 text-sm">{confirmError}</p>
          </div>
        {/if}

        <div class="space-y-4 max-w-md mx-auto">
          {#each confirmIndices as index}
            <div>
              <label class="block text-gray-400 mb-2">
                Word #{index + 1}
              </label>
              <input
                type="text"
                bind:value={userInputs[index]}
                class="w-full px-4 py-3 bg-gray-800 border border-gray-700 rounded-lg text-gray-100 focus:outline-none focus:ring-2 focus:ring-blue-500"
                placeholder="Enter word"
              />
            </div>
          {/each}
        </div>

        <div class="flex justify-center gap-4">
          <button
            onclick={() => (showConfirmation = false)}
            class="px-6 py-3 bg-gray-700 hover:bg-gray-600 text-white font-medium rounded-lg transition-colors"
          >
            Back
          </button>
          <button
            onclick={validateConfirmation}
            class="px-6 py-3 bg-blue-600 hover:bg-blue-700 text-white font-medium rounded-lg transition-colors"
          >
            Continue
          </button>
        </div>
      </div>
    {/if}
  </div>
</div>
