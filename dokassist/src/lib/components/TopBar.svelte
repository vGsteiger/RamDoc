<script lang="ts">
  import { onMount } from 'svelte';
  import { getEngineStatus } from '$lib/api';

  let searchInput = $state<HTMLInputElement | null>(null);
  let llmStatus = $state<'loaded' | 'not_loaded'>('not_loaded');

  onMount(() => {
    const handleKeydown = (e: KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && e.key === 'k') {
        e.preventDefault();
        searchInput?.focus();
      }
    };

    window.addEventListener('keydown', handleKeydown);

    // Check LLM status on mount
    updateLlmStatus();

    // Poll for LLM status every 5 seconds
    const interval = setInterval(updateLlmStatus, 5000);

    return () => {
      window.removeEventListener('keydown', handleKeydown);
      clearInterval(interval);
    };
  });

  async function updateLlmStatus() {
    try {
      const status = await getEngineStatus();
      llmStatus = status.is_loaded ? 'loaded' : 'not_loaded';
    } catch (error) {
      console.error('Failed to get LLM status:', error);
      llmStatus = 'not_loaded';
    }
  }

  function handleSearch(e: Event) {
    // TODO: Implement search functionality
  }
</script>

<header class="h-16 bg-gray-900 border-b border-gray-800 flex items-center px-6 gap-4">
  <div class="flex-1 max-w-2xl">
    <input
      bind:this={searchInput}
      type="text"
      placeholder="Search patients, files... (⌘K)"
      class="w-full px-4 py-2 bg-gray-800 border border-gray-700 rounded-lg text-gray-100 placeholder-gray-400 focus:outline-none focus:ring-2 focus:ring-blue-500"
      oninput={handleSearch}
    />
  </div>

  <div class="flex items-center gap-2">
    <span class="text-sm text-gray-400">LLM:</span>
    <div
      class="w-3 h-3 rounded-full {llmStatus === 'loaded' ? 'bg-green-500' : 'bg-red-500'}"
      title={llmStatus === 'loaded' ? 'Model loaded' : 'No model'}
    ></div>
  </div>
</header>
