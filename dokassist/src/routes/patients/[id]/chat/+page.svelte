<script lang="ts">
  import { onMount } from 'svelte';
  import { page } from '$app/stores';
  import { listChatSessions, createChatSession, getEngineStatus, loadModel, type ChatSession } from '$lib/api';
  import ChatSessionList from '$lib/components/ChatSessionList.svelte';
  import ChatThread from '$lib/components/ChatThread.svelte';

  let patientId = $derived($page.params.id);
  let sessions = $state<ChatSession[]>([]);
  let activeSessionId = $state<string | null>(null);
  let isLoading = $state(true);

  async function loadSessions() {
    try {
      sessions = await listChatSessions('patient', patientId);
      if (sessions.length > 0 && !activeSessionId) {
        activeSessionId = sessions[0].id;
      }
    } catch (e) {
      console.error('Failed to load chat sessions:', e);
    } finally {
      isLoading = false;
    }
  }

  async function handleNewSession() {
    try {
      const session = await createChatSession('patient', patientId, 'Neuer Chat');
      sessions = [session, ...sessions];
      activeSessionId = session.id;
    } catch (e) {
      console.error('Failed to create session:', e);
    }
  }

  onMount(async () => {
    loadSessions();
    try {
      const engineStatus = await getEngineStatus();
      if (engineStatus.is_downloaded && !engineStatus.is_loaded && engineStatus.downloaded_filename) {
        loadModel(engineStatus.downloaded_filename);
      }
    } catch {
      // ignore engine status errors
    }
  });
</script>

<div class="flex h-full">
  <!-- Sidebar: session list -->
  <div class="w-56 border-r border-gray-200 dark:border-gray-700 flex flex-col shrink-0">
    <div class="p-4 border-b border-gray-200 dark:border-gray-700">
      <h2 class="text-sm font-semibold text-gray-500 dark:text-gray-300 uppercase tracking-wide">
        Chats
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
    {#if activeSessionId}
      {#key activeSessionId}
        <ChatThread sessionId={activeSessionId} scope="patient" {patientId} />
      {/key}
    {:else if !isLoading}
      <div class="flex-1 flex items-center justify-center text-gray-400 dark:text-gray-500">
        <div class="text-center">
          <p class="text-lg mb-2">Kein Chat ausgewählt</p>
          <button
            onclick={handleNewSession}
            class="text-blue-500 dark:text-blue-400 hover:text-blue-600 dark:hover:text-blue-300 underline text-sm"
          >
            Neuen Chat starten
          </button>
        </div>
      </div>
    {/if}
  </div>
</div>
