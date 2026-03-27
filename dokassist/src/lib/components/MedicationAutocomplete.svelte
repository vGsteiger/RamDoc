<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { onDestroy } from 'svelte';
  import type { SubstanceSummary } from '$lib/api';

  interface Props {
    value: string;
    onInput: (value: string) => void;
    onSelect: (summary: SubstanceSummary) => void;
    placeholder?: string;
    required?: boolean;
    id?: string;
  }

  let {
    value,
    onInput,
    onSelect,
    placeholder = 'z.B. Sertralin',
    required = false,
    id = 'substance',
  }: Props = $props();

  let suggestions = $state<SubstanceSummary[]>([]);
  let showDropdown = $state(false);
  let selectedIndex = $state(0);
  let debounceTimer: ReturnType<typeof setTimeout> | null = null;

  onDestroy(() => {
    if (debounceTimer) {
      clearTimeout(debounceTimer);
      debounceTimer = null;
    }
  });

  function handleInput(event: Event) {
    const input = (event.target as HTMLInputElement).value;
    onInput(input);

    if (debounceTimer) clearTimeout(debounceTimer);

    if (!input.trim()) {
      suggestions = [];
      showDropdown = false;
      return;
    }

    debounceTimer = setTimeout(async () => {
      try {
        const results = await invoke<SubstanceSummary[]>('search_medication_reference', {
          query: input,
        });
        suggestions = results;
        showDropdown = results.length > 0;
        selectedIndex = 0;
      } catch {
        // Graceful degradation: ref DB not installed or search failed
        suggestions = [];
        showDropdown = false;
      }
    }, 300);
  }

  function handleSelect(entry: SubstanceSummary) {
    onInput(entry.name_de);
    onSelect(entry);
    suggestions = [];
    showDropdown = false;
  }

  function handleKeydown(event: KeyboardEvent) {
    if (!showDropdown) return;

    switch (event.key) {
      case 'ArrowDown':
        event.preventDefault();
        selectedIndex = Math.min(selectedIndex + 1, suggestions.length - 1);
        break;
      case 'ArrowUp':
        event.preventDefault();
        selectedIndex = Math.max(selectedIndex - 1, 0);
        break;
      case 'Enter':
        event.preventDefault();
        if (suggestions[selectedIndex]) {
          handleSelect(suggestions[selectedIndex]);
        }
        break;
      case 'Escape':
        event.preventDefault();
        showDropdown = false;
        break;
    }
  }

  function handleBlur() {
    setTimeout(() => {
      showDropdown = false;
    }, 200);
  }
</script>

<div class="relative">
  <input
    {id}
    type="text"
    {value}
    oninput={handleInput}
    onkeydown={handleKeydown}
    onblur={handleBlur}
    onfocus={() => {
      if (suggestions.length > 0) showDropdown = true;
    }}
    {required}
    {placeholder}
    autocomplete="off"
    role="combobox"
    aria-expanded={showDropdown}
    aria-controls="{id}-listbox"
    aria-activedescendant={showDropdown && suggestions[selectedIndex] ? `${id}-option-${selectedIndex}` : undefined}
    aria-autocomplete="list"
    class="w-full px-3 py-2 bg-gray-700 border border-gray-600 rounded-lg text-gray-100 placeholder-gray-400 focus:outline-none focus:ring-2 focus:ring-blue-500"
  />

  {#if showDropdown && suggestions.length > 0}
    <div
      id="{id}-listbox"
      role="listbox"
      aria-label="Medication suggestions"
      class="absolute z-20 w-full mt-1 bg-gray-800 border border-gray-600 rounded-lg shadow-xl max-h-64 overflow-y-auto"
    >
      {#each suggestions as entry, index}
        <button
          type="button"
          id="{id}-option-{index}"
          role="option"
          aria-selected={index === selectedIndex}
          class="w-full px-3 py-2 text-left transition-colors flex items-start gap-2"
          class:bg-gray-700={index === selectedIndex}
          onmouseenter={() => (selectedIndex = index)}
          onclick={() => handleSelect(entry)}
        >
          <div class="flex-1 min-w-0">
            <span class="text-sm text-gray-100 font-medium">{entry.name_de}</span>
            {#if entry.trade_names.length > 0}
              <span class="ml-2 text-xs text-gray-400 truncate"
                >{entry.trade_names.slice(0, 2).join(', ')}</span
              >
            {/if}
          </div>
          {#if entry.atc_code}
            <span
              class="shrink-0 text-xs font-mono bg-blue-900 text-blue-300 px-1.5 py-0.5 rounded"
            >
              {entry.atc_code}
            </span>
          {/if}
        </button>
      {/each}
    </div>
  {/if}
</div>
