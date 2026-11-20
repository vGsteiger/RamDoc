<script lang="ts">
  import { onMount, onDestroy, tick, type Snippet } from 'svelte';
  import { listen, type UnlistenFn } from '@tauri-apps/api/event';
  import {
    getChatMessages,
    parseError,
    queryPatientHistory,
    runAgentTurn,
    type ChatMessageRow,
  } from '$lib/api';
  import ChatMessage from './ChatMessage.svelte';
  import ThinkingEffortSelect from './ThinkingEffortSelect.svelte';
  import DictationButton from './DictationButton.svelte';
  import RecordsTurnView from './chat/RecordsTurnView.svelte';
  import type { ChatMode, RecordsTurn } from './chat/patient-context';
  import { thinkingEffort } from '$lib/stores/thinking';
  import { engine, loadingModelLabel, refreshEngineStatus } from '$lib/stores/engine';
  import { Button, SegmentedControl, ThinkingIndicator } from '$lib/components/ui';
  import { get } from 'svelte/store';
  import { resolve } from '$app/paths';
  import { AlertTriangle, ArrowUp, Wrench } from 'lucide-svelte';
  import { t } from '$lib/translations';
  import { errorMessage as errorMessageLabel } from '$lib/translations/labels';
  import { chatToolActivityLabel } from '$lib/chat-activity';
  import { stripThinkTags } from '$lib/llm/strip-think';
  import { serializeContextPreamble, type ContextPlan } from './context-picker';
  import { evidenceForTurn, provenanceForAnswer } from '$lib/chat-provenance';

  interface Props {
    /**
     * The persisted chat session. `null` is a fresh chat: nothing is stored
     * until the first assistant message is sent, when `oncreatesession` is
     * called and the new id is written back through this binding.
     */
    sessionId: string | null;
    scope: 'global' | 'patient';
    patientId?: string;
    /** A route-supplied, editable starter prompt; it is never auto-submitted. */
    initialMessage?: string;
    /** Explicit, request-scoped context; it is not presented as durable session state. */
    contextPlan?: ContextPlan;
    /** Creates the session for a fresh chat from its first message and returns its id. */
    oncreatesession?: (firstMessage: string) => Promise<string>;
    /**
     * The single patient this chat is about. Enables the "Records only · cited"
     * mode, which answers from the audited patient-history pipeline.
     */
    recordsPatient?: { id: string; name: string } | null;
    /** The composer text, bindable so the page can prefill suggested prompts. */
    draft?: string;
    mode?: ChatMode;
    /** Shown in place of the conversation while it is empty (the Today panel). */
    empty?: Snippet;
    /** Chips and the attach action, rendered at the top of the composer. */
    contextBar?: Snippet;
  }

  let {
    sessionId = $bindable(),
    scope,
    patientId: _patientId,
    initialMessage = '',
    contextPlan = undefined,
    oncreatesession = undefined,
    recordsPatient = null,
    draft: inputText = $bindable(''),
    mode = $bindable('assistant'),
    empty,
    contextBar,
  }: Props = $props();

  let messages = $state<ChatMessageRow[]>([]);
  let recordTurns = $state<(RecordsTurn & { anchor: number })[]>([]);
  let streamingContent = $state('');
  let isStreaming = $state(false);
  let activityStartedAt = $state<number | null>(null);
  let activeToolName = $state<string | null>(null);
  let pendingTool = $state<{ name: string; startedAt: number } | null>(null);
  let isModelLoaded = $derived($engine.status ? $engine.status.is_loaded : true);
  let isLoadingModel = $derived($engine.isLoading);
  let modelName = $derived($engine.status?.model_name ?? '');
  let errorMessage = $state('');
  let messagesEndEl = $state<HTMLDivElement | null>(null);
  let composerEl = $state<HTMLTextAreaElement | null>(null);
  let recordsInFlight = $state(false);

  let unlistenChunk: UnlistenFn | null = null;
  let unlistenDone: UnlistenFn | null = null;
  let unlistenToolStarted: UnlistenFn | null = null;
  let unlistenToolCalled: UnlistenFn | null = null;
  let unlistenError: UnlistenFn | null = null;
  let unlistenRecordsChunk: UnlistenFn | null = null;
  /** True until `run_agent_turn` settles, including the post-loop persist emit. */
  let turnInFlight = $state(false);
  let requestContextPreamble = $derived(
    scope === 'global' && contextPlan ? serializeContextPreamble(contextPlan) : null
  );
  let recordsMode = $derived(mode === 'records' && !!recordsPatient);
  let busy = $derived(isStreaming || turnInFlight || recordsInFlight);
  let composerDisabled = $derived(!isModelLoaded || busy || isLoadingModel);
  let isEmpty = $derived(messages.length === 0 && recordTurns.length === 0 && !isStreaming);
  let modeOptions = $derived([
    { value: 'assistant', label: $t('chat.modeAssistant') },
    { value: 'records', label: $t('chat.modeRecords') },
  ]);

  $effect(() => {
    if (!recordsPatient && mode === 'records') mode = 'assistant';
  });

  function isThisSession(payload: { session_id?: string } | null | undefined): boolean {
    return !!sessionId && payload?.session_id === sessionId;
  }

  /** Records-only turns that sit before persisted message `index`. */
  function turnsBefore(index: number) {
    return recordTurns.filter((turn) => turn.anchor === index);
  }
  let trailingTurns = $derived(recordTurns.filter((turn) => turn.anchor >= messages.length));

  async function loadMessages() {
    if (!sessionId) {
      messages = [];
      return;
    }
    try {
      messages = await getChatMessages(sessionId);
    } catch (e) {
      console.error('Failed to load messages:', e);
    }
  }

  function applyInitialMessage() {
    if (initialMessage && !inputText && messages.length === 0) inputText = initialMessage;
  }

  $effect(() => {
    initialMessage;
    messages.length;
    inputText;
    applyInitialMessage();
  });

  function scrollToBottom() {
    messagesEndEl?.scrollIntoView({ behavior: 'smooth' });
  }

  /** Focus the composer, e.g. after the page prefilled a suggested prompt. */
  export async function focusComposer() {
    await tick();
    composerEl?.focus();
    const end = composerEl?.value.length ?? 0;
    composerEl?.setSelectionRange(end, end);
  }

  function describeError(e: unknown): string {
    return e instanceof Error
      ? e.message
      : typeof e === 'object' && e !== null && 'message' in e
        ? String((e as { message: unknown }).message)
        : String(e);
  }

  async function handleSubmit() {
    const text = inputText.trim();
    if (!text || busy || !isModelLoaded) return;
    if (recordsMode) {
      await submitRecordsQuestion(text);
      return;
    }

    inputText = '';
    isStreaming = true;
    turnInFlight = true;
    streamingContent = '';
    errorMessage = '';
    activityStartedAt = Date.now();
    activeToolName = null;
    pendingTool = null;

    let activeSessionId = sessionId;
    if (!activeSessionId) {
      try {
        if (!oncreatesession) throw new Error('No chat session');
        activeSessionId = await oncreatesession(text);
        sessionId = activeSessionId;
      } catch (e: unknown) {
        isStreaming = false;
        turnInFlight = false;
        activityStartedAt = null;
        inputText = text;
        errorMessage = `${$t('chat.error')}: ${describeError(e)}`;
        return;
      }
    }

    // Optimistic user message
    const optimisticMsg: ChatMessageRow = {
      id: `optimistic-${Date.now()}`,
      session_id: activeSessionId,
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
      await runAgentTurn(
        activeSessionId,
        requestContextPreamble ? `${requestContextPreamble}\n\n[User request]\n${text}` : text,
        get(thinkingEffort)
      );
      // agent-done triggers re-fetch via event listener
    } catch (e: unknown) {
      isStreaming = false;
      activityStartedAt = null;
      activeToolName = null;
      pendingTool = null;
      errorMessage = `${$t('chat.error')}: ${describeError(e)}`;
      // Remove optimistic message on error
      messages = messages.filter((m) => m.id !== optimisticMsg.id);
    } finally {
      turnInFlight = false;
    }
  }

  /**
   * D4a: answer from the patient's records through the audited
   * `query_patient_history` command, exactly as the overview's history query
   * did. The turn is shown in the thread but not persisted: there is no API
   * to append it to the chat session.
   */
  async function submitRecordsQuestion(text: string) {
    if (!recordsPatient) return;
    const id = `records-${Date.now()}`;
    inputText = '';
    errorMessage = '';
    recordsInFlight = true;
    recordTurns.push({
      id,
      patientId: recordsPatient.id,
      question: text,
      createdAt: new Date().toISOString(),
      answer: '',
      manifest: null,
      audit: null,
      error: '',
      pending: true,
      startedAt: Date.now(),
      anchor: messages.length,
    });
    const turn = recordTurns[recordTurns.length - 1];
    scrollToBottom();

    try {
      unlistenRecordsChunk?.();
      unlistenRecordsChunk = await listen<string>('patient-history-chunk', (event) => {
        turn.answer += event.payload;
        scrollToBottom();
      });
      const result = await queryPatientHistory(
        turn.patientId,
        text,
        undefined,
        get(thinkingEffort)
      );
      turn.answer = stripThinkTags(result.answer);
      turn.manifest = result.manifest;
      turn.audit = result.audit;
    } catch (e) {
      const appError = parseError(e);
      turn.error =
        $errorMessageLabel(appError.code, appError.message) +
        '\n\n' +
        $t('errors.referenceLine').replace('{ref}', appError.ref);
      console.error('Error querying patient history:', appError);
    } finally {
      turn.pending = false;
      recordsInFlight = false;
      unlistenRecordsChunk?.();
      unlistenRecordsChunk = null;
      scrollToBottom();
    }
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Enter' && !e.shiftKey && !e.isComposing) {
      e.preventDefault();
      handleSubmit();
    }
  }

  function appendTranscript(text: string) {
    inputText = inputText.trim() ? `${inputText.trimEnd()} ${text}` : text;
    void focusComposer();
  }

  onMount(async () => {
    await loadMessages();
    await refreshEngineStatus();
    applyInitialMessage();
    scrollToBottom();

    unlistenChunk = await listen<{ session_id: string; token: string }>(
      'agent-chunk-session',
      (event) => {
        if (!isThisSession(event.payload)) return;
        activeToolName = null;
        pendingTool = null;
        streamingContent += event.payload.token;
        scrollToBottom();
      }
    );

    unlistenDone = await listen<{ final_answer: string; session_id: string }>(
      'agent-done',
      async (event) => {
        if (!isThisSession(event.payload)) return;
        isStreaming = false;
        streamingContent = '';
        activityStartedAt = null;
        activeToolName = null;
        pendingTool = null;
        await loadMessages();
        scrollToBottom();
      }
    );

    unlistenToolStarted = await listen<{ name: string; args_json: string; session_id: string }>(
      'agent-tool-started',
      (event) => {
        if (!isThisSession(event.payload)) return;
        pendingTool = { name: event.payload.name, startedAt: Date.now() };
        activeToolName = event.payload.name;
        scrollToBottom();
      }
    );

    unlistenToolCalled = await listen<{
      name: string;
      args_json: string;
      result_json: string;
      session_id: string;
    }>('agent-tool-called', async (event) => {
      if (!isThisSession(event.payload)) return;
      pendingTool = null;
      activeToolName = event.payload.name;
      await loadMessages();
      scrollToBottom();
    });

    unlistenError = await listen<{ session_id: string; message: string }>(
      'agent-error-session',
      (event) => {
        if (!isThisSession(event.payload)) return;
        isStreaming = false;
        streamingContent = '';
        activityStartedAt = null;
        activeToolName = null;
        pendingTool = null;
        errorMessage = event.payload.message;
      }
    );
  });

  onDestroy(() => {
    unlistenChunk?.();
    unlistenDone?.();
    unlistenToolStarted?.();
    unlistenToolCalled?.();
    unlistenError?.();
    unlistenRecordsChunk?.();
  });
</script>

<div class="flex min-h-0 flex-1 flex-col">
  <!-- Conversation: scrolls on its own, centred in the chat measure. -->
  <div class="min-h-0 flex-1 overflow-y-auto" data-testid="chat-scroll">
    <div class="mx-auto flex min-h-full w-full max-w-chat flex-col px-4 pb-6 pt-4 sm:px-6">
      {#if isEmpty && empty}
        <div class="my-auto py-4">{@render empty()}</div>
      {/if}

      <div class="space-y-6">
        {#each messages as message, index (message.id)}
          {#each turnsBefore(index) as turn (turn.id)}
            <RecordsTurnView {turn} />
          {/each}
          <ChatMessage
            {message}
            provenance={message.role === 'assistant'
              ? provenanceForAnswer(messages, index, scope === 'patient')
              : undefined}
            draftEvidence={message.role === 'tool_result' ? evidenceForTurn(messages, index) : []}
          />
        {/each}
        {#each trailingTurns as turn (turn.id)}
          <RecordsTurnView {turn} />
        {/each}

        {#if pendingTool}
          <div class="flex items-center gap-2 text-label text-fg-muted">
            <Wrench size={14} class="shrink-0 text-fg-subtle" aria-hidden="true" />
            <ThinkingIndicator
              startedAt={pendingTool.startedAt}
              label={chatToolActivityLabel(pendingTool.name, $t, true)}
            />
          </div>
        {/if}

        <!-- Streaming assistant message -->
        {#if isStreaming && (streamingContent || !pendingTool)}
          <ChatMessage
            message={{
              id: 'streaming',
              session_id: sessionId ?? '',
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
            class="flex items-start gap-2 rounded-card border border-danger-line bg-danger-subtle px-3 py-2 text-label text-danger-fg"
            role="alert"
          >
            <AlertTriangle size={14} class="mt-0.5 shrink-0" aria-hidden="true" />
            <span class="min-w-0 break-words">{errorMessage}</span>
          </div>
        {/if}
      </div>

      <div bind:this={messagesEndEl}></div>
    </div>
  </div>

  <!-- Composer: pinned to the bottom; the context bar lives inside it. -->
  <div class="shrink-0 px-4 pb-4 sm:px-6">
    <div class="mx-auto w-full max-w-chat">
      {#if isLoadingModel && $engine.loadingStartedAt}
        <div class="mb-2 px-1">
          <ThinkingIndicator
            startedAt={$engine.loadingStartedAt}
            label={$t('chat.loadingModel').replace(
              '{name}',
              loadingModelLabel($engine.loadingFilename)
            )}
          />
        </div>
      {:else if !isModelLoaded}
        <div
          class="mb-2 flex flex-wrap items-center gap-x-3 gap-y-1 rounded-card border border-warning-line bg-warning-subtle px-3 py-2"
        >
          <AlertTriangle size={14} class="shrink-0 text-warning-fg" aria-hidden="true" />
          <p class="min-w-0 flex-1 text-label text-warning-fg">{$t('chat.noModelDesc')}</p>
          <Button size="sm" href={resolve('/settings')}>{$t('chat.openSettings')}</Button>
        </div>
      {/if}

      <div
        class="glass relative rounded-panel transition-shadow duration-150 ease-standard focus-within:ring-1 focus-within:ring-line-strong"
        data-testid="chat-composer"
      >
        {#if contextBar}
          <div class="flex min-w-0 flex-wrap items-center gap-1.5 px-3.5 pt-3.5">
            {@render contextBar()}
          </div>
        {/if}
        <!-- A borderless native textarea: the composer card carries the border
             and focus state, which the Textarea primitive would double. -->
        <textarea
          bind:this={composerEl}
          bind:value={inputText}
          onkeydown={handleKeydown}
          disabled={composerDisabled}
          aria-label={$t('chat.composerLabel')}
          placeholder={!isModelLoaded
            ? $t('settings.modelNotLoaded')
            : recordsMode
              ? $t('chat.recordsPlaceholder')
              : $t('chat.typeMessageHint')}
          rows={2}
          class="block max-h-60 min-h-16 w-full resize-none bg-transparent px-4 pt-3 text-body text-fg placeholder:text-fg-subtle focus:outline-none focus-visible:outline-none disabled:cursor-not-allowed disabled:opacity-60 [field-sizing:content]"
        ></textarea>
        <div class="flex flex-wrap items-center gap-2 px-3 pb-3">
          {#if recordsPatient}
            <SegmentedControl
              size="sm"
              options={modeOptions}
              bind:value={mode}
              label={$t('chat.mode')}
              data-testid="chat-mode"
            />
          {/if}
          <ThinkingEffortSelect size="sm" disabled={composerDisabled} />
          <div class="ml-auto flex shrink-0 items-center gap-1.5">
            {#if isModelLoaded && modelName}
              <span
                class="hidden max-w-[12rem] truncate text-caption text-fg-subtle lg:inline"
                title={modelName}>{modelName}</span
              >
            {/if}
            <DictationButton onTranscript={appendTranscript} disabled={composerDisabled} />
            <Button
              variant="primary"
              size="lg"
              class="w-control-lg px-0!"
              onclick={handleSubmit}
              disabled={composerDisabled || !inputText.trim()}
              aria-label={$t('chat.send')}
              title={$t('chat.send')}
            >
              <ArrowUp size={18} aria-hidden="true" />
            </Button>
          </div>
        </div>
      </div>

      {#if recordsMode && recordsPatient}
        <p class="mt-1.5 px-1 text-caption text-fg-subtle">
          {$t('chat.recordsHint').replace('{name}', recordsPatient.name)}
        </p>
      {:else if scope === 'global' && contextPlan?.selectedPatients.length}
        <p class="mt-1.5 px-1 text-caption text-fg-subtle">{$t('chat.contextWillBeAttached')}</p>
      {/if}
    </div>
  </div>
</div>
