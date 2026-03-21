<script lang="ts">
  import { page } from "$app/stores";
  import { goto } from "$app/navigation";
  import { onMount, onDestroy } from "svelte";
  import { listen, type UnlistenFn } from "@tauri-apps/api/event";
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
  } from "$lib/api";
  import ErrorDisplay from "$lib/components/ErrorDisplay.svelte";
  import { t } from "$lib/translations";

  $: patientId = $page.params.id;

  let patient: Patient | null = null;
  let recipientEmail = "";
  let subject = "";
  let body = "";
  let error: AppError | null = null;
  let isSaving = false;

  // AI Assist panel
  let showAiPanel = false;
  let aiPrompt = "";
  let isGenerating = false;
  let aiError = "";
  let engineStatus: LlmEngineStatus | null = null;

  let unlistenChunk: UnlistenFn | null = null;
  let unlistenDone: UnlistenFn | null = null;
  let unlistenError: UnlistenFn | null = null;

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
    aiError = "";
    body = "";

    try {
      const session = await createChatSession("patient", patientId, "Email Draft");
      const prompt = aiPrompt.trim() || `Write a professional email for this patient.`;
      await runAgentTurn(session.id, prompt);
    } catch (e) {
      isGenerating = false;
      aiError = e instanceof Error ? e.message : String(e);
    }
  }

  async function handleSaveDraft() {
    if (!recipientEmail.trim() || !subject.trim() || !body.trim()) {
      error = {
        code: "VALIDATION_ERROR",
        message: $t('email.validationError'),
        ref: "VALIDATION",
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
        code: "VALIDATION_ERROR",
        message: $t('email.validationError'),
        ref: "VALIDATION",
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

      const mailtoLink = encodeURI(`mailto:${recipientEmail}?subject=${encodeURIComponent(subject)}&body=${encodeURIComponent(body)}`);
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

    unlistenChunk = await listen<string>("agent-chunk", (event) => {
      body += event.payload;
    });

    unlistenDone = await listen("agent-done", () => {
      isGenerating = false;
    });

    unlistenError = await listen<{ message: string }>("agent-error", (event) => {
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
    <h2 class="text-2xl font-bold text-gray-900 dark:text-gray-100 mb-2">{$t('email.compose')}</h2>
    {#if patient}
      <p class="text-gray-500 dark:text-gray-400">
        {$t('email.forPatient')} {patient.first_name} {patient.last_name}
      </p>
    {/if}
  </div>

  {#if error}
    <div class="mb-6">
      <ErrorDisplay {error} showDetails={true} />
    </div>
  {/if}

  <div class="bg-gray-50 dark:bg-gray-800 rounded-lg p-6 border border-gray-200 dark:border-gray-700 space-y-4">
    <div>
      <label for="recipient" class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
        {$t('email.to')}
      </label>
      <input
        id="recipient"
        type="email"
        bind:value={recipientEmail}
        placeholder="recipient@example.com"
        class="w-full px-3 py-2 bg-white dark:bg-gray-900 border border-gray-300 dark:border-gray-700 rounded text-gray-900 dark:text-gray-100 placeholder-gray-400 dark:placeholder-gray-500 focus:outline-none focus:ring-2 focus:ring-blue-500"
      />
    </div>

    <div>
      <label for="subject" class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
        {$t('email.subject')}
      </label>
      <input
        id="subject"
        type="text"
        bind:value={subject}
        placeholder={$t('email.subjectPlaceholder')}
        class="w-full px-3 py-2 bg-white dark:bg-gray-900 border border-gray-300 dark:border-gray-700 rounded text-gray-900 dark:text-gray-100 placeholder-gray-400 dark:placeholder-gray-500 focus:outline-none focus:ring-2 focus:ring-blue-500"
      />
    </div>

    <div>
      <label for="body" class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
        {$t('email.message')}
      </label>
      <textarea
        id="body"
        bind:value={body}
        placeholder={$t('email.messagePlaceholder')}
        rows="15"
        class="w-full px-3 py-2 bg-white dark:bg-gray-900 border border-gray-300 dark:border-gray-700 rounded text-gray-900 dark:text-gray-100 placeholder-gray-400 dark:placeholder-gray-500 focus:outline-none focus:ring-2 focus:ring-blue-500 font-mono"
      ></textarea>
    </div>

    <div class="border-t border-gray-200 dark:border-gray-700 pt-4">
      <button
        on:click={() => (showAiPanel = !showAiPanel)}
        class="text-sm text-blue-600 dark:text-blue-400 hover:underline"
      >
        AI Assist
      </button>

      {#if showAiPanel}
        <div class="mt-3 space-y-3">
          {#if aiError}
            <p class="text-sm text-red-600 dark:text-red-400">{aiError}</p>
          {/if}

          <div>
            <label for="ai-prompt" class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
              What should this email say?
            </label>
            <textarea
              id="ai-prompt"
              bind:value={aiPrompt}
              rows="3"
              class="w-full px-3 py-2 bg-white dark:bg-gray-900 border border-gray-300 dark:border-gray-700 rounded text-gray-900 dark:text-gray-100 placeholder-gray-400 dark:placeholder-gray-500 focus:outline-none focus:ring-2 focus:ring-blue-500"
            ></textarea>
          </div>

          {#if engineStatus?.is_loaded}
            <button
              on:click={handleGenerateDraft}
              disabled={isGenerating}
              class="px-4 py-2 bg-purple-600 text-white rounded hover:bg-purple-700 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
            >
              {isGenerating ? "Generating…" : "Generate Draft"}
            </button>
          {:else}
            <p class="text-sm text-gray-500 dark:text-gray-400">No AI model is loaded</p>
          {/if}
        </div>
      {/if}
    </div>

    <div class="flex justify-between items-center pt-4 border-t border-gray-200 dark:border-gray-700">
      <a
        href={`/patients/${patientId}/email`}
        class="px-4 py-2 text-gray-500 dark:text-gray-400 hover:text-gray-700 dark:hover:text-gray-300 transition-colors"
      >
        {$t('common.cancel')}
      </a>
      <div class="flex space-x-3">
        <button
          on:click={handleSaveDraft}
          disabled={isSaving || isGenerating}
          class="px-4 py-2 bg-gray-200 dark:bg-gray-700 text-gray-700 dark:text-gray-300 rounded hover:bg-gray-300 dark:hover:bg-gray-600 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
        >
          {isSaving ? $t('email.saving') : $t('email.saveDraft')}
        </button>
        <button
          on:click={handleSendEmail}
          disabled={isSaving || isGenerating}
          class="px-4 py-2 bg-blue-600 text-white rounded hover:bg-blue-700 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
        >
          {isSaving ? $t('email.opening') : $t('email.openMailClient')}
        </button>
      </div>
    </div>
  </div>

  <div class="mt-4 text-sm text-gray-400 dark:text-gray-500">
    <p>{$t('email.mailClientHint')}</p>
  </div>
</div>
