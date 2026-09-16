<script lang="ts">
  import { errorText } from '$lib/translations/labels';
  import { get } from 'svelte/store';
  import { page } from '$app/stores';
  import { goto } from '$app/navigation';
  import { onMount, onDestroy } from 'svelte';
  import { listen, type UnlistenFn } from '@tauri-apps/api/event';
  import {
    getPatient,
    createEmail,
    markEmailAsSent,
    getEngineStatus,
    createChatSession,
    runAgentTurn,
    parseError,
    type CreateEmail,
    type Patient,
    type AppError,
    type LlmEngineStatus,
  } from '$lib/api';
  import { thinkingEffort } from '$lib/stores/thinking';
  import ErrorDisplay from '$lib/components/ErrorDisplay.svelte';
  import { t } from '$lib/translations';

  $: patientId = $page.params.id!;

  let patient: Patient | null = null;
  let recipientEmail = '';
  let subject = '';
  let body = '';
  let error: AppError | null = null;
  let isSaving = false;

  // AI Assist panel
  let showAiPanel = false;
  let aiPrompt = '';
  let isGenerating = false;
  let aiError = '';
  let aiDraft = '';
  let aiThinking = '';
  let rawDraft = '';
  let showThinking = false;
  let engineStatus: LlmEngineStatus | null = null;

  let unlistenChunk: UnlistenFn | null = null;
  let unlistenDone: UnlistenFn | null = null;
  let unlistenError: UnlistenFn | null = null;

  function parseRawDraft(raw: string): { thinking: string; draft: string } {
    const thinkMatches = [...raw.matchAll(/<think>([\s\S]*?)<\/think>/g)];
    const thinking = thinkMatches.map((m) => m[1].trim()).join('\n\n');
    const draft = raw.replace(/<think>[\s\S]*?<\/think>/g, '').trim();
    return { thinking, draft };
  }

  async function loadPatient() {
    try {
      patient = await getPatient(patientId);
      if (patient.email) {
        recipientEmail = patient.email;
      }
    } catch (e) {
      error = parseError(e);
    }
  }

  async function loadEngineStatus() {
    try {
      engineStatus = await getEngineStatus();
    } catch {
      // ignore
    }
  }

  async function handleGenerateDraft() {
    if (!engineStatus?.is_loaded || isGenerating) return;
    isGenerating = true;
    aiError = '';
    rawDraft = '';
    aiDraft = '';
    aiThinking = '';
    showThinking = false;

    try {
      const session = await createChatSession('patient', patientId, get(t)('chat.emailDraftTitle'));
      const userIntent =
        aiPrompt.trim() || 'Schreibe eine professionelle E-Mail für diesen Patienten.';
      const prompt = `Schreibe den Text einer E-Mail an den Patienten. Verwende KEIN Tool – gib nur den fertigen E-Mail-Text aus (ohne Betreff, nur den Nachrichtentext). Anweisung: ${userIntent}`;
      await runAgentTurn(session.id, prompt, get(thinkingEffort));
    } catch (e) {
      isGenerating = false;
      aiError = $errorText(e);
    }
  }

  async function handleSaveDraft() {
    if (!recipientEmail.trim() || !subject.trim() || !body.trim()) {
      error = {
        code: 'VALIDATION_ERROR',
        message: $t('email.validationError'),
        ref: 'VALIDATION',
      };
      return;
    }

    try {
      isSaving = true;
      error = null;

      const input: CreateEmail = {
        patient_id: patientId,
        recipient_email: recipientEmail,
        subject: subject,
        body: body,
      };

      await createEmail(input);
      await goto(`/patients/${patientId}/email`);
    } catch (e) {
      error = parseError(e);
    } finally {
      isSaving = false;
    }
  }

  async function handleSendEmail() {
    if (!recipientEmail.trim() || !subject.trim() || !body.trim()) {
      error = {
        code: 'VALIDATION_ERROR',
        message: $t('email.validationError'),
        ref: 'VALIDATION',
      };
      return;
    }

    try {
      isSaving = true;
      error = null;

      const input: CreateEmail = {
        patient_id: patientId,
        recipient_email: recipientEmail,
        subject: subject,
        body: body,
      };

      const savedEmail = await createEmail(input);
      await markEmailAsSent(savedEmail.id);

      const mailtoLink = encodeURI(
        `mailto:${recipientEmail}?subject=${encodeURIComponent(subject)}&body=${encodeURIComponent(body)}`
      );
      window.location.href = mailtoLink;

      setTimeout(() => {
        goto(`/patients/${patientId}/email`);
      }, 500);
    } catch (e) {
      error = parseError(e);
    } finally {
      isSaving = false;
    }
  }

  onMount(async () => {
    await loadPatient();
    await loadEngineStatus();

    unlistenChunk = await listen<string>('agent-chunk', (event) => {
      rawDraft += event.payload;
      const parsed = parseRawDraft(rawDraft);
      aiDraft = parsed.draft;
      aiThinking = parsed.thinking;
    });

    unlistenDone = await listen('agent-done', () => {
      isGenerating = false;
      const parsed = parseRawDraft(rawDraft);
      aiDraft = parsed.draft;
      aiThinking = parsed.thinking;
    });

    unlistenError = await listen<{ message: string }>('agent-error', (event) => {
      isGenerating = false;
      aiError = event.payload?.message ?? String(event.payload);
    });
  });

  onDestroy(() => {
    unlistenChunk?.();
    unlistenDone?.();
    unlistenError?.();
  });
</script>

<div class="p-8 max-w-4xl mx-auto">
  <div class="mb-6">
    <h2 class="text-display font-semibold text-fg mb-2">{$t('email.compose')}</h2>
    {#if patient}
      <p class="text-fg-muted">
        {$t('email.forPatient')}
        {patient.first_name}
        {patient.last_name}
      </p>
    {/if}
  </div>

  {#if error}
    <div class="mb-6">
      <ErrorDisplay {error} showDetails={true} />
    </div>
  {/if}

  <div class="bg-surface-sunken rounded-card p-6 border border-line space-y-4">
    <div>
      <label for="recipient" class="block text-body font-medium text-fg-muted mb-2">
        {$t('email.to')}
      </label>
      <input
        id="recipient"
        type="email"
        bind:value={recipientEmail}
        placeholder="recipient@example.com"
        class="w-full px-3 py-2 bg-surface-raised border border-line rounded-control text-fg focus:outline-none focus:ring-2 focus:ring-accent/30"
      />
    </div>

    <div>
      <label for="subject" class="block text-body font-medium text-fg-muted mb-2">
        {$t('email.subject')}
      </label>
      <input
        id="subject"
        type="text"
        bind:value={subject}
        placeholder={$t('email.subjectPlaceholder')}
        class="w-full px-3 py-2 bg-surface-raised border border-line rounded-control text-fg focus:outline-none focus:ring-2 focus:ring-accent/30"
      />
    </div>

    <div>
      <label for="body" class="block text-body font-medium text-fg-muted mb-2">
        {$t('email.message')}
      </label>
      <textarea
        id="body"
        bind:value={body}
        placeholder={$t('email.messagePlaceholder')}
        rows="15"
        class="w-full px-3 py-2 bg-surface-raised border border-line rounded-control text-fg focus:outline-none focus:ring-2 focus:ring-accent/30 font-mono"
      ></textarea>
    </div>

    <div class="border-t border-line pt-4">
      <button
        on:click={() => (showAiPanel = !showAiPanel)}
        class="text-body text-accent-fg hover:underline"
      >
        {$t('email.aiAssist')}
      </button>

      {#if showAiPanel}
        <div class="mt-3 space-y-3">
          {#if aiError}
            <p class="text-body text-danger-fg">{aiError}</p>
          {/if}

          <div>
            <label for="ai-prompt" class="block text-body font-medium text-fg-muted mb-1">
              {$t('email.aiPromptLabel')}
            </label>
            <textarea
              id="ai-prompt"
              bind:value={aiPrompt}
              rows="3"
              class="w-full px-3 py-2 bg-surface-raised border border-line rounded-control text-fg focus:outline-none focus:ring-2 focus:ring-accent/30"
            ></textarea>
          </div>

          {#if engineStatus?.is_loaded}
            <button
              on:click={handleGenerateDraft}
              disabled={isGenerating}
              class="h-8 px-3 bg-accent text-on-accent rounded-control hover:bg-accent-hover transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
            >
              {isGenerating ? $t('email.generating') : $t('email.generateDraft')}
            </button>
          {:else}
            <p class="text-body text-fg-muted">{$t('email.modelNotLoaded')}</p>
          {/if}

          {#if aiDraft || isGenerating}
            <div class="mt-3 space-y-2">
              <div class="flex items-center justify-between">
                <span class="text-body font-medium text-fg-muted">{$t('email.generatedDraft')}</span
                >
                {#if aiDraft && !isGenerating}
                  <button
                    on:click={() => {
                      body = aiDraft;
                      aiDraft = '';
                      aiThinking = '';
                      rawDraft = '';
                    }}
                    class="text-body h-7 px-2.5 bg-accent text-on-accent rounded-control hover:bg-accent-hover transition-colors"
                  >
                    {$t('email.applyToBody')}
                  </button>
                {/if}
              </div>
              <textarea
                readonly
                value={aiDraft}
                rows="8"
                class="w-full px-3 py-2 bg-surface-raised border border-line rounded-control text-fg font-mono text-body focus:outline-none"
              ></textarea>
            </div>
          {/if}

          {#if aiThinking}
            <div class="mt-2">
              <button
                on:click={() => (showThinking = !showThinking)}
                class="text-caption text-fg-subtle hover:underline"
              >
                {showThinking ? $t('email.hideReasoning') : $t('email.showReasoning')} ({aiThinking.split(
                  '\n'
                ).length}
                {$t('email.reasoningLines')})
              </button>
              {#if showThinking}
                <pre
                  class="mt-2 p-3 bg-surface-sunken border border-line rounded-card text-caption text-fg-muted whitespace-pre-wrap overflow-auto max-h-48">{aiThinking}</pre>
              {/if}
            </div>
          {/if}
        </div>
      {/if}
    </div>

    <div class="flex justify-between items-center pt-4 border-t border-line">
      <a
        href={`/patients/${patientId}/email`}
        class="px-4 py-2 text-fg-muted hover:text-fg transition-colors"
      >
        {$t('common.cancel')}
      </a>
      <div class="flex space-x-3">
        <button
          on:click={handleSaveDraft}
          disabled={isSaving || isGenerating}
          class="h-8 px-3 bg-surface-selected text-fg-muted rounded-control hover:bg-surface-selected transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
        >
          {isSaving ? $t('email.saving') : $t('email.saveDraft')}
        </button>
        <button
          on:click={handleSendEmail}
          disabled={isSaving || isGenerating}
          class="h-8 px-3 bg-accent text-on-accent rounded-control hover:bg-accent-hover transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
        >
          {isSaving ? $t('email.opening') : $t('email.openMailClient')}
        </button>
      </div>
    </div>
  </div>

  <div class="mt-4 text-body text-fg-subtle">
    <p>{$t('email.mailClientHint')}</p>
  </div>
</div>
