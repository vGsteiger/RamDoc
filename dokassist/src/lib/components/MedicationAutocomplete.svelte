<script lang="ts">
  import { t } from '$lib/translations';
  import { invoke } from '@tauri-apps/api/core';
  import { onDestroy } from 'svelte';
  import type { SubstanceSummary } from '$lib/api';
  import { Input } from '$lib/components/ui';

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
    placeholder = '',
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
  <Input
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
    aria-activedescendant={showDropdown && suggestions[selectedIndex]
      ? `${id}-option-${selectedIndex}`
      : undefined}
    aria-autocomplete="list"
  />

  {#if showDropdown && suggestions.length > 0}
    <div
      id="{id}-listbox"
      role="listbox"
      aria-label={$t('medications.suggestions')}
      class="absolute z-20 mt-1 max-h-64 w-full overflow-y-auto rounded-card border border-line bg-surface-overlay p-1 shadow-popover"
    >
      {#each suggestions as entry, index (`${entry.id}-${index}`)}
        <button
          type="button"
          id="{id}-option-{index}"
          role="option"
          aria-selected={index === selectedIndex}
          class="flex w-full items-start gap-2 rounded-control px-2 py-1.5 text-left transition-colors duration-150 ease-standard"
          class:bg-surface-selected={index === selectedIndex}
          onmouseenter={() => (selectedIndex = index)}
          onclick={() => handleSelect(entry)}
        >
          <div class="flex-1 min-w-0">
            <span class="text-body text-fg font-medium">{entry.name_de}</span>
            {#if entry.trade_names.length > 0}
              <span class="ml-2 text-caption text-fg-muted truncate"
                >{entry.trade_names.slice(0, 2).join(', ')}</span
              >
            {/if}
          </div>
          {#if entry.atc_code}
            <span
              class="shrink-0 rounded-control border border-line-subtle px-1.5 font-mono text-caption text-fg-muted"
            >
              {entry.atc_code}
            </span>
          {/if}
        </button>
      {/each}
    </div>
  {/if}
</div>
