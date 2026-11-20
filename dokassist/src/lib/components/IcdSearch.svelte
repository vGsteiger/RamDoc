<script lang="ts">
  import { onMount } from 'svelte';
  import { Input } from '$lib/components/ui';
  import { t } from '$lib/translations';

  interface ICD10Entry {
    code: string;
    description: string;
  }

  interface Props {
    onSelect: (code: string, description: string) => void;
    placeholder?: string;
    id?: string;
  }

  let { onSelect, placeholder = undefined, id = undefined }: Props = $props();
  const listId = $props.id();

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
        entry.code.toLowerCase().includes(query) || entry.description.toLowerCase().includes(query)
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
  <Input
    {id}
    type="text"
    role="combobox"
    autocomplete="off"
    aria-autocomplete="list"
    aria-expanded={showDropdown && filteredResults.length > 0}
    aria-controls={listId}
    aria-activedescendant={showDropdown && filteredResults.length > 0
      ? `${listId}-${selectedIndex}`
      : undefined}
    bind:value={searchQuery}
    onkeydown={handleKeydown}
    onblur={handleBlur}
    onfocus={() => {
      if (filteredResults.length > 0) showDropdown = true;
    }}
    placeholder={placeholder ?? $t('diagnoses.search')}
  />

  {#if showDropdown && filteredResults.length > 0}
    <div
      id={listId}
      role="listbox"
      class="absolute z-10 mt-1 max-h-64 w-full overflow-y-auto rounded-card border border-line bg-surface-overlay p-1 shadow-popover"
    >
      {#each filteredResults as entry, index (`${entry.code}-${index}`)}
        <button
          type="button"
          role="option"
          id="{listId}-{index}"
          aria-selected={index === selectedIndex}
          tabindex="-1"
          class="flex h-control w-full items-center gap-2 rounded-control px-2 text-left transition-colors duration-150 ease-standard hover:bg-surface-hover {index ===
          selectedIndex
            ? 'bg-surface-selected'
            : ''}"
          onmousedown={(e) => e.preventDefault()}
          onclick={() => handleSelect(entry)}
        >
          <span class="w-14 shrink-0 font-mono text-label text-fg">{entry.code}</span>
          <span class="min-w-0 truncate text-label text-fg-muted">{entry.description}</span>
        </button>
      {/each}
    </div>
  {/if}
</div>
