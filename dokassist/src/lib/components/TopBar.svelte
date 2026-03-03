<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { getEngineStatus, globalSearch, type SearchResult } from '$lib/api';

  let searchInput = $state<HTMLInputElement | null>(null);
  let llmStatus = $state<'loaded' | 'not_loaded'>('not_loaded');
  let searchQuery = $state('');
  let searchResults = $state<SearchResult[]>([]);
  let showDropdown = $state(false);
  let isSearching = $state(false);
  let searchTimeout: ReturnType<typeof setTimeout> | null = null;

  onMount(() => {
    const handleKeydown = (e: KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && e.key === 'k') {
        e.preventDefault();
        searchInput?.focus();
      }
      if (e.key === 'Escape') {
        closeDropdown();
      }
    };

    window.addEventListener('keydown', handleKeydown);
    updateLlmStatus();
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
    const query = (e.target as HTMLInputElement).value;
    searchQuery = query;

    if (searchTimeout) clearTimeout(searchTimeout);

    if (!query.trim()) {
      searchResults = [];
      showDropdown = false;
      return;
    }

    searchTimeout = setTimeout(async () => {
      isSearching = true;
      try {
        searchResults = await globalSearch(query, 20);
        showDropdown = true;
      } catch (err) {
        console.error('Search error:', err);
        searchResults = [];
      } finally {
        isSearching = false;
      }
    }, 300);
  }

  function handleBlur() {
    // Delay close so clicks on results register first
    setTimeout(() => {
      showDropdown = false;
    }, 150);
  }

  function closeDropdown() {
    showDropdown = false;
    searchQuery = '';
    searchResults = [];
    searchInput?.blur();
  }

  function navigateTo(result: SearchResult) {
    closeDropdown();
    switch (result.result_type) {
      case 'patient':
        goto(`/patients/${result.entity_id}`);
        break;
      case 'file':
        goto(`/patients/${result.patient_id}/files`);
        break;
      case 'session':
        goto(`/patients/${result.patient_id}/sessions`);
        break;
      case 'diagnosis':
        goto(`/patients/${result.patient_id}/diagnoses`);
        break;
      case 'medication':
        goto(`/patients/${result.patient_id}/medications`);
        break;
      case 'report':
        goto(`/patients/${result.patient_id}/reports/${result.entity_id}`);
        break;
      default:
        goto(`/patients/${result.patient_id}`);
    }
  }

  const typeLabel: Record<string, string> = {
    patient: 'Patient',
    file: 'File',
    session: 'Session',
    diagnosis: 'Diagnosis',
    medication: 'Medication',
    report: 'Report',
  };
</script>

<header class="h-16 bg-gray-900 border-b border-gray-800 flex items-center px-6 gap-4">
  <div class="flex-1 max-w-2xl relative">
    <input
      bind:this={searchInput}
      type="text"
      placeholder="Search patients, files... (⌘K)"
      class="w-full px-4 py-2 bg-gray-800 border border-gray-700 rounded-lg text-gray-100 placeholder-gray-400 focus:outline-none focus:ring-2 focus:ring-blue-500"
      value={searchQuery}
      oninput={handleSearch}
      onblur={handleBlur}
    />

    {#if showDropdown && searchQuery.trim()}
      <div class="absolute top-full left-0 right-0 mt-1 bg-gray-800 border border-gray-700 rounded-lg shadow-xl z-50 max-h-96 overflow-y-auto">
        {#if isSearching}
          <div class="px-4 py-3 text-sm text-gray-400">Searching...</div>
        {:else if searchResults.length === 0}
          <div class="px-4 py-3 text-sm text-gray-400">No results for "{searchQuery}"</div>
        {:else}
          {#each searchResults as result (result.entity_id)}
            <button
              class="w-full text-left px-4 py-3 hover:bg-gray-700 transition-colors border-b border-gray-700 last:border-0"
              onclick={() => navigateTo(result)}
            >
              <div class="flex items-center justify-between gap-2">
                <span class="text-xs font-medium text-blue-400 shrink-0">
                  {typeLabel[result.result_type] ?? result.result_type}
                </span>
                <span class="text-xs text-gray-500 shrink-0">{result.patient_name}</span>
              </div>
              <div class="text-sm text-gray-100 mt-0.5 truncate">{result.title}</div>
              {#if result.snippet}
                <div class="text-xs text-gray-400 mt-0.5 line-clamp-1">{@html result.snippet}</div>
              {/if}
            </button>
          {/each}
        {/if}
      </div>
    {/if}
  </div>

  <div class="flex items-center gap-2">
    <span class="text-sm text-gray-400">LLM:</span>
    <div
      class="w-3 h-3 rounded-full {llmStatus === 'loaded' ? 'bg-green-500' : 'bg-red-500'}"
      title={llmStatus === 'loaded' ? 'Model loaded' : 'No model'}
    ></div>
  </div>
</header>
