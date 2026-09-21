<script lang="ts">
  import { get } from 'svelte/store';
  import { onMount } from 'svelte';
  import { page } from '$app/stores';
  import {
    listChatSessions,
    createChatSession,
    getOrCreatePatientChatSession,
    getPatient,
    type ChatSession,
  } from '$lib/api';
  import { ensureEngineLoaded } from '$lib/stores/engine';
  import ChatSessionList from '$lib/components/ChatSessionList.svelte';
  import ChatThread from '$lib/components/ChatThread.svelte';
  import { t } from '$lib/translations';
  import ContextPicker from '$lib/components/context-picker/ContextPicker.svelte';
  import {
    createContextPlan,
    selectContextPatient,
    type ContextPlan,
  } from '$lib/components/context-picker';

  let sessions = $state<ChatSession[]>([]);
  let activeSessionId = $state<string | null>(null);
  let isLoading = $state(true);
  let patientId = $derived($page.url.searchParams.get('patientId'));
  let intent = $derived($page.url.searchParams.get('intent'));
  let scope = $derived<'global' | 'patient'>(patientId ? 'patient' : 'global');
  let starterPrompt = $derived(intent === 'report' ? get(t)('chat.reportStarterPrompt') : '');
  let contextPlan = $state<ContextPlan>(createContextPlan());
  let showContextPicker = $state(false);

  async function preloadLegacyPatientContext() {
    if (!patientId) return;
    try {
      contextPlan = selectContextPatient(contextPlan, await getPatient(patientId));
    } catch (error) {
      console.error('Failed to preselect patient context:', error);
    }
  }

  async function loadSessions() {
    try {
      isLoading = true;
      sessions = await listChatSessions(scope, patientId ?? undefined);
      if (patientId && sessions.length === 0) {
        const session = await getOrCreatePatientChatSession(patientId);
        sessions = [session];
      }
      if (sessions.length > 0) activeSessionId = sessions[0].id;
    } catch (e) {
      console.error('Failed to load chat sessions:', e);
    } finally {
      isLoading = false;
    }
  }

  async function handleNewSession() {
    try {
      const session = await createChatSession(
        scope,
        patientId ?? undefined,
        get(t)('chat.defaultTitle')
      );
      sessions = [session, ...sessions];
      activeSessionId = session.id;
    } catch (e) {
      console.error('Failed to create session:', e);
    }
  }

  onMount(() => {
    if (patientId) showContextPicker = true;
    void preloadLegacyPatientContext();
    loadSessions();
    void ensureEngineLoaded();
  });
</script>

<div class="flex h-full flex-col sm:flex-row">
  <!-- Sidebar: session list -->
  <div
    class="h-52 w-full border-b border-line flex flex-col shrink-0 sm:h-auto sm:w-64 sm:border-b-0 sm:border-r"
  >
    <div class="p-4 border-b border-line">
      <h2 class="text-body font-semibold text-fg-muted uppercase tracking-wide">
        {$t('chat.chats')}
      </h2>
    </div>
    {#if !isLoading}
      <ChatSessionList
        bind:sessions
        {activeSessionId}
        onsessionselect={(id) => (activeSessionId = id)}
        onsessionnew={handleNewSession}
        onlistchange={loadSessions}
      />
    {/if}
  </div>

  <!-- Main: chat thread -->
  <div class="flex-1 flex flex-col min-w-0">
    <div class="border-b border-line bg-surface px-3 py-2 sm:px-4">
      <div class="flex flex-wrap items-center justify-between gap-2">
        <div class="min-w-0">
          {#if contextPlan.selectedPatients.length}
            <p class="truncate text-caption text-fg-muted">
              {$t('chat.contextSummary').replace(
                '{patients}',
                contextPlan.selectedPatients
                  .map((patient) => `${patient.first_name} ${patient.last_name}`)
                  .join(', ')
              )}
            </p>
          {:else}
            <p class="text-caption text-fg-muted">{$t('chat.noContextAttached')}</p>
          {/if}
        </div>
        <button
          type="button"
          class="min-h-10 shrink-0 rounded-control border border-line px-3 text-body text-fg hover:bg-surface-hover"
          aria-expanded={showContextPicker}
          onclick={() => (showContextPicker = !showContextPicker)}
          >{contextPlan.selectedPatients.length
            ? $t('chat.changeContext')
            : $t('chat.attachContext')}</button
        >
      </div>
      {#if showContextPicker}
        <div class="mt-3">
          <ContextPicker bind:value={contextPlan} />
        </div>
      {/if}
    </div>
    {#if activeSessionId}
      {#key activeSessionId}
        <ChatThread
          sessionId={activeSessionId}
          {scope}
          patientId={patientId ?? undefined}
          initialMessage={starterPrompt}
          {contextPlan}
        />
      {/key}
    {:else if !isLoading}
      <div class="flex-1 flex items-center justify-center text-fg-subtle">
        <div class="text-center">
          <p class="text-heading mb-2">{$t('chat.noChat')}</p>
          <button
            onclick={handleNewSession}
            class="text-accent-fg hover:text-accent-fg underline text-body"
          >
            {$t('chat.startNewChat')}
          </button>
        </div>
      </div>
    {/if}
  </div>
</div>
