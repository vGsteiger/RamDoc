<script lang="ts">
  import { errorText } from '$lib/translations/labels';
  import { get } from 'svelte/store';
  import { page } from '$app/state';
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
  import {
    Alert,
    Button,
    Field,
    Input,
    PageShell,
    StatusIndicator,
    Textarea,
    ThinkingIndicator,
  } from '$lib/components/ui';
  import { ArrowLeft, ChevronDown, ChevronRight } from 'lucide-svelte';
  import StickyActionBar from '$lib/components/documents/StickyActionBar.svelte';
  import { documentsUrl } from '$lib/components/documents/documents';
  import { t } from '$lib/translations';

  let patientId = $derived(page.params.id!);

  let patient = $state<Patient | null>(null);
  let recipientEmail = $state('');
  let subject = $state('');
  let body = $state('');
  let error: AppError | null = $state(null);
  let isSaving = $state(false);

  // AI Assist panel
  let showAiPanel = $state(false);
  let aiPrompt = $state('');
  let isGenerating = $state(false);
  let activityStartedAt: number | null = $state(null);
  let aiError = $state('');
  let aiDraft = $state('');
  let aiThinking = $state('');
  // Accumulates streamed chunks; only its parsed results are rendered.
  let rawDraft = '';
  let showThinking = $state(false);
  let engineStatus: LlmEngineStatus | null = $state(null);

  // Listener handles are not rendered, so they stay plain variables.
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
    activityStartedAt = Date.now();
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
      activityStartedAt = null;
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
      await goto(documentsUrl(patientId, 'emails'));
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
        goto(documentsUrl(patientId, 'emails'));
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
      activityStartedAt = null;
      const parsed = parseRawDraft(rawDraft);
      aiDraft = parsed.draft;
      aiThinking = parsed.thinking;
    });

    unlistenError = await listen<{ message: string }>('agent-error', (event) => {
      isGenerating = false;
      activityStartedAt = null;
      aiError = event.payload?.message ?? String(event.payload);
    });
  });

  onDestroy(() => {
    unlistenChunk?.();
    unlistenDone?.();
    unlistenError?.();
  });

  function applyDraft() {
    body = aiDraft;
    aiDraft = '';
    aiThinking = '';
    rawDraft = '';
  }

  let patientLine = $derived(
    patient ? `${$t('email.forPatient')} ${patient.first_name} ${patient.last_name}` : undefined
  );
</script>

<PageShell width="prose" title={$t('email.compose')} description={patientLine}>
  {#snippet breadcrumb()}
    <a
      href={documentsUrl(patientId, 'emails')}
      class="inline-flex items-center gap-1 text-fg-subtle hover:text-fg"
    >
      <ArrowLeft size={12} aria-hidden="true" />
      {$t('email.backToDocuments')}
    </a>
  {/snippet}

  {#if error}
    <div class="mb-4">
      <ErrorDisplay {error} showDetails={true} />
    </div>
  {/if}

  <div class="space-y-4">
    <Field label={$t('email.to')} for="recipient">
      <Input
        id="recipient"
        type="email"
        bind:value={recipientEmail}
        placeholder="recipient@example.com"
      />
    </Field>

    <Field label={$t('email.subject')} for="subject">
      <Input id="subject" bind:value={subject} placeholder={$t('email.subjectPlaceholder')} />
    </Field>

    <Field label={$t('email.message')} for="body">
      <Textarea
        id="body"
        bind:value={body}
        placeholder={$t('email.messagePlaceholder')}
        rows={15}
        class="leading-relaxed"
      />
    </Field>

    <section class="rounded-card border border-line">
      <Button
        variant="ghost"
        full
        class="justify-start rounded-card"
        aria-expanded={showAiPanel}
        aria-controls="email-ai-panel"
        onclick={() => (showAiPanel = !showAiPanel)}
      >
        {#if showAiPanel}
          <ChevronDown size={14} aria-hidden="true" />
        {:else}
          <ChevronRight size={14} aria-hidden="true" />
        {/if}
        {$t('email.aiAssist')}
      </Button>

      {#if showAiPanel}
        <div id="email-ai-panel" class="space-y-3 border-t border-line-subtle p-4">
          {#if aiError}
            <Alert tone="danger">{aiError}</Alert>
          {/if}

          <Field label={$t('email.aiPromptLabel')} for="ai-prompt">
            <Textarea id="ai-prompt" bind:value={aiPrompt} rows={3} />
          </Field>

          {#if engineStatus?.is_loaded}
            <Button size="sm" onclick={handleGenerateDraft} disabled={isGenerating}>
              {isGenerating ? $t('email.generating') : $t('email.generateDraft')}
            </Button>
          {:else}
            <StatusIndicator tone="warn" label={$t('email.modelNotLoaded')} />
          {/if}

          {#if aiDraft || isGenerating}
            <div class="space-y-2">
              <div class="flex flex-wrap items-center justify-between gap-2">
                <span class="text-label font-medium text-fg-muted"
                  >{$t('email.generatedDraft')}</span
                >
                {#if isGenerating && activityStartedAt}
                  <span role="status" aria-live="polite">
                    <ThinkingIndicator
                      startedAt={activityStartedAt}
                      label={$t('email.activityWriting')}
                    />
                  </span>
                {/if}
                {#if aiDraft && !isGenerating}
                  <Button size="sm" onclick={applyDraft}>{$t('email.applyToBody')}</Button>
                {/if}
              </div>
              <Textarea
                readonly
                value={aiDraft}
                rows={8}
                class="bg-surface-sunken"
                aria-label={$t('email.generatedDraft')}
              />
            </div>
          {/if}

          {#if aiThinking}
            <div>
              <Button
                variant="ghost"
                size="sm"
                class="-ml-2.5"
                aria-expanded={showThinking}
                onclick={() => (showThinking = !showThinking)}
              >
                {showThinking ? $t('email.hideReasoning') : $t('email.showReasoning')} ({aiThinking.split(
                  '\n'
                ).length}
                {$t('email.reasoningLines')})
              </Button>
              {#if showThinking}
                <pre
                  class="mt-2 max-h-48 overflow-auto whitespace-pre-wrap rounded-card border border-line bg-surface-sunken p-3 text-caption text-fg-muted">{aiThinking}</pre>
              {/if}
            </div>
          {/if}
        </div>
      {/if}
    </section>

    <p class="text-label text-fg-subtle">{$t('email.mailClientHint')}</p>
  </div>

  <StickyActionBar>
    {#snippet start()}
      <Button variant="ghost" href={documentsUrl(patientId, 'emails')}>
        {$t('common.cancel')}
      </Button>
    {/snippet}
    <Button onclick={handleSaveDraft} disabled={isSaving || isGenerating}>
      {isSaving ? $t('email.saving') : $t('email.saveDraft')}
    </Button>
    <Button variant="primary" onclick={handleSendEmail} disabled={isSaving || isGenerating}>
      {isSaving ? $t('email.opening') : $t('email.openMailClient')}
    </Button>
  </StickyActionBar>
</PageShell>
