<script lang="ts">
  import { get } from 'svelte/store';
  import { onMount } from 'svelte';
  import { page } from '$app/stores';
  import {
    listChatSessions,
    createChatSession,
    getOrCreatePatientChatSession,
    type ChatSession,
  } from '$lib/api';
  import { ensureEngineLoaded } from '$lib/stores/engine';
  import ChatSessionList from '$lib/components/ChatSessionList.svelte';
  import ChatThread from '$lib/components/ChatThread.svelte';
  import { t } from '$lib/translations';

  let sessions = $state<ChatSession[]>([]);
  let activeSessionId = $state<string | null>(null);
  let isLoading = $state(true);
  let patientId = $derived($page.url.searchParams.get('patientId'));
  let intent = $derived($page.url.searchParams.get('intent'));
  let scope = $derived<'global' | 'patient'>(patientId ? 'patient' : 'global');
  let starterPrompt = $derived(intent === 'report' ? get(t)('chat.reportStarterPrompt') : '');

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
    loadSessions();
    void ensureEngineLoaded();
  });
</script>

<div class="flex h-full">
  <!-- Sidebar: session list -->
  <div class="w-64 border-r border-line flex flex-col shrink-0">
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
    {#if patientId}
      <div class="border-b border-line bg-surface-hover px-4 py-2 text-caption text-fg-muted">
        {$t('chat.patientContextAttached')}
      </div>
    {/if}
    {#if activeSessionId}
      {#key activeSessionId}
        <ChatThread
          sessionId={activeSessionId}
          {scope}
          patientId={patientId ?? undefined}
          initialMessage={starterPrompt}
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
