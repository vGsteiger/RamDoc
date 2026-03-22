<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { globalSearch, type SearchResult, createSession, type Patient } from '$lib/api';
  import { t } from '$lib/translations';
  import { lockApp } from '$lib/api';
  import { authStatus } from '$lib/stores/auth';
  import { themePreference } from '$lib/stores/theme';
  import { language } from '$lib/stores/language';
  import {
    Search,
    UserPlus,
    Calendar,
    LayoutDashboard,
    Users,
    BookOpen,
    MessageSquare,
    Settings,
    Lock,
    Moon,
    Sun,
    Monitor,
    Globe,
  } from 'lucide-svelte';

  interface Props {
    isOpen: boolean;
    onClose: () => void;
    patients?: Patient[];
  }

  let { isOpen = $bindable(), onClose, patients = [] }: Props = $props();

  interface Action {
    id: string;
    label: string;
    category: 'navigation' | 'action' | 'settings';
    icon: typeof Search;
    handler: () => void | Promise<void>;
    shortcut?: string;
  }

  let searchQuery = $state('');
  let selectedIndex = $state(0);
  let isSearching = $state(false);
  let searchResults = $state<SearchResult[]>([]);
  let inputRef = $state<HTMLInputElement | null>(null);
  let searchTimeout: ReturnType<typeof setTimeout> | null = null;

  // Define available actions
  let actions = $derived<Action[]>([
    {
      id: 'nav-dashboard',
      label: $t('commandPalette.navDashboard'),
      category: 'navigation',
      icon: LayoutDashboard,
      handler: () => {
        goto('/dashboard');
        onClose();
      },
    },
    {
      id: 'nav-patients',
      label: $t('commandPalette.navPatients'),
      category: 'navigation',
      icon: Users,
      handler: () => {
        goto('/patients');
        onClose();
      },
    },
    {
      id: 'nav-calendar',
      label: $t('commandPalette.navCalendar'),
      category: 'navigation',
      icon: Calendar,
      handler: () => {
        goto('/calendar');
        onClose();
      },
    },
    {
      id: 'nav-literature',
      label: $t('commandPalette.navLiterature'),
      category: 'navigation',
      icon: BookOpen,
      handler: () => {
        goto('/literature');
        onClose();
      },
    },
    {
      id: 'nav-chat',
      label: $t('commandPalette.navChat'),
      category: 'navigation',
      icon: MessageSquare,
      handler: () => {
        goto('/chat');
        onClose();
      },
    },
    {
      id: 'nav-settings',
      label: $t('commandPalette.navSettings'),
      category: 'navigation',
      icon: Settings,
      handler: () => {
        goto('/settings');
        onClose();
      },
    },
    {
      id: 'action-new-patient',
      label: $t('commandPalette.newPatient'),
      category: 'action',
      icon: UserPlus,
      handler: () => {
        goto('/patients/new');
        onClose();
      },
      shortcut: 'Cmd+N',
    },
    {
      id: 'action-lock',
      label: $t('commandPalette.lockApp'),
      category: 'action',
      icon: Lock,
      handler: async () => {
        await lockApp();
        authStatus.set('locked');
        goto('/unlock');
        onClose();
      },
    },
    {
      id: 'settings-theme-light',
      label: $t('commandPalette.themeLight'),
      category: 'settings',
      icon: Sun,
      handler: () => {
        themePreference.set('light');
        onClose();
      },
    },
    {
      id: 'settings-theme-dark',
      label: $t('commandPalette.themeDark'),
      category: 'settings',
      icon: Moon,
      handler: () => {
        themePreference.set('dark');
        onClose();
      },
    },
    {
      id: 'settings-theme-system',
      label: $t('commandPalette.themeSystem'),
      category: 'settings',
      icon: Monitor,
      handler: () => {
        themePreference.set('system');
        onClose();
      },
    },
    {
      id: 'settings-lang-en',
      label: $t('commandPalette.langEnglish'),
      category: 'settings',
      icon: Globe,
      handler: () => {
        language.set('en');
        onClose();
      },
    },
    {
      id: 'settings-lang-de',
      label: $t('commandPalette.langGerman'),
      category: 'settings',
      icon: Globe,
      handler: () => {
        language.set('de');
        onClose();
      },
    },
  ]);

  // Filter actions and search results based on query
  let filteredItems = $derived(() => {
    const query = searchQuery.toLowerCase().trim();

    if (!query) {
      // Show all actions when no query
      return { actions, patients: [], searchResults: [] };
    }

    // Filter actions by label
    const filteredActions = actions.filter((action) =>
      action.label.toLowerCase().includes(query)
    );

    // Filter patients by name (client-side)
    const filteredPatients = patients.filter(
      (p) =>
        p.first_name.toLowerCase().includes(query) ||
        p.last_name.toLowerCase().includes(query) ||
        p.ahv_number.includes(query)
    );

    return {
      actions: filteredActions,
      patients: filteredPatients,
      searchResults,
    };
  });

  // Combined list of all items for keyboard navigation
  let allItems = $derived(() => {
    const items = filteredItems();
    const combined: Array<
      { type: 'action'; item: Action } | { type: 'patient'; item: Patient } | { type: 'search'; item: SearchResult }
    > = [];

    items.actions.forEach((action) => combined.push({ type: 'action', item: action }));
    items.patients.forEach((patient) => combined.push({ type: 'patient', item: patient }));
    items.searchResults.forEach((result) => combined.push({ type: 'search', item: result }));

    return combined;
  });

  // Handle search input with debouncing
  function handleSearchInput(e: Event) {
    const query = (e.target as HTMLInputElement).value;
    searchQuery = query;
    selectedIndex = 0;

    if (searchTimeout) clearTimeout(searchTimeout);

    if (!query.trim()) {
      searchResults = [];
      isSearching = false;
      return;
    }

    // Only trigger FTS5 search if query is long enough
    if (query.length >= 2) {
      searchTimeout = setTimeout(async () => {
        isSearching = true;
        try {
          searchResults = await globalSearch(query, 10);
        } catch (err) {
          console.error('Search error:', err);
          searchResults = [];
        } finally {
          isSearching = false;
        }
      }, 300);
    }
  }

  // Handle keyboard navigation
  function handleKeydown(e: KeyboardEvent) {
    const items = allItems();

    switch (e.key) {
      case 'ArrowDown':
        e.preventDefault();
        selectedIndex = Math.min(selectedIndex + 1, items.length - 1);
        break;
      case 'ArrowUp':
        e.preventDefault();
        selectedIndex = Math.max(selectedIndex - 1, 0);
        break;
      case 'Enter':
        e.preventDefault();
        if (items[selectedIndex]) {
          executeItem(items[selectedIndex]);
        }
        break;
      case 'Escape':
        e.preventDefault();
        onClose();
        break;
    }
  }

  // Execute the selected item
  function executeItem(
    item:
      | { type: 'action'; item: Action }
      | { type: 'patient'; item: Patient }
      | { type: 'search'; item: SearchResult }
  ) {
    if (item.type === 'action') {
      item.item.handler();
    } else if (item.type === 'patient') {
      goto(`/patients/${item.item.id}`);
      onClose();
    } else if (item.type === 'search') {
      navigateToSearchResult(item.item);
    }
  }

  // Navigate to search result
  function navigateToSearchResult(result: SearchResult) {
    onClose();
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

  // Focus input when opened
  $effect(() => {
    if (isOpen) {
      setTimeout(() => inputRef?.focus(), 50);
      searchQuery = '';
      selectedIndex = 0;
      searchResults = [];
    }
  });

  let categoryLabel = $derived<Record<string, string>>({
    navigation: $t('commandPalette.categoryNavigation'),
    action: $t('commandPalette.categoryAction'),
    settings: $t('commandPalette.categorySettings'),
  });

  let typeLabel = $derived<Record<string, string>>({
    patient: $t('commandPalette.typePatient'),
    file: $t('commandPalette.typeFile'),
    session: $t('commandPalette.typeSession'),
    diagnosis: $t('commandPalette.typeDiagnosis'),
    medication: $t('commandPalette.typeMedication'),
    report: $t('commandPalette.typeReport'),
  });
</script>

{#if isOpen}
  <!-- Backdrop -->
  <div
    class="fixed inset-0 bg-black/50 z-50 flex items-start justify-center pt-32"
    onclick={onClose}
    role="presentation"
  >
    <!-- Command Palette Modal -->
    <div
      class="bg-white dark:bg-gray-800 rounded-xl shadow-2xl w-full max-w-2xl mx-4 overflow-hidden"
      onclick={(e) => e.stopPropagation()}
      role="dialog"
      aria-modal="true"
      aria-label={$t('commandPalette.title')}
    >
      <!-- Search Input -->
      <div class="flex items-center gap-3 px-4 py-3 border-b border-gray-200 dark:border-gray-700">
        <Search class="text-gray-400 shrink-0" size={20} />
        <input
          bind:this={inputRef}
          type="text"
          placeholder={$t('commandPalette.placeholder')}
          class="flex-1 bg-transparent text-gray-900 dark:text-gray-100 placeholder-gray-400 focus:outline-none"
          value={searchQuery}
          oninput={handleSearchInput}
          onkeydown={handleKeydown}
        />
        <kbd
          class="px-2 py-1 text-xs font-semibold text-gray-500 dark:text-gray-400 bg-gray-100 dark:bg-gray-700 border border-gray-300 dark:border-gray-600 rounded"
        >
          ESC
        </kbd>
      </div>

      <!-- Results -->
      <div class="max-h-96 overflow-y-auto">
        {#if isSearching}
          <div class="px-4 py-8 text-center text-sm text-gray-500 dark:text-gray-400">
            {$t('commandPalette.searching')}
          </div>
        {:else if allItems().length === 0}
          <div class="px-4 py-8 text-center text-sm text-gray-500 dark:text-gray-400">
            {searchQuery.trim() ? $t('commandPalette.noResults') : $t('commandPalette.noQuery')}
          </div>
        {:else}
          <!-- Actions -->
          {#if filteredItems().actions.length > 0}
            <div class="py-2">
              <div class="px-4 py-2 text-xs font-semibold text-gray-500 dark:text-gray-400 uppercase">
                {$t('commandPalette.categoryActions')}
              </div>
              {#each filteredItems().actions as action, index}
                {@const Icon = action.icon}
                {@const globalIndex = index}
                <button
                  type="button"
                  class="w-full flex items-center gap-3 px-4 py-2 hover:bg-gray-100 dark:hover:bg-gray-700 transition-colors"
                  class:bg-gray-100={selectedIndex === globalIndex}
                  class:dark:bg-gray-700={selectedIndex === globalIndex}
                  onclick={() => executeItem({ type: 'action', item: action })}
                >
                  <Icon size={18} class="text-gray-500 dark:text-gray-400 shrink-0" />
                  <span class="flex-1 text-left text-sm text-gray-900 dark:text-gray-100">
                    {action.label}
                  </span>
                  {#if action.shortcut}
                    <kbd
                      class="px-2 py-1 text-xs font-semibold text-gray-500 dark:text-gray-400 bg-gray-100 dark:bg-gray-700 border border-gray-300 dark:border-gray-600 rounded shrink-0"
                    >
                      {action.shortcut}
                    </kbd>
                  {/if}
                  <span class="text-xs text-gray-400 dark:text-gray-500 shrink-0">
                    {categoryLabel[action.category]}
                  </span>
                </button>
              {/each}
            </div>
          {/if}

          <!-- Patients -->
          {#if filteredItems().patients.length > 0}
            <div class="py-2 border-t border-gray-200 dark:border-gray-700">
              <div class="px-4 py-2 text-xs font-semibold text-gray-500 dark:text-gray-400 uppercase">
                {$t('commandPalette.categoryPatients')}
              </div>
              {#each filteredItems().patients as patient, index}
                {@const globalIndex = filteredItems().actions.length + index}
                <button
                  type="button"
                  class="w-full flex items-center gap-3 px-4 py-2 hover:bg-gray-100 dark:hover:bg-gray-700 transition-colors"
                  class:bg-gray-100={selectedIndex === globalIndex}
                  class:dark:bg-gray-700={selectedIndex === globalIndex}
                  onclick={() => executeItem({ type: 'patient', item: patient })}
                >
                  <Users size={18} class="text-gray-500 dark:text-gray-400 shrink-0" />
                  <span class="flex-1 text-left text-sm text-gray-900 dark:text-gray-100">
                    {patient.first_name}
                    {patient.last_name}
                  </span>
                  <span class="text-xs text-gray-400 dark:text-gray-500 shrink-0">
                    {patient.ahv_number}
                  </span>
                </button>
              {/each}
            </div>
          {/if}

          <!-- Search Results -->
          {#if searchResults.length > 0}
            <div class="py-2 border-t border-gray-200 dark:border-gray-700">
              <div class="px-4 py-2 text-xs font-semibold text-gray-500 dark:text-gray-400 uppercase">
                {$t('commandPalette.categorySearchResults')}
              </div>
              {#each searchResults as result, index}
                {@const globalIndex = filteredItems().actions.length + filteredItems().patients.length + index}
                <button
                  type="button"
                  class="w-full text-left px-4 py-2 hover:bg-gray-100 dark:hover:bg-gray-700 transition-colors"
                  class:bg-gray-100={selectedIndex === globalIndex}
                  class:dark:bg-gray-700={selectedIndex === globalIndex}
                  onclick={() => executeItem({ type: 'search', item: result })}
                >
                  <div class="flex items-center justify-between gap-2 mb-1">
                    <span class="text-xs font-medium text-blue-600 dark:text-blue-400 shrink-0">
                      {typeLabel[result.result_type] ?? result.result_type}
                    </span>
                    <span class="text-xs text-gray-400 dark:text-gray-500 shrink-0">
                      {result.patient_name}
                    </span>
                  </div>
                  <div class="text-sm text-gray-900 dark:text-gray-100 truncate">
                    {result.title}
                  </div>
                  {#if result.snippet}
                    <div class="text-xs text-gray-500 dark:text-gray-400 line-clamp-1 mt-0.5">
                      {@html result.snippet}
                    </div>
                  {/if}
                </button>
              {/each}
            </div>
          {/if}
        {/if}
      </div>

      <!-- Footer -->
      <div
        class="px-4 py-2 border-t border-gray-200 dark:border-gray-700 bg-gray-50 dark:bg-gray-900 flex items-center justify-between text-xs text-gray-500 dark:text-gray-400"
      >
        <div class="flex items-center gap-4">
          <div class="flex items-center gap-1">
            <kbd
              class="px-2 py-1 text-xs font-semibold bg-white dark:bg-gray-800 border border-gray-300 dark:border-gray-600 rounded"
            >
              ↑↓
            </kbd>
            <span>{$t('commandPalette.navigate')}</span>
          </div>
          <div class="flex items-center gap-1">
            <kbd
              class="px-2 py-1 text-xs font-semibold bg-white dark:bg-gray-800 border border-gray-300 dark:border-gray-600 rounded"
            >
              ↵
            </kbd>
            <span>{$t('commandPalette.select')}</span>
          </div>
        </div>
        <div class="flex items-center gap-1">
          <span>{$t('commandPalette.openWith')}</span>
          <kbd
            class="px-2 py-1 text-xs font-semibold bg-white dark:bg-gray-800 border border-gray-300 dark:border-gray-600 rounded"
          >
            Cmd+K
          </kbd>
        </div>
      </div>
    </div>
  </div>
{/if}
