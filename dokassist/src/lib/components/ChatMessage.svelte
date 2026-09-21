<script lang="ts">
  import type { ChatMessageRow } from '$lib/api';
  import { createReport, type CreateReport } from '$lib/api';
  import { t } from '$lib/translations';
  import { Wrench, Check } from 'lucide-svelte';
  import { ThinkingIndicator } from '$lib/components/ui';
  import { THINK_END, THINK_START, chatActivityStage } from '$lib/chat-activity';

  // Internal fields to hide from tool result display
  const HIDDEN_FIELDS = new Set([
    'id',
    'patient_id',
    'session_id',
    'created_at',
    'updated_at',
    'vault_path',
    'extracted_text',
    'metadata_json',
    'prompt_hash',
    'amdp_data',
  ]);

  function toLabel(key: string): string {
    return key.replace(/_/g, ' ').replace(/\b\w/g, (c) => c.toUpperCase());
  }

  function formatValue(val: unknown): string {
    if (val === null || val === undefined) return '—';
    if (typeof val === 'boolean') return val ? $t('common.yes') : $t('common.no');
    if (typeof val === 'string' && val.match(/^\d{4}-\d{2}-\d{2}/)) {
      return val.slice(0, 10).split('-').reverse().join('.');
    }
    return String(val);
  }

  type JsonObject = Record<string, unknown>;

  function parseToolResult(content: string): JsonObject | JsonObject[] | null {
    try {
      const parsed: unknown = JSON.parse(content);
      if (Array.isArray(parsed) || (typeof parsed === 'object' && parsed !== null)) {
        return parsed as JsonObject | JsonObject[];
      }
    } catch {
      // not JSON
    }
    return null;
  }

  function visibleEntries(obj: JsonObject): [string, unknown][] {
    return Object.entries(obj).filter(([k]) => !HIDDEN_FIELDS.has(k));
  }

  function reportDraftProposal(content: string): CreateReport | null {
    const parsed = parseToolResult(content);
    if (!parsed || Array.isArray(parsed)) return null;
    if (
      parsed.status !== 'pending_clinician_confirmation' ||
      parsed.action !== 'create_report' ||
      !parsed.proposal ||
      typeof parsed.proposal !== 'object' ||
      Array.isArray(parsed.proposal)
    ) {
      return null;
    }

    const proposal = parsed.proposal as Record<string, unknown>;
    if (
      typeof proposal.patient_id !== 'string' ||
      typeof proposal.report_type !== 'string' ||
      typeof proposal.content !== 'string'
    ) {
      return null;
    }

    return {
      patient_id: proposal.patient_id,
      report_type: proposal.report_type,
      content: proposal.content,
      model_name: typeof proposal.model_name === 'string' ? proposal.model_name : null,
      prompt_hash: typeof proposal.prompt_hash === 'string' ? proposal.prompt_hash : null,
      session_ids: typeof proposal.session_ids === 'string' ? proposal.session_ids : null,
    };
  }

  interface Props {
    message: ChatMessageRow;
    isStreaming?: boolean;
    activityStartedAt?: number;
    activeToolName?: string | null;
  }

  let { message, isStreaming = false, activityStartedAt, activeToolName = null }: Props = $props();

  let thinkContent = $derived(() => {
    if (!message.content.startsWith(THINK_START)) return '';
    const end = message.content.indexOf(THINK_END);
    return end !== -1
      ? message.content.slice(THINK_START.length, end).trim()
      : message.content.slice(THINK_START.length).trim();
  });

  let mainContent = $derived(() => {
    if (!message.content.startsWith(THINK_START)) return message.content;
    const end = message.content.indexOf(THINK_END);
    return end !== -1 ? message.content.slice(end + THINK_END.length).trim() : '';
  });

  let toolCallCollapsed = $state(true);
  let toolResultCollapsed = $state(true);
  let reviewedDraft = $state(false);
  let isSavingDraft = $state(false);
  let savedDraft = $state(false);
  let draftError = $state('');
  let fallbackStartedAt = $state(Date.now());
  let activityStage = $derived(
    chatActivityStage(message.content, isStreaming ? activeToolName : null)
  );
  let showActivity = $derived(isStreaming && activityStage !== 'writing');
  let startedAt = $derived(activityStartedAt ?? fallbackStartedAt);

  async function saveReviewedDraft(proposal: CreateReport) {
    if (!reviewedDraft || isSavingDraft || savedDraft) return;
    isSavingDraft = true;
    draftError = '';
    try {
      await createReport(proposal);
      savedDraft = true;
    } catch (error) {
      draftError = error instanceof Error ? error.message : String(error);
    } finally {
      isSavingDraft = false;
    }
  }
</script>

{#if message.role === 'user'}
  <div class="flex justify-end mb-3">
    <div
      class="max-w-[75%] bg-accent text-on-accent rounded-card px-4 py-2 text-body whitespace-pre-wrap"
    >
      {message.content}
    </div>
  </div>
{:else if message.role === 'assistant'}
  <div class="flex justify-start mb-3">
    <div class="max-w-[80%] space-y-2">
      {#if thinkContent()}
        <details class="bg-surface-hover border border-line rounded-card px-3 py-2">
          <summary class="text-caption text-fg-subtle uppercase tracking-wide cursor-pointer">
            {$t('chat.thinkingLabel')}
          </summary>
          <pre
            class="whitespace-pre-wrap font-sans text-caption text-fg-muted italic mt-1">{thinkContent()}</pre>
        </details>
      {/if}
      {#if mainContent().trim() || showActivity}
        <div
          class="bg-surface-hover border border-line rounded-card px-4 py-2 text-body text-fg whitespace-pre-wrap"
        >
          {#if mainContent().trim()}
            {mainContent()}
          {:else if showActivity}
            <ThinkingIndicator stage={activityStage} {startedAt} toolName={activeToolName} />
          {/if}
        </div>
      {/if}
    </div>
  </div>
{:else if message.role === 'tool_call'}
  <div class="flex justify-start mb-2">
    <div class="max-w-[80%]">
      <button
        onclick={() => (toolCallCollapsed = !toolCallCollapsed)}
        aria-label={toolCallCollapsed ? $t('chat.showToolCall') : $t('chat.hideToolCall')}
        class="flex items-center gap-2 text-caption text-fg-muted hover:text-fg-muted transition-colors"
      >
        <Wrench size={14} class="text-fg-subtle" />
        <span>{$t('chat.toolCall')}: {message.tool_name ?? 'unknown'}</span>
        <span>{toolCallCollapsed ? '▶' : '▼'}</span>
      </button>
      {#if !toolCallCollapsed}
        <div class="mt-1 bg-surface-hover border border-line rounded-card px-3 py-2">
          <pre
            class="text-caption text-fg-muted whitespace-pre-wrap overflow-x-auto">{message.tool_args_json ??
              message.content}</pre>
        </div>
      {/if}
    </div>
  </div>
{:else if message.role === 'tool_result'}
  {@const parsed = parseToolResult(message.content)}
  {@const reportDraft = reportDraftProposal(message.content)}
  <div class="flex justify-start mb-3">
    <div class="max-w-[80%]">
      <button
        onclick={() => (toolResultCollapsed = !toolResultCollapsed)}
        aria-label={toolResultCollapsed ? $t('chat.showToolResult') : $t('chat.hideToolResult')}
        class="flex items-center gap-2 text-caption text-fg-muted hover:text-fg-muted transition-colors"
      >
        <Check size={14} class="text-success-fg" />
        <span>{$t('chat.toolResult')}</span>
        <span>{toolResultCollapsed ? '▶' : '▼'}</span>
      </button>
      {#if !toolResultCollapsed}
        <div class="mt-1 bg-success-subtle border border-success-line rounded-card px-3 py-2">
          {#if parsed !== null}
            {#if Array.isArray(parsed)}
              <div class="space-y-2">
                {#each parsed as item, i}
                  <div
                    class="text-caption text-fg-muted border-b border-success-line pb-1 last:border-0 last:pb-0"
                  >
                    <span class="text-[10px] text-fg-muted uppercase">{i + 1}</span>
                    {#each visibleEntries(item) as [key, val]}
                      <div class="flex gap-2">
                        <span class="text-fg-muted shrink-0">{toLabel(key)}:</span>
                        <span class="text-fg">{formatValue(val)}</span>
                      </div>
                    {/each}
                  </div>
                {/each}
              </div>
            {:else}
              <div class="space-y-1">
                {#each visibleEntries(parsed) as [key, val]}
                  <div class="flex gap-2 text-caption">
                    <span class="text-fg-muted shrink-0">{toLabel(key)}:</span>
                    <span class="text-fg">{formatValue(val)}</span>
                  </div>
                {/each}
              </div>
            {/if}
          {:else}
            <pre
              class="text-caption text-fg-muted whitespace-pre-wrap overflow-x-auto">{message.content}</pre>
          {/if}
        </div>
      {/if}
      {#if reportDraft}
        <div class="mt-2 rounded-card border border-accent/30 bg-accent-subtle p-3 space-y-2">
          <p class="text-caption font-medium text-fg">{$t('chat.reportDraftUnsaved')}</p>
          <p class="text-caption text-fg-muted">
            {$t('chat.reportDraftReviewHint')}
          </p>
          <details class="text-caption text-fg-muted">
            <summary class="cursor-pointer text-fg">{$t('chat.reviewReportDraft')}</summary>
            <pre class="mt-2 whitespace-pre-wrap font-sans">{reportDraft.content}</pre>
          </details>
          {#if savedDraft}
            <p class="text-caption text-success-fg">{$t('chat.reportDraftSaved')}</p>
          {:else}
            <label class="flex items-start gap-2 text-caption text-fg-muted">
              <input type="checkbox" bind:checked={reviewedDraft} disabled={isSavingDraft} />
              <span>{$t('chat.confirmReportDraftReviewed')}</span>
            </label>
            {#if draftError}
              <p class="text-caption text-danger-fg">{draftError}</p>
            {/if}
            <button
              onclick={() => saveReviewedDraft(reportDraft)}
              disabled={!reviewedDraft || isSavingDraft}
              class="h-8 px-3 bg-accent text-on-accent rounded-control disabled:opacity-50 disabled:cursor-not-allowed"
            >
              {isSavingDraft ? $t('chat.savingReportDraft') : $t('chat.saveReviewedReportDraft')}
            </button>
          {/if}
        </div>
      {/if}
    </div>
  </div>
{/if}
