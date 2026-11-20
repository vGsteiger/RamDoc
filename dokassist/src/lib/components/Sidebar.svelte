<script lang="ts">
  import { onMount, tick } from 'svelte';
  import { page } from '$app/state';
  import { goto } from '$app/navigation';
  import { lockApp } from '$lib/api';
  import { authStatus } from '$lib/stores/auth';
  import { engine, loadingModelLabel, refreshEngineStatus } from '$lib/stores/engine';
  import { chatSessions, refreshChatSessions } from '$lib/stores/chat-sessions';
  import { t } from '$lib/translations';
  import { isActivePath, NEW_CHAT_URL } from '$lib/shell';
  import ChatSessionList from '$lib/components/ChatSessionList.svelte';
  import { BrandMark, IconButton, Kbd, StatusIndicator, Tooltip } from '$lib/components/ui';
  import {
    AlertTriangle,
    BookOpen,
    Calendar,
    CheckCircle2,
    Circle,
    LoaderCircle,
    Lock,
    MessageSquare,
    PanelLeftClose,
    PanelLeftOpen,
    Search,
    Settings,
    SquarePen,
    Users,
    XCircle,
  } from 'lucide-svelte';

  /*
   * Mac source-list sidebar (D2). Three presentations:
   *   full     232px, in flow (≥1024px, not collapsed)
   *   rail     52px icon rail with tooltips (700–1023px, or collapsed)
   *   overlay  the full sidebar floating over a rail-width spacer, opened from
   *            the rail below 1024px so the recent chats stay reachable.
   *
   * Rows are hand-built links/buttons rather than Button primitives: they are
   * source-list rows (full-width, left-aligned, active bar), not actions.
   */
  interface Props {
    /** User preference: collapse to the rail on wide windows. */
    collapsed?: boolean;
    /** Window is ≥1024px. Below that the sidebar is always a rail. */
    wide?: boolean;
    overlayOpen?: boolean;
    onToggle?: () => void;
    onOverlayClose?: () => void;
    onOpenPalette?: () => void;
  }

  let {
    collapsed = false,
    wide = true,
    overlayOpen = false,
    onToggle,
    onOverlayClose,
    onOpenPalette,
  }: Props = $props();

  let mode = $derived<'full' | 'rail' | 'overlay'>(
    wide ? (collapsed ? 'rail' : 'full') : overlayOpen ? 'overlay' : 'rail'
  );
  let rail = $derived(mode === 'rail');

  let sidebarElement = $state<HTMLElement | null>(null);
  let restoreFocusTo: HTMLElement | null = null;

  let currentPath = $derived(page.url.pathname);
  let activeSessionId = $derived(
    currentPath === '/chat' ? page.url.searchParams.get('sessionId') : null
  );
  let unlocked = $derived($authStatus === 'unlocked');

  const navItems = [
    { path: '/patients', labelKey: 'nav.patients', icon: Users, aliases: [] as string[] },
    { path: '/calendar', labelKey: 'nav.calendar', icon: Calendar, aliases: [] as string[] },
    { path: '/reference', labelKey: 'nav.reference', icon: BookOpen, aliases: ['/literature'] },
  ];

  let chatActive = $derived(isActivePath(currentPath, '/chat'));
  // With a chat row selected, the row carries the bar; "Chat" stays emphasised.
  let chatItemSelected = $derived(chatActive && (rail || !activeSessionId));

  const statusIcons = {
    ok: CheckCircle2,
    warn: AlertTriangle,
    error: XCircle,
    idle: Circle,
    busy: LoaderCircle,
  };
  const statusTones = {
    ok: 'text-success-fg',
    warn: 'text-warning-fg',
    error: 'text-danger-fg',
    idle: 'text-fg-subtle',
    busy: 'text-fg-muted',
  };

  let modelStatus = $derived.by(() => {
    const status = $engine.status;
    const name =
      status?.desired_model?.name ??
      loadingModelLabel(status?.lifecycle?.active_filename ?? status?.model_name ?? null);
    if ($engine.isLoading) {
      return {
        tone: 'busy' as const,
        label: $t('nav.modelLoading'),
        detail: loadingModelLabel($engine.loadingFilename) || undefined,
      };
    }
    if (!status) return { tone: 'idle' as const, label: $t('nav.modelChecking') };
    if (status.lifecycle?.phase === 'error') {
      return { tone: 'error' as const, label: $t('nav.modelError'), detail: name || undefined };
    }
    if (status.is_loaded) {
      return { tone: 'ok' as const, label: $t('nav.modelReady'), detail: name || undefined };
    }
    if (status.is_downloaded) {
      return { tone: 'idle' as const, label: $t('nav.modelNotLoaded'), detail: name || undefined };
    }
    return { tone: 'warn' as const, label: $t('nav.noModel') };
  });
  let StatusIcon = $derived(statusIcons[modelStatus.tone]);

  async function handleLock() {
    try {
      await lockApp();
      authStatus.set('locked');
      goto('/unlock');
    } catch (error) {
      console.error('Failed to lock app:', error);
    }
  }

  function onNavigate() {
    if (mode === 'overlay') onOverlayClose?.();
  }

  function handleSessionDeleted(sessionId: string) {
    void refreshChatSessions();
    if (sessionId === activeSessionId) void goto('/chat', { replaceState: true });
  }

  // Keep the recent-chat list current: the chat page creates and titles
  // sessions, so refresh on every navigation (local IPC, cheap).
  $effect(() => {
    void page.url.href;
    if (unlocked) void refreshChatSessions();
  });

  onMount(() => {
    const interval = setInterval(() => {
      if ($authStatus === 'unlocked') void refreshEngineStatus();
    }, 5000);
    return () => clearInterval(interval);
  });

  $effect(() => {
    if (unlocked) void refreshEngineStatus();
  });

  /* Overlay: focus moves in, Tab is trapped, Esc and the scrim close it. */
  function focusableElements(): HTMLElement[] {
    return sidebarElement
      ? [...sidebarElement.querySelectorAll<HTMLElement>('a[href], button:not([disabled])')]
      : [];
  }

  function handleOverlayKeydown(event: KeyboardEvent) {
    if (event.key === 'Escape') {
      event.preventDefault();
      onOverlayClose?.();
      return;
    }
    if (event.key !== 'Tab') return;
    const focusable = focusableElements();
    if (!focusable.length) return;
    const currentIndex = focusable.indexOf(document.activeElement as HTMLElement);
    if (event.shiftKey && currentIndex <= 0) {
      event.preventDefault();
      focusable.at(-1)?.focus();
    } else if (!event.shiftKey && currentIndex === focusable.length - 1) {
      event.preventDefault();
      focusable[0]?.focus();
    }
  }

  $effect(() => {
    if (mode !== 'overlay') return;
    restoreFocusTo = document.activeElement instanceof HTMLElement ? document.activeElement : null;
    void tick().then(() => focusableElements()[0]?.focus());
    window.addEventListener('keydown', handleOverlayKeydown);
    return () => {
      window.removeEventListener('keydown', handleOverlayKeydown);
      restoreFocusTo?.focus();
    };
  });

  const rowBase =
    'relative flex h-control w-full items-center gap-2.5 rounded-control text-body transition-colors duration-150 ease-standard';
  function rowClass(selected: boolean) {
    return `${rowBase} ${rail ? 'justify-center' : 'px-2'} ${
      selected
        ? 'bg-fill-strong font-medium text-fg'
        : 'text-fg-muted hover:bg-fill hover:text-fg'
    }`;
  }
</script>

{#snippet activeBar()}
  <span
    class="absolute left-0 top-1/2 h-4 w-0.5 -translate-y-1/2 rounded-full bg-brand"
    aria-hidden="true"
  ></span>
{/snippet}

{#snippet navLink(
  href: string,
  label: string,
  Icon: typeof Users,
  selected: boolean,
  current: boolean,
  hint?: string
)}
  {#if rail}
    <Tooltip text={hint ? `${label} (${hint})` : label} side="right" class="w-full">
      {#snippet children(describedby)}
        <a
          {href}
          class={rowClass(selected)}
          aria-current={current ? 'page' : undefined}
          aria-label={label}
          aria-describedby={describedby}
          onclick={onNavigate}
        >
          {#if selected}{@render activeBar()}{/if}
          <Icon size={18} class={selected ? 'text-fg' : 'text-fg-subtle'} aria-hidden="true" />
        </a>
      {/snippet}
    </Tooltip>
  {:else}
    <a
      {href}
      class={rowClass(selected)}
      aria-current={current ? 'page' : undefined}
      onclick={onNavigate}
    >
      {#if selected}{@render activeBar()}{/if}
      <Icon
        size={16}
        class="shrink-0 {selected ? 'text-fg' : 'text-fg-subtle'}"
        aria-hidden="true"
      />
      <span class="min-w-0 flex-1 truncate">{label}</span>
      {#if hint}<Kbd class="shrink-0">{hint}</Kbd>{/if}
    </a>
  {/if}
{/snippet}

{#snippet navButton(label: string, Icon: typeof Users, onclick: () => void, hint?: string)}
  {#if rail}
    <Tooltip text={hint ? `${label} (${hint})` : label} side="right" class="w-full">
      {#snippet children(describedby)}
        <button
          type="button"
          class={rowClass(false)}
          aria-label={label}
          aria-describedby={describedby}
          {onclick}
        >
          <Icon size={18} class="text-fg-subtle" aria-hidden="true" />
        </button>
      {/snippet}
    </Tooltip>
  {:else}
    <button type="button" class={rowClass(false)} {onclick}>
      <Icon size={16} class="shrink-0 text-fg-subtle" aria-hidden="true" />
      <span class="min-w-0 flex-1 truncate text-left">{label}</span>
      {#if hint}<Kbd class="shrink-0">{hint}</Kbd>{/if}
    </button>
  {/if}
{/snippet}

{#if mode === 'overlay'}
  <!-- Keeps the content where the rail was while the full sidebar floats. -->
  <div class="w-13 shrink-0 border-r border-line-subtle bg-surface-sunken" aria-hidden="true"></div>
  <button
    type="button"
    class="fixed inset-0 z-30 bg-scrim"
    aria-label={$t('nav.closeNavigation')}
    tabindex="-1"
    onclick={() => onOverlayClose?.()}
  ></button>
{/if}

<aside
  bind:this={sidebarElement}
  data-sidebar-mode={mode}
  aria-label={$t('nav.sidebar')}
  class="glass flex shrink-0 flex-col overflow-hidden rounded-panel {rail
    ? 'w-13'
    : 'w-58'} {mode === 'overlay'
    ? 'fixed inset-y-2 left-2 z-40'
    : 'relative z-10 my-2 ml-2'}"
>
  <!-- Header: brand, and the collapse control. -->
  <div class="flex h-12 shrink-0 items-center gap-2 {rail ? 'justify-center' : 'pl-3 pr-1.5'}">
    {#if rail}
      <BrandMark size={20} title="RamDoc" />
    {:else}
      <a
        href="/chat"
        class="flex min-w-0 items-center gap-2 rounded-control text-fg"
        onclick={onNavigate}
      >
        <BrandMark size={20} />
        <span class="truncate text-heading font-semibold">RamDoc</span>
      </a>
      <IconButton
        size="sm"
        class="ml-auto"
        label={$t(mode === 'overlay' ? 'nav.closeNavigation' : 'nav.collapseSidebar')}
        onclick={() => (mode === 'overlay' ? onOverlayClose?.() : onToggle?.())}
      >
        <PanelLeftClose size={16} aria-hidden="true" />
      </IconButton>
    {/if}
  </div>

  <div class="shrink-0 space-y-px px-2 pb-2">
    {#if rail}
      {@render navButton(
        $t(wide ? 'nav.expandSidebar' : 'nav.openNavigation'),
        PanelLeftOpen,
        () => onToggle?.(),
        '⌘\\'
      )}
    {/if}
    {@render navLink(NEW_CHAT_URL, $t('nav.newChat'), SquarePen, false, false, '⌘N')}
    {@render navButton($t('nav.search'), Search, () => onOpenPalette?.(), '⌘K')}
  </div>

  <nav
    class="min-h-0 flex-1 border-t border-line-subtle px-2 py-2 {rail ? '' : 'overflow-y-auto'}"
    aria-label={$t('nav.primaryNavigation')}
  >
    <ul class="space-y-px">
      <li>
        {@render navLink(
          '/chat',
          $t('nav.chat'),
          MessageSquare,
          chatItemSelected,
          chatActive && !activeSessionId
        )}
        {#if !rail}
          <div class="mt-px">
            <ChatSessionList
              sessions={$chatSessions.sessions}
              patientNames={$chatSessions.patientNames}
              {activeSessionId}
              onnavigate={onNavigate}
              onsessiondeleted={handleSessionDeleted}
              onlistchange={() => void refreshChatSessions()}
            />
          </div>
        {/if}
      </li>
      {#each navItems as item (item.path)}
        {@const active = isActivePath(currentPath, item.path, item.aliases)}
        <li>{@render navLink(item.path, $t(item.labelKey), item.icon, active, active)}</li>
      {/each}
    </ul>
  </nav>

  <div class="shrink-0 space-y-px border-t border-line-subtle p-2">
    {@render navLink(
      '/settings',
      $t('nav.settings'),
      Settings,
      isActivePath(currentPath, '/settings'),
      isActivePath(currentPath, '/settings')
    )}
    {@render navButton($t('nav.lock'), Lock, handleLock)}
    {#if rail}
      <Tooltip
        text={modelStatus.detail
          ? `${modelStatus.label} · ${modelStatus.detail}`
          : modelStatus.label}
        side="right"
        class="w-full"
      >
        {#snippet children(describedby)}
          <a
            href="/settings"
            class={rowClass(false)}
            aria-label={modelStatus.label}
            aria-describedby={describedby}
            data-model-status={modelStatus.tone}
            onclick={onNavigate}
          >
            <StatusIcon
              size={16}
              class="{statusTones[modelStatus.tone]} {modelStatus.tone === 'busy'
                ? 'animate-spin'
                : ''}"
              aria-hidden="true"
            />
          </a>
        {/snippet}
      </Tooltip>
    {:else}
      <!-- The whole chip links to settings; the state reads first, the model
           name second, so a long name never truncates "Model ready". -->
      <a
        href="/settings"
        title={$t('nav.modelSettings')}
        data-model-status={modelStatus.tone}
        class="flex min-h-control flex-col justify-center rounded-control px-2 py-1 transition-colors duration-150 ease-standard hover:bg-surface-hover"
        onclick={onNavigate}
      >
        <StatusIndicator tone={modelStatus.tone} label={modelStatus.label} />
        {#if modelStatus.detail}
          <span class="truncate pl-5 text-caption text-fg-subtle">{modelStatus.detail}</span>
        {/if}
      </a>
    {/if}
  </div>
</aside>
