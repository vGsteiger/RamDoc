<script lang="ts">
  import { onMount } from 'svelte';

  interface ICD10Entry {
    code: string;
    description: string;
  }

  interface Props {
    onSelect: (code: string, description: string) => void;
    placeholder?: string;
  }

  let { onSelect, placeholder = 'ICD-10 Code suchen...' }: Props = $props();

  let icd10Data: ICD10Entry[] = $state([]);
  let searchQuery = $state('');
  let filteredResults = $state<ICD10Entry[]>([]);
  let showDropdown = $state(false);
  let selectedIndex = $state(0);

  onMount(async () => {
    try {
      const response = await fetch('/icd10gm.json');
      icd10Data = await response.json();
    } catch (error) {
      console.error('Failed to load ICD-10 data:', error);
    }
  });

  $effect(() => {
    if (!searchQuery.trim()) {
      filteredResults = [];
      showDropdown = false;
      return;
    }

    const query = searchQuery.toLowerCase();
    const results = icd10Data.filter(
      (entry) =>
        entry.code.toLowerCase().includes(query) ||
        entry.description.toLowerCase().includes(query)
    );

    filteredResults = results.slice(0, 20); // Limit to 20 results
    showDropdown = results.length > 0;
    selectedIndex = 0;
  });

  function handleSelect(entry: ICD10Entry) {
    onSelect(entry.code, entry.description);
    searchQuery = '';
    showDropdown = false;
  }

  function handleKeydown(event: KeyboardEvent) {
    if (!showDropdown) return;

    switch (event.key) {
      case 'ArrowDown':
        event.preventDefault();
        selectedIndex = Math.min(selectedIndex + 1, filteredResults.length - 1);
        break;
      case 'ArrowUp':
        event.preventDefault();
        selectedIndex = Math.max(selectedIndex - 1, 0);
        break;
      case 'Enter':
        event.preventDefault();
        if (filteredResults[selectedIndex]) {
          handleSelect(filteredResults[selectedIndex]);
        }
        break;
      case 'Escape':
        event.preventDefault();
        showDropdown = false;
        break;
    }
  }

  function handleBlur() {
    // Delay to allow click events on dropdown items
    setTimeout(() => {
      showDropdown = false;
    }, 200);
  }
</script>

<div class="relative">
  <input
    type="text"
    bind:value={searchQuery}
    onkeydown={handleKeydown}
    onblur={handleBlur}
    onfocus={() => {
      if (filteredResults.length > 0) showDropdown = true;
    }}
    {placeholder}
    class="w-full px-4 py-2 bg-gray-700 border border-gray-600 rounded-lg text-gray-100 placeholder-gray-400 focus:outline-none focus:ring-2 focus:ring-blue-500"
  />

  {#if showDropdown && filteredResults.length > 0}
    <div
      class="absolute z-10 w-full mt-1 bg-gray-800 border border-gray-600 rounded-lg shadow-lg max-h-64 overflow-y-auto"
    >
      {#each filteredResults as entry, index}
        <button
          type="button"
          class="w-full px-4 py-2 text-left hover:bg-gray-700 transition-colors flex gap-2"
          class:bg-gray-700={index === selectedIndex}
          onclick={() => handleSelect(entry)}
        >
          <span class="font-mono text-sm text-blue-400 shrink-0">{entry.code}</span>
          <span class="text-sm text-gray-300">{entry.description}</span>
        </button>
      {/each}
    </div>
  {/if}
</div>
