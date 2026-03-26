<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import {
    getEngineStatus,
    loadModel,
    globalSearch,
    parseError,
    type LlmEngineStatus,
    type SearchResult,
  } from '$lib/api';
  import { t } from '$lib/translations';

  let searchInput = $state<HTMLInputElement | null>(null);
  let engineStatus = $state<LlmEngineStatus | null>(null);
  let isLoadingModel = $state(false);
  let searchQuery = $state('');
  let searchResults = $state<SearchResult[]>([]);
  let showDropdown = $state(false);
  let isSearching = $state(false);
  let searchTimeout: ReturnType<typeof setTimeout> | null = null;

  let isLoaded = $derived(engineStatus?.is_loaded ?? false);
  let isDownloaded = $derived(engineStatus?.is_downloaded ?? false);

  onMount(() => {
    const handleKeydown = (e: KeyboardEvent) => {
      // Cmd+K is now handled globally for command palette
      // Only handle Escape here
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
      engineStatus = await getEngineStatus();
    } catch (error) {
      console.error('Failed to get LLM status:', error);
    }
  }

  async function handleDotClick() {
    if (isLoaded || isLoadingModel) return;
    if (isDownloaded && engineStatus?.downloaded_filename) {
      isLoadingModel = true;
      try {
        await loadModel(engineStatus.downloaded_filename);
        engineStatus = await getEngineStatus();
      } catch (e) {
        console.error('Failed to load model:', parseError(e).message);
      } finally {
        isLoadingModel = false;
      }
    } else {
      goto('/settings');
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

  let typeLabel = $derived<Record<string, string>>({
    patient: $t('topbar.typePatient'),
    file: $t('topbar.typeFile'),
    session: $t('topbar.typeSession'),
    diagnosis: $t('topbar.typeDiagnosis'),
    medication: $t('topbar.typeMedication'),
    report: $t('topbar.typeReport'),
  });
</script>

<header
  class="h-16 bg-gray-50 dark:bg-gray-900 border-b border-gray-200 dark:border-gray-800 flex items-center px-6 gap-4"
>
  <div class="flex-1 max-w-2xl relative">
    <input
      bind:this={searchInput}
      type="text"
      placeholder={$t('topbar.searchPlaceholder')}
      class="w-full px-4 py-2 bg-white dark:bg-gray-800 border border-gray-300 dark:border-gray-700 rounded-lg text-gray-900 dark:text-gray-100 placeholder-gray-400 focus:outline-none focus:ring-2 focus:ring-blue-500"
      value={searchQuery}
      oninput={handleSearch}
      onblur={handleBlur}
    />

    {#if showDropdown && searchQuery.trim()}
      <div
        class="absolute top-full left-0 right-0 mt-1 bg-white dark:bg-gray-800 border border-gray-200 dark:border-gray-700 rounded-lg shadow-xl z-50 max-h-96 overflow-y-auto"
      >
        {#if isSearching}
          <div class="px-4 py-3 text-sm text-gray-500 dark:text-gray-400">
            {$t('topbar.searching')}
          </div>
        {:else if searchResults.length === 0}
          <div class="px-4 py-3 text-sm text-gray-500 dark:text-gray-400">
            {$t('topbar.noResults').replace('{query}', searchQuery)}
          </div>
        {:else}
          {#each searchResults as result (result.entity_id)}
            <button
              class="w-full text-left px-4 py-3 hover:bg-gray-100 dark:hover:bg-gray-700 transition-colors border-b border-gray-200 dark:border-gray-700 last:border-0"
              onclick={() => navigateTo(result)}
            >
              <div class="flex items-center justify-between gap-2">
                <span class="text-xs font-medium text-blue-600 dark:text-blue-400 shrink-0">
                  {typeLabel[result.result_type] ?? result.result_type}
                </span>
                <span class="text-xs text-gray-400 dark:text-gray-500 shrink-0"
                  >{result.patient_name}</span
                >
              </div>
              <div class="text-sm text-gray-900 dark:text-gray-100 mt-0.5 truncate">
                {result.title}
              </div>
              {#if result.snippet}
                <div class="text-xs text-gray-500 dark:text-gray-400 mt-0.5 line-clamp-1">
                  {result.snippet}
                </div>
              {/if}
            </button>
          {/each}
        {/if}
      </div>
    {/if}
  </div>

  <div class="flex items-center gap-2">
    <span class="text-sm text-gray-500 dark:text-gray-400">LLM:</span>
    <button
      onclick={handleDotClick}
      disabled={isLoaded || isLoadingModel}
      class="w-3 h-3 rounded-full transition-colors focus:outline-none focus:ring-2 focus:ring-offset-1 focus:ring-blue-500 {isLoadingModel
        ? 'bg-amber-400 animate-pulse cursor-wait'
        : isLoaded
          ? 'bg-green-500 cursor-default'
          : isDownloaded
            ? 'bg-amber-400 cursor-pointer hover:bg-amber-300'
            : 'bg-red-500 cursor-pointer hover:bg-red-400'}"
      aria-label={isLoadingModel
        ? $t('topbar.loadingModel')
        : isLoaded
          ? $t('topbar.modelLoaded')
          : isDownloaded
            ? $t('topbar.modelDownloaded')
            : $t('topbar.noModelDownloaded')}
      title={isLoadingModel
        ? $t('topbar.loadingModel')
        : isLoaded
          ? $t('topbar.modelLoaded')
          : isDownloaded
            ? $t('topbar.modelDownloaded')
            : $t('topbar.noModelDownloaded')}
    ></button>
  </div>
</header>
