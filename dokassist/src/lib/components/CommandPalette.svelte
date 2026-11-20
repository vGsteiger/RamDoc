<script lang="ts">
  import { goto } from '$app/navigation';
  import {
    globalSearch,
    listPatients,
    lockApp,
    type ChatSession,
    type Patient,
    type SearchResult,
  } from '$lib/api';
  import { t } from '$lib/translations';
  import { fmt } from '$lib/format';
  import { authStatus } from '$lib/stores/auth';
  import { themePreference } from '$lib/stores/theme';
  import { language } from '$lib/stores/language';
  import { chatSessions, refreshChatSessions } from '$lib/stores/chat-sessions';
  import { chatSessionHref, NEW_CHAT_URL } from '$lib/shell';
  import { HighlightedSnippet, Kbd } from '$lib/components/ui';
  import {
    BookOpen,
    Calendar,
    FileText,
    Globe,
    Lock,
    MessageSquare,
    Monitor,
    Moon,
    Search,
    Settings,
    SquarePen,
    Sun,
    User,
    UserPlus,
    Users,
  } from 'lucide-svelte';

  /*
   * Cmd+K: one place for search and commands (D2 removed the top bar).
   * Groups: actions, navigation, recent chats, patients, record search
   * (FTS via globalSearch) and preferences. The input is an ARIA combobox
   * driving a listbox; arrow keys move the active option, Enter runs it,
   * Esc closes. Record snippets render through HighlightedSnippet, so the
   * backend's <mark> tags highlight instead of being printed or stripped.
   */
  interface Props {
    isOpen: boolean;
    onClose: () => void;
    /** Optional preloaded patients; otherwise loaded when the palette opens. */
    patients?: Patient[];
  }

  let { isOpen = $bindable(), onClose, patients = undefined }: Props = $props();

  type Group = 'actions' | 'navigation' | 'chats' | 'patients' | 'records' | 'preferences';

  interface Item {
    id: string;
    group: Group;
    label: string;
    secondary?: string;
    icon: typeof Search;
    shortcut?: string;
    result?: SearchResult;
    run: () => void | Promise<void>;
  }

  const listId = $props.id();
  let searchQuery = $state('');
  let selectedIndex = $state(0);
  let isSearching = $state(false);
  let searchResults = $state<SearchResult[]>([]);
  let loadedPatients = $state<Patient[]>([]);
  let inputRef = $state<HTMLInputElement | null>(null);
  let listRef = $state<HTMLElement | null>(null);
  let searchTimeout: ReturnType<typeof setTimeout> | null = null;
  let searchVersion = 0;

  let allPatients = $derived(patients ?? loadedPatients);

  function go(path: string) {
    onClose();
    void goto(path);
  }

  function patientName(p: Patient) {
    return `${p.first_name} ${p.last_name}`.trim();
  }

  function chatSecondary(session: ChatSession) {
    const date = $fmt.relativeDate(session.updated_at);
    const name = session.patient_id ? $chatSessions.patientNames[session.patient_id] : undefined;
    return name ? `${name} · ${date}` : date;
  }

  function resultPath(result: SearchResult): string {
    switch (result.result_type) {
      case 'patient':
        return `/patients/${result.entity_id}`;
      case 'file':
        return `/patients/${result.patient_id}/files`;
      case 'session':
        return `/patients/${result.patient_id}/sessions/${result.entity_id}`;
      case 'diagnosis':
        return `/patients/${result.patient_id}/diagnoses`;
      case 'medication':
        return `/patients/${result.patient_id}/medications`;
      case 'report':
        return `/patients/${result.patient_id}/reports/${result.entity_id}`;
      default:
        return `/patients/${result.patient_id}`;
    }
  }

  let typeLabel = $derived<Record<string, string>>({
    patient: $t('commandPalette.typePatient'),
    file: $t('commandPalette.typeFile'),
    session: $t('commandPalette.typeSession'),
    diagnosis: $t('commandPalette.typeDiagnosis'),
    medication: $t('commandPalette.typeMedication'),
    report: $t('commandPalette.typeReport'),
  });

  let commands = $derived<Item[]>([
    {
      id: 'action-new-chat',
      group: 'actions',
      label: $t('commandPalette.newChat'),
      icon: SquarePen,
      shortcut: '⌘N',
      run: () => go(NEW_CHAT_URL),
    },
    {
      id: 'action-new-patient',
      group: 'actions',
      label: $t('commandPalette.newPatient'),
      icon: UserPlus,
      shortcut: '⇧⌘N',
      run: () => go('/patients/new'),
    },
    {
      id: 'action-lock',
      group: 'actions',
      label: $t('commandPalette.lockApp'),
      icon: Lock,
      run: async () => {
        await lockApp();
        authStatus.set('locked');
        go('/unlock');
      },
    },
    {
      id: 'nav-chat',
      group: 'navigation',
      label: $t('commandPalette.navChat'),
      icon: MessageSquare,
      run: () => go('/chat'),
    },
    {
      id: 'nav-patients',
      group: 'navigation',
      label: $t('commandPalette.navPatients'),
      icon: Users,
      run: () => go('/patients'),
    },
    {
      id: 'nav-calendar',
      group: 'navigation',
      label: $t('commandPalette.navCalendar'),
      icon: Calendar,
      run: () => go('/calendar'),
    },
    {
      id: 'nav-reference',
      group: 'navigation',
      label: $t('commandPalette.navReference'),
      icon: BookOpen,
      run: () => go('/reference'),
    },
    {
      id: 'nav-settings',
      group: 'navigation',
      label: $t('commandPalette.navSettings'),
      icon: Settings,
      run: () => go('/settings'),
    },
    {
      id: 'pref-theme-light',
      group: 'preferences',
      label: $t('commandPalette.themeLight'),
      icon: Sun,
      run: () => {
        themePreference.set('light');
        onClose();
      },
    },
    {
      id: 'pref-theme-dark',
      group: 'preferences',
      label: $t('commandPalette.themeDark'),
      icon: Moon,
      run: () => {
        themePreference.set('dark');
        onClose();
      },
    },
    {
      id: 'pref-theme-system',
      group: 'preferences',
      label: $t('commandPalette.themeSystem'),
      icon: Monitor,
      run: () => {
        themePreference.set('system');
        onClose();
      },
    },
    {
      id: 'pref-lang-en',
      group: 'preferences',
      label: $t('commandPalette.langEnglish'),
      icon: Globe,
      run: () => {
        language.set('en');
        onClose();
      },
    },
    {
      id: 'pref-lang-de',
      group: 'preferences',
      label: $t('commandPalette.langGerman'),
      icon: Globe,
      run: () => {
        language.set('de');
        onClose();
      },
    },
  ]);

  const GROUP_ORDER: Group[] = [
    'actions',
    'navigation',
    'chats',
    'patients',
    'records',
    'preferences',
  ];

  let items = $derived.by<Item[]>(() => {
    const query = searchQuery.toLowerCase().trim();
    const matches = (text: string) => text.toLowerCase().includes(query);

    const commandItems = query ? commands.filter((c) => matches(c.label)) : commands;

    const chats = $chatSessions.sessions
      .filter(
        (s) =>
          !query ||
          matches(s.title) ||
          (s.patient_id && matches($chatSessions.patientNames[s.patient_id] ?? ''))
      )
      .slice(0, query ? 8 : 5)
      .map<Item>((s) => ({
        id: `chat-${s.id}`,
        group: 'chats',
        label: s.title.trim() || $t('nav.untitledChat'),
        secondary: chatSecondary(s),
        icon: MessageSquare,
        run: () => go(chatSessionHref(s)),
      }));

    const patientItems = query
      ? allPatients
          .filter(
            (p) => matches(patientName(p)) || (p.ahv_number ?? '').toLowerCase().includes(query)
          )
          .slice(0, 8)
          .map<Item>((p) => ({
            id: `patient-${p.id}`,
            group: 'patients',
            label: patientName(p),
            secondary: $fmt.date(p.date_of_birth),
            icon: User,
            run: () => go(`/patients/${p.id}`),
          }))
      : [];

    const recordItems = searchResults.map<Item>((r, i) => ({
      id: `record-${r.result_type}-${r.entity_id}-${i}`,
      group: 'records',
      label: r.title,
      icon: r.result_type === 'patient' ? User : FileText,
      result: r,
      run: () => go(resultPath(r)),
    }));

    const all = [...commandItems, ...chats, ...patientItems, ...recordItems];
    return GROUP_ORDER.flatMap((group) => all.filter((item) => item.group === group));
  });

  // Items grouped for rendering, each keeping its index in the flat list.
  let groups = $derived(
    GROUP_ORDER.map((group) => ({
      group,
      entries: items
        .map((item, index) => ({ item, index }))
        .filter(({ item }) => item.group === group),
    })).filter((g) => g.entries.length > 0)
  );

  let groupLabel = $derived<Record<Group, string>>({
    actions: $t('commandPalette.categoryActions'),
    navigation: $t('commandPalette.categoryNavigation'),
    chats: $t('commandPalette.categoryChats'),
    patients: $t('commandPalette.categoryPatients'),
    records: $t('commandPalette.categorySearchResults'),
    preferences: $t('commandPalette.categorySettings'),
  });

  // Keep the active option in range as the list changes.
  $effect(() => {
    if (selectedIndex >= items.length) selectedIndex = Math.max(0, items.length - 1);
  });

  function optionId(index: number) {
    return `${listId}-option-${index}`;
  }

  function handleSearchInput(e: Event) {
    const query = (e.target as HTMLInputElement).value;
    searchQuery = query;
    selectedIndex = 0;

    if (searchTimeout) clearTimeout(searchTimeout);
    const version = ++searchVersion;

    // FTS only for two or more characters; shorter queries clear stale hits.
    if (query.trim().length < 2) {
      searchResults = [];
      isSearching = false;
      return;
    }

    searchTimeout = setTimeout(async () => {
      isSearching = true;
      try {
        const results = await globalSearch(query, 10);
        if (version === searchVersion) searchResults = results;
      } catch (err) {
        console.error('Search error:', err);
        if (version === searchVersion) searchResults = [];
      } finally {
        if (version === searchVersion) isSearching = false;
      }
    }, 250);
  }

  // Close only when the backdrop itself is clicked, not a click inside the dialog
  function handleBackdropClick(e: MouseEvent) {
    if (e.target === e.currentTarget) {
      onClose();
    }
  }

  function scrollActiveIntoView() {
    queueMicrotask(() => {
      listRef
        ?.querySelector(`#${CSS.escape(optionId(selectedIndex))}`)
        ?.scrollIntoView?.({ block: 'nearest' });
    });
  }

  // Bound on the dialog so it also covers focus landing on the dialog
  // container rather than the search input.
  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      e.preventDefault();
      e.stopPropagation();
      onClose();
      return;
    }
    if (items.length === 0) return;
    switch (e.key) {
      case 'ArrowDown':
        e.preventDefault();
        selectedIndex = Math.min(selectedIndex + 1, items.length - 1);
        scrollActiveIntoView();
        break;
      case 'ArrowUp':
        e.preventDefault();
        selectedIndex = Math.max(selectedIndex - 1, 0);
        scrollActiveIntoView();
        break;
      case 'Home':
        if (e.target !== inputRef) {
          e.preventDefault();
          selectedIndex = 0;
        }
        break;
      case 'End':
        if (e.target !== inputRef) {
          e.preventDefault();
          selectedIndex = items.length - 1;
        }
        break;
      case 'Enter':
        e.preventDefault();
        if (items[selectedIndex]) void execute(items[selectedIndex]);
        break;
    }
  }

  async function execute(item: Item) {
    try {
      await item.run();
    } catch (error) {
      console.error('Failed to execute command:', error);
    }
  }

  // Reset and load fresh data each time the palette opens.
  $effect(() => {
    if (!isOpen) return;
    searchQuery = '';
    selectedIndex = 0;
    searchResults = [];
    const timer = setTimeout(() => inputRef?.focus(), 50);
    if ($authStatus === 'unlocked') {
      void refreshChatSessions();
      if (!patients) {
        listPatients()
          .then((list) => (loadedPatients = list))
          .catch((error) => console.error('Failed to load patients:', error));
      }
    }
    return () => clearTimeout(timer);
  });
</script>

{#if isOpen}
  <div
    class="fixed inset-0 z-50 flex items-start justify-center bg-scrim pt-[12vh]"
    onclick={handleBackdropClick}
    role="presentation"
  >
    <div
      class="mx-4 flex max-h-[76vh] w-full max-w-2xl flex-col overflow-hidden rounded-card border border-line bg-surface-overlay shadow-modal"
      role="dialog"
      aria-modal="true"
      aria-label={$t('commandPalette.title')}
      tabindex="-1"
      onkeydown={handleKeydown}
    >
      <div class="flex h-12 shrink-0 items-center gap-2.5 border-b border-line-subtle px-4">
        <Search class="shrink-0 text-fg-subtle" size={16} aria-hidden="true" />
        <!-- A borderless combobox input: the dialog is the field here, so the
             bordered Input primitive does not fit. -->
        <input
          bind:this={inputRef}
          type="text"
          role="combobox"
          aria-expanded="true"
          aria-controls="{listId}-list"
          aria-autocomplete="list"
          aria-activedescendant={items.length ? optionId(selectedIndex) : undefined}
          aria-label={$t('commandPalette.placeholder')}
          placeholder={$t('commandPalette.placeholder')}
          class="min-w-0 flex-1 bg-transparent text-body text-fg placeholder:text-fg-subtle focus:outline-none"
          value={searchQuery}
          oninput={handleSearchInput}
        />
        <Kbd class="shrink-0">esc</Kbd>
      </div>

      <div
        bind:this={listRef}
        id="{listId}-list"
        role="listbox"
        aria-label={$t('commandPalette.title')}
        class="min-h-0 flex-1 overflow-y-auto p-1.5"
      >
        {#if items.length === 0}
          <p class="px-3 py-8 text-center text-body text-fg-muted">
            {isSearching ? $t('commandPalette.searching') : $t('commandPalette.noResults')}
          </p>
        {:else}
          {#each groups as group (group.group)}
            <div role="group" aria-labelledby="{listId}-{group.group}" class="pb-1">
              <div
                id="{listId}-{group.group}"
                class="px-2 pb-1 pt-2 text-caption font-medium text-fg-subtle"
              >
                {groupLabel[group.group]}
              </div>
              {#each group.entries as { item, index } (item.id)}
                {@const Icon = item.icon}
                {@const selected = index === selectedIndex}
                <!-- Options are buttons so a pointer click runs them; keyboard
                     focus stays in the combobox input (aria-activedescendant). -->
                <button
                  type="button"
                  role="option"
                  id={optionId(index)}
                  tabindex="-1"
                  aria-selected={selected}
                  class="flex w-full items-center gap-2.5 rounded-control px-2 text-left transition-colors duration-150 ease-standard {item.result
                    ? 'min-h-row py-1.5'
                    : 'h-control'} {selected ? 'bg-surface-selected' : 'hover:bg-surface-hover'}"
                  onmousemove={() => (selectedIndex = index)}
                  onclick={() => void execute(item)}
                >
                  <Icon size={16} class="shrink-0 text-fg-subtle" aria-hidden="true" />
                  {#if item.result}
                    {@const r = item.result}
                    <span class="min-w-0 flex-1">
                      <span class="flex items-baseline gap-2">
                        <span class="truncate text-body text-fg">{r.title}</span>
                        <span class="shrink-0 text-caption text-fg-subtle">
                          {typeLabel[r.result_type] ?? r.result_type}
                        </span>
                      </span>
                      {#if r.snippet}
                        <HighlightedSnippet
                          snippet={r.snippet}
                          class="line-clamp-1 text-caption text-fg-muted"
                        />
                      {/if}
                    </span>
                    <span class="shrink-0 text-right text-caption text-fg-subtle">
                      <span class="block">{r.patient_name}</span>
                      {#if r.date}<span class="block">{$fmt.date(r.date)}</span>{/if}
                    </span>
                  {:else}
                    <span class="min-w-0 flex-1 truncate text-body text-fg">{item.label}</span>
                    {#if item.secondary}
                      <span class="max-w-[45%] shrink-0 truncate text-caption text-fg-subtle">
                        {item.secondary}
                      </span>
                    {/if}
                    {#if item.shortcut}
                      <Kbd class="shrink-0">{item.shortcut}</Kbd>
                    {/if}
                  {/if}
                </button>
              {/each}
            </div>
          {/each}
          {#if isSearching}
            <p class="px-2 py-2 text-caption text-fg-subtle">{$t('commandPalette.searching')}</p>
          {/if}
        {/if}
      </div>

      <div
        class="flex shrink-0 items-center gap-4 border-t border-line-subtle px-4 py-2 text-caption text-fg-subtle"
      >
        <span class="flex items-center gap-1.5">
          <Kbd>↑↓</Kbd>{$t('commandPalette.navigate')}
        </span>
        <span class="flex items-center gap-1.5">
          <Kbd>↵</Kbd>{$t('commandPalette.select')}
        </span>
        <span class="ml-auto flex items-center gap-1.5">
          <Kbd>⌘K</Kbd>{$t('commandPalette.openWith')}
        </span>
      </div>
    </div>
  </div>
{/if}
