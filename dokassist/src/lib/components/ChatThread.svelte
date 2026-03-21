<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { listen, type UnlistenFn } from '@tauri-apps/api/event';
  import {
    getChatMessages,
    runAgentTurn,
    getEngineStatus,
    type ChatMessageRow,
  } from '$lib/api';
  import ChatMessage from './ChatMessage.svelte';
  import { goto } from '$app/navigation';
  import { AlertTriangle } from 'lucide-svelte';
  import { t } from '$lib/translations';

  interface Props {
    sessionId: string;
    scope: 'global' | 'patient';
    patientId?: string;
  }

  let { sessionId, scope: _scope, patientId: _patientId }: Props = $props();

  let messages = $state<ChatMessageRow[]>([]);
  let streamingContent = $state('');
  let isStreaming = $state(false);
  let inputText = $state('');
  let isModelLoaded = $state(true);
  let errorMessage = $state('');
  let messagesEndEl = $state<HTMLDivElement | null>(null);

  let unlistenChunk: UnlistenFn | null = null;
  let unlistenDone: UnlistenFn | null = null;
  let unlistenToolCalled: UnlistenFn | null = null;
  let unlistenError: UnlistenFn | null = null;
  let statusPollInterval: ReturnType<typeof setInterval> | null = null;

  async function loadMessages() {
    try {
      messages = await getChatMessages(sessionId);
    } catch (e) {
      console.error('Failed to load messages:', e);
    }
  }

  async function checkModelStatus() {
    try {
      const status = await getEngineStatus();
      isModelLoaded = status.is_loaded;
    } catch {
      // ignore
    }
  }

  function scrollToBottom() {
    messagesEndEl?.scrollIntoView({ behavior: 'smooth' });
  }

  async function handleSubmit() {
    const text = inputText.trim();
    if (!text || isStreaming || !isModelLoaded) return;

    inputText = '';
    isStreaming = true;
    streamingContent = '';
    errorMessage = '';

    // Optimistic user message
    const optimisticMsg: ChatMessageRow = {
      id: `optimistic-${Date.now()}`,
      session_id: sessionId,
      role: 'user',
      content: text,
      tool_name: null,
      tool_args_json: null,
      tool_result_for: null,
      created_at: new Date().toISOString(),
    };
    messages = [...messages, optimisticMsg];
    scrollToBottom();

    try {
      await runAgentTurn(sessionId, text);
      // agent-done triggers re-fetch via event listener
    } catch (e: unknown) {
      isStreaming = false;
      const msg = e instanceof Error ? e.message : String(e);
      errorMessage = `Fehler: ${msg}`;
      // Remove optimistic message on error
      messages = messages.filter((m) => m.id !== optimisticMsg.id);
    }
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      handleSubmit();
    }
  }

  onMount(async () => {
    await loadMessages();
    await checkModelStatus();
    scrollToBottom();

    unlistenChunk = await listen<string>('agent-chunk', (event) => {
      streamingContent += event.payload;
      scrollToBottom();
    });

    unlistenDone = await listen<{ final_answer: string }>('agent-done', async () => {
      isStreaming = false;
      streamingContent = '';
      await loadMessages();
      scrollToBottom();
    });

    unlistenToolCalled = await listen<{ name: string; args_json: string; result_json: string }>(
      'agent-tool-called',
      async () => {
        await loadMessages();
        scrollToBottom();
      },
    );

    unlistenError = await listen<{ message: string }>('agent-error', (event) => {
      isStreaming = false;
      streamingContent = '';
      errorMessage = event.payload.message;
    });

    statusPollInterval = setInterval(checkModelStatus, 5000);
  });

  onDestroy(() => {
    unlistenChunk?.();
    unlistenDone?.();
    unlistenToolCalled?.();
    unlistenError?.();
    if (statusPollInterval) clearInterval(statusPollInterval);
  });
</script>

<div class="flex flex-col h-full">
  {#if !isModelLoaded}
    <div class="bg-amber-50 dark:bg-amber-900/30 border-b border-amber-300 dark:border-amber-700 px-4 py-3 flex items-center gap-3">
      <AlertTriangle size={18} class="text-amber-600 dark:text-amber-400" />
      <p class="text-sm text-amber-700 dark:text-amber-300 flex-1">
        {$t('chat.noModelDesc')}
      </p>
      <button
        onclick={() => goto('/settings')}
        class="text-xs text-amber-600 dark:text-amber-400 underline hover:text-amber-700 dark:hover:text-amber-300"
      >
        {$t('chat.openSettings')}
      </button>
    </div>
  {/if}

  <!-- Message list -->
  <div class="flex-1 overflow-y-auto px-4 py-4 space-y-1">
    {#each messages as message (message.id)}
      <ChatMessage {message} />
    {/each}

    <!-- Streaming assistant message -->
    {#if isStreaming && streamingContent}
      <ChatMessage
        message={{
          id: 'streaming',
          session_id: sessionId,
          role: 'assistant',
          content: streamingContent,
          tool_name: null,
          tool_args_json: null,
          tool_result_for: null,
          created_at: new Date().toISOString(),
        }}
        isStreaming={true}
      />
    {:else if isStreaming && !streamingContent}
      <div class="flex justify-start mb-3">
        <div class="bg-gray-100 dark:bg-gray-800 border border-gray-200 dark:border-gray-700 rounded-2xl rounded-bl-sm px-4 py-2">
          <span class="animate-pulse text-gray-500 text-sm">●</span>
        </div>
      </div>
    {/if}

    {#if errorMessage}
      <div class="bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 rounded-lg px-4 py-3 text-sm text-red-600 dark:text-red-400">
        {errorMessage}
      </div>
    {/if}

    <div bind:this={messagesEndEl}></div>
  </div>

  <!-- Input area -->
  <div class="border-t border-gray-200 dark:border-gray-700 p-4">
    <div class="flex gap-2">
      <textarea
        bind:value={inputText}
        onkeydown={handleKeydown}
        disabled={!isModelLoaded || isStreaming}
        placeholder={isModelLoaded ? $t('chat.typeMessageHint') : $t('settings.modelNotLoaded')}
        rows={2}
        class="flex-1 bg-white dark:bg-gray-800 border border-gray-300 dark:border-gray-700 rounded-lg px-3 py-2 text-sm text-gray-900 dark:text-gray-100
               placeholder-gray-400 dark:placeholder-gray-500 resize-none focus:outline-none focus:border-blue-500
               disabled:opacity-50 disabled:cursor-not-allowed"
      ></textarea>
      <button
        onclick={handleSubmit}
        disabled={!isModelLoaded || isStreaming || !inputText.trim()}
        class="px-4 py-2 bg-blue-600 text-white rounded-lg text-sm font-medium
               hover:bg-blue-700 transition-colors disabled:opacity-50 disabled:cursor-not-allowed
               self-end"
      >
        {isStreaming ? '…' : 'Senden'}
      </button>
    </div>
  </div>
</div>
