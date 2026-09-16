<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { listen, type UnlistenFn } from '@tauri-apps/api/event';
  import { getChatMessages, runAgentTurn, type ChatMessageRow } from '$lib/api';
  import ChatMessage from './ChatMessage.svelte';
  import ThinkingEffortSelect from './ThinkingEffortSelect.svelte';
  import { thinkingEffort } from '$lib/stores/thinking';
  import { engine, loadingModelLabel, refreshEngineStatus } from '$lib/stores/engine';
  import { ThinkingIndicator } from '$lib/components/ui';
  import { get } from 'svelte/store';
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
  let activityStartedAt = $state<number | null>(null);
  let activeToolName = $state<string | null>(null);
  let inputText = $state('');
  let isModelLoaded = $derived($engine.status ? $engine.status.is_loaded : true);
  let isLoadingModel = $derived($engine.isLoading);
  let modelName = $derived($engine.status?.model_name ?? '');
  let errorMessage = $state('');
  let messagesEndEl = $state<HTMLDivElement | null>(null);

  let unlistenChunk: UnlistenFn | null = null;
  let unlistenDone: UnlistenFn | null = null;
  let unlistenToolCalled: UnlistenFn | null = null;
  let unlistenError: UnlistenFn | null = null;

  async function loadMessages() {
    try {
      messages = await getChatMessages(sessionId);
    } catch (e) {
      console.error('Failed to load messages:', e);
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
    activityStartedAt = Date.now();
    activeToolName = null;

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
      await runAgentTurn(sessionId, text, get(thinkingEffort));
      // agent-done triggers re-fetch via event listener
    } catch (e: unknown) {
      isStreaming = false;
      activityStartedAt = null;
      activeToolName = null;
      const msg =
        e instanceof Error
          ? e.message
          : typeof e === 'object' && e !== null && 'message' in e
            ? String((e as { message: unknown }).message)
            : String(e);
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
    await refreshEngineStatus();
    scrollToBottom();

    unlistenChunk = await listen<string>('agent-chunk', (event) => {
      activeToolName = null;
      streamingContent += event.payload;
      scrollToBottom();
    });

    unlistenDone = await listen<{ final_answer: string }>('agent-done', async () => {
      isStreaming = false;
      streamingContent = '';
      activityStartedAt = null;
      activeToolName = null;
      await loadMessages();
      scrollToBottom();
    });

    unlistenToolCalled = await listen<{ name: string; args_json: string; result_json: string }>(
      'agent-tool-called',
      async (event) => {
        activeToolName = event.payload.name;
        await loadMessages();
        scrollToBottom();
      }
    );

    unlistenError = await listen<{ message: string }>('agent-error', (event) => {
      isStreaming = false;
      streamingContent = '';
      activityStartedAt = null;
      activeToolName = null;
      errorMessage = event.payload.message;
    });
  });

  onDestroy(() => {
    unlistenChunk?.();
    unlistenDone?.();
    unlistenToolCalled?.();
    unlistenError?.();
  });
</script>

<div class="flex flex-col h-full">
  {#if isLoadingModel && $engine.loadingStartedAt}
    <div class="bg-warning-subtle border-b border-warning-line px-4 py-3">
      <ThinkingIndicator
        startedAt={$engine.loadingStartedAt}
        label={$t('chat.loadingModel').replace(
          '{name}',
          loadingModelLabel($engine.loadingFilename)
        )}
      />
    </div>
  {:else if !isModelLoaded}
    <div class="bg-warning-subtle border-b border-warning-line px-4 py-3 flex items-center gap-3">
      <AlertTriangle size={18} class="text-warning-fg" />
      <p class="text-body text-warning-fg flex-1">
        {$t('chat.noModelDesc')}
      </p>
      <button
        onclick={() => goto('/settings')}
        class="text-caption text-warning-fg underline hover:text-warning-fg"
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
    {#if isStreaming}
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
        activityStartedAt={activityStartedAt ?? undefined}
        {activeToolName}
      />
    {/if}

    {#if errorMessage}
      <div
        class="bg-danger-subtle border border-danger-line rounded-card px-4 py-3 text-body text-danger-fg"
      >
        {errorMessage}
      </div>
    {/if}

    <div bind:this={messagesEndEl}></div>
  </div>

  <!-- Input area -->
  <div class="border-t border-line p-4 space-y-2">
    <div class="flex gap-2">
      <textarea
        bind:value={inputText}
        onkeydown={handleKeydown}
        disabled={!isModelLoaded || isStreaming || isLoadingModel}
        placeholder={isModelLoaded ? $t('chat.typeMessageHint') : $t('settings.modelNotLoaded')}
        rows={2}
        class="flex-1 bg-surface-raised border border-line rounded-control px-3 py-2 text-body text-fg
 resize-none focus:outline-none focus:border-accent
 disabled:opacity-50 disabled:cursor-not-allowed"></textarea>
      <button
        onclick={handleSubmit}
        disabled={!isModelLoaded || isStreaming || isLoadingModel || !inputText.trim()}
        class="h-8 px-3 bg-accent text-on-accent rounded-control text-body font-medium
 hover:bg-accent-hover transition-colors disabled:opacity-50 disabled:cursor-not-allowed
 self-end"
      >
        {isStreaming ? '…' : $t('chat.send')}
      </button>
    </div>
    <div class="flex flex-wrap items-center justify-between gap-2">
      <ThinkingEffortSelect disabled={!isModelLoaded || isStreaming || isLoadingModel} />
      {#if isModelLoaded && modelName}
        <span class="text-caption text-fg-subtle truncate max-w-[16rem]" title={modelName}
          >{modelName}</span
        >
      {/if}
    </div>
  </div>
</div>
