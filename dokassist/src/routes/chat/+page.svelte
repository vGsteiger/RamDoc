<script lang="ts">
  /*
   * The chat workspace (primary clinical surface).
   *
   * URL contract:
   *   /chat                         fresh chat with the Today panel (D3)
   *   /chat?new=<any>               same; a changed value forces a fresh chat (Cmd+N)
   *   /chat?sessionId=<id>          open a saved chat
   *   /chat?patientId=<id>          fresh chat with that patient attached
   *   /chat?patientId=<id>&intent=report   … with the report starter prompt
   *
   * A fresh chat is only persisted when its first assistant message is sent;
   * the URL is then replaced with `?sessionId=<id>`.
   */
  import { onMount, untrack } from 'svelte';
  import { get } from 'svelte/store';
  import { page } from '$app/state';
  import { goto } from '$app/navigation';
  import { resolve } from '$app/paths';
  import { PanelRight, Paperclip, UserRound, X } from 'lucide-svelte';
  import {
    createChatSession,
    getPatient,
    listChatSessions,
    type ChatSession,
    type Patient,
  } from '$lib/api';
  import { ensureEngineLoaded } from '$lib/stores/engine';
  import ChatThread from '$lib/components/ChatThread.svelte';
  import ContextPicker from '$lib/components/context-picker/ContextPicker.svelte';
  import TodayPanel from '$lib/components/chat/TodayPanel.svelte';
  import PatientContextPanel from '$lib/components/chat/PatientContextPanel.svelte';
  import { notifyChatSessionsChanged } from '$lib/components/chat/chat-events';
  import type { ChatMode } from '$lib/components/chat/patient-context';
  import { t } from '$lib/translations';
  import {
    createContextPlan,
    removeContextPatient,
    type ContextPlan,
  } from '$lib/components/context-picker';
  import { Button, IconButton, ListRow } from '$lib/components/ui';

  /** The context panel is docked beside the thread from this width; below it is a sheet. */
  const PANEL_DOCK_QUERY = '(min-width: 1280px)';
  const PATIENT_PROMPT_KEYS = ['patient1', 'patient2', 'patient3', 'patient4'];

  let activeSessionId = $state<string | null>(null);
  let session = $state<ChatSession | null>(null);
  /** The patient of a fresh, not yet persisted, patient chat. */
  let pendingPatientId = $state<string | null>(null);
  let scopedPatient = $state<Patient | null>(null);
  let contextPlan = $state<ContextPlan>(createContextPlan());
  let showContextPicker = $state(false);
  let threadKey = $state(0);
  let draft = $state('');
  let mode = $state<ChatMode>('assistant');
  let starterPrompt = $state('');
  let panelDocked = $state(false);
  let panelOpen = $state(false);
  let thread = $state<{ focusComposer: () => Promise<void> } | null>(null);

  let loadedRoute: string | null = null;
  let routeLoadVersion = 0;
  /** Set right before navigating to a fresh patient chat from the Today panel. */
  let queuedPrompt: string | null = null;

  let scopedPatientId = $derived(session ? session.patient_id : pendingPatientId);
  let scope = $derived<'global' | 'patient'>(
    session ? (session.scope === 'patient' ? 'patient' : 'global') : pendingPatientId ? 'patient' : 'global'
  );
  /** The one patient the panel and records-only mode refer to. */
  let attachedPatient = $derived<Patient | null>(
    scope === 'patient'
      ? scopedPatient && scopedPatient.id === scopedPatientId
        ? scopedPatient
        : null
      : !contextPlan.comparisonMode && contextPlan.selectedPatients.length === 1
        ? contextPlan.selectedPatients[0]
        : null
  );
  let recordsPatient = $derived(
    attachedPatient
      ? {
          id: attachedPatient.id,
          name: `${attachedPatient.first_name} ${attachedPatient.last_name}`,
        }
      : null
  );
  let title = $derived(
    session?.title ??
      (recordsPatient
        ? $t('chat.askAbout').replace('{name}', recordsPatient.name)
        : $t('chat.newChat'))
  );

  function patientName(patient: Patient) {
    return `${patient.first_name} ${patient.last_name}`;
  }

  /** Saved chats are listed per scope; find this one to learn its patient. */
  async function findSession(id: string): Promise<ChatSession | null> {
    const [global, patient] = await Promise.all([
      listChatSessions('global').catch(() => [] as ChatSession[]),
      listChatSessions('patient').catch(() => [] as ChatSession[]),
    ]);
    return [...global, ...patient].find((item) => item.id === id) ?? null;
  }

  async function loadScopedPatient(id: string, version: number) {
    try {
      const patient = await getPatient(id);
      if (version === routeLoadVersion) scopedPatient = patient;
    } catch (error) {
      console.error('Failed to load the chat patient:', error);
    }
  }

  async function openSession(id: string, version: number) {
    try {
      const found = await findSession(id);
      if (version !== routeLoadVersion) return;
      session = found;
      if (found?.scope === 'patient' && found.patient_id) {
        void loadScopedPatient(found.patient_id, version);
      }
    } catch (error) {
      console.error('Failed to load chat session:', error);
    }
  }

  function resetChatState() {
    session = null;
    pendingPatientId = null;
    scopedPatient = null;
    contextPlan = createContextPlan();
    showContextPicker = false;
    mode = 'assistant';
    draft = '';
    starterPrompt = '';
  }

  $effect(() => {
    const params = page.url.searchParams;
    const sessionParam = params.get('sessionId');
    const route = page.url.search;
    if (route === loadedRoute) return;
    loadedRoute = route;
    untrack(() => applyRoute(params, sessionParam));
  });

  function applyRoute(params: Pick<URLSearchParams, 'get'>, sessionParam: string | null) {
    // The URL was just rewritten for the chat this page created: keep the thread.
    if (sessionParam && sessionParam === activeSessionId) return;

    const version = ++routeLoadVersion;
    resetChatState();
    if (sessionParam) {
      activeSessionId = sessionParam;
      void openSession(sessionParam, version);
    } else {
      activeSessionId = null;
      const patientParam = params.get('patientId');
      if (patientParam) {
        pendingPatientId = patientParam;
        void loadScopedPatient(patientParam, version);
      }
      const prompt =
        queuedPrompt ??
        (patientParam && params.get('intent') === 'report' ? get(t)('chat.reportStarterPrompt') : '');
      queuedPrompt = null;
      starterPrompt = prompt;
      draft = prompt;
      if (prompt) void thread?.focusComposer();
    }
    threadKey += 1;
  }

  function chatTitleFrom(text: string): string {
    const line = text.replace(/\s+/g, ' ').trim();
    if (line.length <= 60) return line || get(t)('chat.defaultTitle');
    const cut = line.slice(0, 60);
    const space = cut.lastIndexOf(' ');
    return `${space > 30 ? cut.slice(0, space) : cut}…`;
  }

  async function createSession(firstMessage: string): Promise<string> {
    const created = await createChatSession(
      scope,
      scope === 'patient' ? (scopedPatientId ?? undefined) : undefined,
      chatTitleFrom(firstMessage)
    );
    session = created;
    activeSessionId = created.id;
    notifyChatSessionsChanged();
    void goto(`${resolve('/chat')}?sessionId=${encodeURIComponent(created.id)}`, {
      replace: true,
      reset: false,
    });
    return created.id;
  }

  function startPatientChat(patientId: string, prompt: string) {
    queuedPrompt = prompt;
    void goto(`${resolve('/chat')}?patientId=${encodeURIComponent(patientId)}`);
  }

  function usePrompt(prompt: string) {
    draft = prompt;
    void thread?.focusComposer();
  }

  function detachPendingPatient() {
    void goto(`${resolve('/chat')}?new=${Date.now()}`);
  }

  function togglePanel() {
    panelOpen = !panelOpen;
  }

  function onWindowKeydown(event: KeyboardEvent) {
    if (event.key !== 'Escape') return;
    if (showContextPicker) {
      showContextPicker = false;
    } else if (panelOpen && !panelDocked) {
      panelOpen = false;
    }
  }

  onMount(() => {
    void ensureEngineLoaded();
    if (typeof window.matchMedia !== 'function') return;
    const query = window.matchMedia(PANEL_DOCK_QUERY);
    panelDocked = query.matches;
    panelOpen = query.matches;
    const onChange = (event: MediaQueryListEvent) => {
      panelDocked = event.matches;
      panelOpen = event.matches;
    };
    query.addEventListener('change', onChange);
    return () => query.removeEventListener('change', onChange);
  });
</script>

<svelte:window onkeydown={onWindowKeydown} />

{#snippet contextBar()}
  {#if scope === 'patient'}
    {#if attachedPatient}
      <span
        class="inline-flex h-control-sm min-w-0 max-w-full items-center gap-1.5 rounded-full bg-fill pl-2.5 pr-1 text-label text-fg"
        data-testid="attached-patient-chip"
      >
        <UserRound size={14} class="shrink-0 text-fg-subtle" aria-hidden="true" />
        <span class="min-w-0 truncate"
          ><span class="sr-only">{$t('chat.attachedPatient').replace('{name}', '')}</span
          >{patientName(attachedPatient)}</span
        >
        {#if !session}
          <IconButton
            size="sm"
            class="-my-1 size-6"
            label={$t('chat.removeAttachedPatient').replace('{name}', patientName(attachedPatient))}
            onclick={detachPendingPatient}
          >
            <X size={12} />
          </IconButton>
        {/if}
      </span>
    {/if}
  {:else}
    {#each contextPlan.selectedPatients as patient (patient.id)}
      <span
        class="inline-flex h-control-sm min-w-0 max-w-full items-center gap-1.5 rounded-full bg-fill pl-2.5 pr-1 text-label text-fg"
        data-testid="attached-patient-chip"
      >
        <UserRound size={14} class="shrink-0 text-fg-subtle" aria-hidden="true" />
        <span class="min-w-0 truncate">{patientName(patient)}</span>
        <IconButton
          size="sm"
          class="-my-1 size-6"
          label={$t('chat.removeAttachedPatient').replace('{name}', patientName(patient))}
          onclick={() => (contextPlan = removeContextPatient(contextPlan, patient.id))}
        >
          <X size={12} />
        </IconButton>
      </span>
    {/each}
    <Button
      size="sm"
      variant="ghost"
      aria-expanded={showContextPicker}
      aria-controls="chat-context-picker"
      onclick={() => (showContextPicker = !showContextPicker)}
    >
      <Paperclip size={14} aria-hidden="true" />
      {contextPlan.selectedPatients.length ? $t('chat.changeContext') : $t('chat.attachContext')}
    </Button>
    {#if showContextPicker}
      <div
        id="chat-context-picker"
        class="glass-strong absolute inset-x-0 bottom-full z-30 mb-2 max-h-[min(70vh,36rem)] overflow-y-auto rounded-panel p-4"
      >
        <ContextPicker
          bind:value={contextPlan}
          onclose={() => (showContextPicker = false)}
          closeLabel={$t('chat.closeContextPicker')}
          labels={{
            title: $t('chat.picker.title'),
            patientSearch: $t('chat.picker.patientSearch'),
            patientSearchPlaceholder: $t('chat.picker.patientSearchPlaceholder'),
            comparisonMode: $t('chat.picker.comparisonMode'),
            comparisonModeHint: $t('chat.picker.comparisonModeHint'),
            selectedPatients: $t('chat.picker.selectedPatients'),
            clearPatients: $t('chat.picker.clearPatients'),
            sources: $t('chat.picker.sources'),
            pin: $t('chat.picker.pin'),
            unpin: $t('chat.picker.unpin'),
            included: $t('chat.picker.included'),
            retrieved: $t('chat.picker.retrieved'),
            summarized: $t('chat.picker.summarized'),
            loading: $t('chat.picker.loading'),
            noPatients: $t('chat.picker.noPatients'),
          }}
          sourceLabels={{
            overview: $t('chat.contextSourceOverview'),
            diagnoses: $t('chat.contextSourceDiagnoses'),
            medications: $t('chat.contextSourceMedications'),
            sessions: $t('chat.contextSourceSessions'),
            files: $t('chat.contextSourceFiles'),
          }}
        />
      </div>
    {/if}
  {/if}
{/snippet}

{#snippet emptyState()}
  {#if recordsPatient}
    <section class="space-y-3" aria-labelledby="chat-patient-prompts">
      <h2 id="chat-patient-prompts" class="text-title text-fg">
        {$t('chat.askAbout').replace('{name}', recordsPatient.name)}
      </h2>
      <div class="-mx-3">
        {#each PATIENT_PROMPT_KEYS as key (key)}
          {@const prompt = $t(`chat.suggestedPrompts.${key}`)}
          <ListRow title={prompt} onclick={() => usePrompt(prompt)} />
        {/each}
      </div>
    </section>
  {:else if scope === 'global'}
    <TodayPanel onsummarize={startPatientChat} onprompt={usePrompt} />
  {/if}
{/snippet}

<div class="flex h-full min-h-0 min-w-0" data-testid="chat-workspace">
  <div class="flex min-h-0 min-w-0 flex-1 flex-col">
    <header
      class="flex min-h-12 shrink-0 items-center gap-2 px-4 sm:px-6"
    >
      <h1 class="min-w-0 flex-1 truncate text-heading font-medium text-fg">{title}</h1>
      {#if recordsPatient}
        <IconButton
          label={panelOpen ? $t('chat.hidePatientPanel') : $t('chat.showPatientPanel')}
          aria-expanded={panelOpen}
          aria-controls="chat-patient-panel"
          onclick={togglePanel}
          class={panelOpen ? 'bg-surface-selected text-fg' : ''}
        >
          <PanelRight size={16} />
        </IconButton>
      {/if}
    </header>

    {#key threadKey}
      <ChatThread
        bind:this={thread}
        bind:sessionId={activeSessionId}
        bind:draft
        bind:mode
        {scope}
        patientId={scopedPatientId ?? undefined}
        initialMessage={starterPrompt}
        {contextPlan}
        {recordsPatient}
        oncreatesession={createSession}
        empty={emptyState}
        {contextBar}
      />
    {/key}
  </div>

  {#if recordsPatient && panelOpen}
    {#if panelDocked}
      <div class="glass my-2 mr-2 w-80 shrink-0 overflow-hidden rounded-panel">
        <PatientContextPanel id="chat-patient-panel" patientId={recordsPatient.id} />
      </div>
    {:else}
      <div
        class="fixed inset-0 z-40 bg-scrim"
        role="presentation"
        onclick={() => (panelOpen = false)}
      ></div>
      <div
        class="glass-strong fixed inset-y-2 right-2 z-50 w-[min(22rem,calc(100vw-1rem))] overflow-hidden rounded-panel"
        role="dialog"
        aria-modal="true"
        aria-label={$t('chat.patientPanel.title')}
      >
        <PatientContextPanel
          id="chat-patient-panel"
          patientId={recordsPatient.id}
          onclose={() => (panelOpen = false)}
        />
      </div>
    {/if}
  {/if}
</div>
