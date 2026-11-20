<script lang="ts">
  import { onMount } from 'svelte';
  import type { ChatDraftVersion, ChatMessageRow } from '$lib/api';
  import {
    appendChatDraftVersion,
    createReport,
    listChatDraftVersions,
    type CreateReport,
  } from '$lib/api';
  import {
    type ChatProvenance,
    type ClaimResolution,
    type ProvenanceSource,
    unsupportedDraftClaims,
    unresolvedClaims,
  } from '$lib/chat-provenance';
  import { t } from '$lib/translations';
  import { AlertTriangle, Check, ChevronRight, Wrench } from 'lucide-svelte';
  import { Button, Markdown, Select, Textarea, ThinkingIndicator } from '$lib/components/ui';
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

  function writeReportDraftProposal(message: ChatMessageRow): CreateReport | null {
    return message.tool_name === 'write_report' ? reportDraftProposal(message.content) : null;
  }

  interface Props {
    message: ChatMessageRow;
    isStreaming?: boolean;
    activityStartedAt?: number;
    activeToolName?: string | null;
    /** Provenance assembled from persisted typed tool results in this turn. */
    provenance?: ChatProvenance;
    /** Read sources preceding this report proposal; never assistant citation text. */
    draftEvidence?: ProvenanceSource[];
  }

  let {
    message,
    isStreaming = false,
    activityStartedAt,
    activeToolName = null,
    provenance = undefined,
    draftEvidence = [],
  }: Props = $props();

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
  let draftVersionsLoaded = $state(false);
  let draftVersions = $state<ChatDraftVersion[]>([]);
  let draftContent = $state('');
  let claimResolutions = $state<Record<string, ClaimResolution | ''>>({});
  let isSavingRevision = $state(false);
  let fallbackStartedAt = $state(Date.now());
  let activityStage = $derived(
    chatActivityStage(message.content, isStreaming ? activeToolName : null)
  );
  let showActivity = $derived(isStreaming && activityStage !== 'writing');
  let startedAt = $derived(activityStartedAt ?? fallbackStartedAt);
  let latestDraftVersion = $derived(draftVersions.at(-1) ?? null);
  let draftClaims = $derived(
    writeReportDraftProposal(message) ? unsupportedDraftClaims(draftContent, draftEvidence) : []
  );
  let unresolvedDraftClaims = $derived(unresolvedClaims(draftClaims, claimResolutions));
  let removedClaimsStillPresent = $derived(
    draftClaims.some(
      (claim) =>
        claim.citation &&
        claimResolutions[claim.id] === 'removed' &&
        draftContent.includes(claim.citation)
    )
  );
  let canSaveDraft = $derived(
    reviewedDraft &&
      !isSavingDraft &&
      draftVersionsLoaded &&
      !draftError &&
      unresolvedDraftClaims.length === 0 &&
      !removedClaimsStillPresent
  );

  async function saveReviewedDraft(proposal: CreateReport) {
    if (!canSaveDraft || savedDraft) return;
    isSavingDraft = true;
    draftError = '';
    try {
      // The acknowledgement becomes an immutable manual revision before the
      // report write. It gives a later reviewer the exact text and the
      // clinician's resolution choices without trusting model citations.
      const version = await appendChatDraftVersion({
        tool_result_message_id: message.id,
        content: draftContent,
        origin: 'manual',
        claim_resolutions_json: JSON.stringify(claimResolutions),
      });
      draftVersions = [...draftVersions, version];
      await createReport({ ...proposal, content: draftContent });
      savedDraft = true;
    } catch (error) {
      draftError = error instanceof Error ? error.message : String(error);
    } finally {
      isSavingDraft = false;
    }
  }

  async function loadDraftVersions() {
    const proposal = writeReportDraftProposal(message);
    if (!proposal) return;
    try {
      const loaded = await listChatDraftVersions(message.id);
      draftVersions = Array.isArray(loaded) ? loaded : [];
      draftContent = draftVersions.at(-1)?.content ?? proposal.content;
      draftVersionsLoaded = true;
    } catch (error) {
      // An initial version is persisted with the tool result. A failed read
      // must not turn the original proposal into a silently-saveable draft.
      draftContent = proposal.content;
      draftError = error instanceof Error ? error.message : String(error);
      draftVersionsLoaded = false;
    }
  }

  async function saveManualRevision() {
    if (isSavingRevision || !draftContent.trim()) return;
    isSavingRevision = true;
    draftError = '';
    try {
      const version = await appendChatDraftVersion({
        tool_result_message_id: message.id,
        content: draftContent,
        origin: 'manual',
        claim_resolutions_json: JSON.stringify(claimResolutions),
      });
      draftVersions = [...draftVersions, version];
    } catch (error) {
      draftError = error instanceof Error ? error.message : String(error);
    } finally {
      isSavingRevision = false;
    }
  }

  function restoreDraftVersion(version: ChatDraftVersion) {
    draftContent = version.content;
    try {
      const saved: unknown = JSON.parse(version.claim_resolutions_json);
      claimResolutions =
        saved && typeof saved === 'object' && !Array.isArray(saved)
          ? (saved as Record<string, ClaimResolution | ''>)
          : {};
    } catch {
      claimResolutions = {};
    }
    reviewedDraft = false;
    savedDraft = false;
  }

  function claimLabel(claim: (typeof draftClaims)[number]) {
    return claim.kind === 'unverified_citation'
      ? $t('chat.unverifiedCitationClaim').replace('{citation}', claim.citation ?? '')
      : $t('chat.missingTypedSourceClaim');
  }

  function sourceLabel(source: ProvenanceSource) {
    return $t(`chat.provenanceSource.${source.toolName}`);
  }

  onMount(() => {
    void loadDraftVersions();
  });
</script>

{#if message.role === 'user'}
  <div class="flex justify-end">
    <div
      class="min-w-0 max-w-[85%] whitespace-pre-wrap break-words rounded-panel bg-fill px-4 py-2.5 text-body text-fg"
    >
      {message.content}
    </div>
  </div>
{:else if message.role === 'assistant'}
  <div class="min-w-0 space-y-3" data-role="assistant">
    {#if thinkContent()}
      <details class="group text-label text-fg-muted">
        <summary
          class="inline-flex cursor-pointer items-center gap-1 text-caption text-fg-subtle hover:text-fg-muted"
        >
          <ChevronRight
            size={12}
            class="transition-transform duration-150 group-open:rotate-90"
            aria-hidden="true"
          />
          {$t('chat.thinkingLabel')}
        </summary>
        <pre
          class="mt-1.5 whitespace-pre-wrap border-l-2 border-line pl-3 font-sans text-label text-fg-muted">{thinkContent()}</pre>
      </details>
    {/if}
    {#if mainContent().trim()}
      <Markdown source={mainContent()} />
    {:else if showActivity}
      <ThinkingIndicator stage={activityStage} {startedAt} toolName={activeToolName} />
    {/if}
    {#if provenance && !isStreaming}
      <div class="rounded-card bg-fill px-3 py-2 text-caption text-fg-muted">
        {#if provenance.sources.length}
          <p class="font-medium text-fg">{$t('chat.evidenceSources')}</p>
          <ul class="mt-1 list-disc pl-4">
            {#each provenance.sources as source (source.toolName)}
              <li>{sourceLabel(source)}</li>
            {/each}
          </ul>
          <p class="mt-1">{$t('chat.evidenceInference')}</p>
        {:else}
          <p class="flex items-start gap-1.5 text-warning-fg">
            <AlertTriangle size={12} class="mt-0.5 shrink-0" aria-hidden="true" />
            <span>{$t('chat.evidenceUnsupported')}</span>
          </p>
        {/if}
        {#if provenance.unverifiedCitations.length}
          <p class="mt-1 text-warning-fg">
            {$t('chat.unverifiedCitations').replace(
              '{citations}',
              provenance.unverifiedCitations.join(', ')
            )}
          </p>
        {/if}
      </div>
    {/if}
  </div>
{:else if message.role === 'tool_call'}
  <div class="min-w-0">
    <Button
      variant="ghost"
      size="sm"
      class="-ml-2.5 max-w-full"
      onclick={() => (toolCallCollapsed = !toolCallCollapsed)}
      aria-label={toolCallCollapsed ? $t('chat.showToolCall') : $t('chat.hideToolCall')}
      aria-expanded={!toolCallCollapsed}
    >
      <Wrench size={14} class="text-fg-subtle" aria-hidden="true" />
      <span class="truncate">{$t('chat.toolCall')}: {message.tool_name ?? $t('common.unknown')}</span>
      <ChevronRight
        size={12}
        class="shrink-0 transition-transform duration-150 {toolCallCollapsed ? '' : 'rotate-90'}"
        aria-hidden="true"
      />
    </Button>
    {#if !toolCallCollapsed}
      <div class="mt-1 rounded-card bg-fill px-3 py-2">
        <pre
          class="overflow-x-auto whitespace-pre-wrap text-caption text-fg-muted">{message.tool_args_json ??
            message.content}</pre>
      </div>
    {/if}
  </div>
{:else if message.role === 'tool_result'}
  {@const parsed = parseToolResult(message.content)}
  {@const reportDraft = writeReportDraftProposal(message)}
  <div class="min-w-0">
    <Button
      variant="ghost"
      size="sm"
      class="-ml-2.5 max-w-full"
      onclick={() => (toolResultCollapsed = !toolResultCollapsed)}
      aria-label={toolResultCollapsed ? $t('chat.showToolResult') : $t('chat.hideToolResult')}
      aria-expanded={!toolResultCollapsed}
    >
      <Check size={14} class="text-success-fg" aria-hidden="true" />
      <span class="truncate">{$t('chat.toolResult')}</span>
      <ChevronRight
        size={12}
        class="shrink-0 transition-transform duration-150 {toolResultCollapsed ? '' : 'rotate-90'}"
        aria-hidden="true"
      />
    </Button>
    {#if !toolResultCollapsed}
      <div class="mt-1 rounded-card bg-fill px-3 py-2">
        {#if parsed !== null}
          {#if Array.isArray(parsed)}
            <div class="space-y-2">
              {#each parsed as item, i (i)}
                <div
                  class="border-b border-line-subtle pb-1 text-caption text-fg-muted last:border-0 last:pb-0"
                >
                  <span class="text-caption text-fg-subtle" data-numeric>{i + 1}</span>
                  {#each visibleEntries(item) as [key, val] (key)}
                    <div class="flex gap-2">
                      <span class="shrink-0 text-fg-muted">{toLabel(key)}:</span>
                      <span class="min-w-0 break-words text-fg">{formatValue(val)}</span>
                    </div>
                  {/each}
                </div>
              {/each}
            </div>
          {:else}
            <div class="space-y-1">
              {#each visibleEntries(parsed) as [key, val] (key)}
                <div class="flex gap-2 text-caption">
                  <span class="shrink-0 text-fg-muted">{toLabel(key)}:</span>
                  <span class="min-w-0 break-words text-fg">{formatValue(val)}</span>
                </div>
              {/each}
            </div>
          {/if}
        {:else}
          <pre
            class="overflow-x-auto whitespace-pre-wrap text-caption text-fg-muted">{message.content}</pre>
        {/if}
      </div>
    {/if}
    {#if reportDraft}
      <div class="mt-2 space-y-3 card-surface rounded-panel p-3">
        <div>
          <p class="text-label font-medium text-fg">{$t('chat.reportDraftUnsaved')}</p>
          <p class="mt-0.5 text-caption text-fg-muted">
            {$t('chat.reportDraftReviewHint')}
          </p>
        </div>
        <details class="text-caption text-fg-muted">
          <summary class="cursor-pointer text-label text-fg">{$t('chat.reviewReportDraft')}</summary>
          <Textarea
            bind:value={draftContent}
            aria-label={$t('chat.editReportDraft')}
            disabled={savedDraft || isSavingDraft}
            rows={10}
            class="mt-2"
          />
        </details>
        <div class="flex flex-wrap items-center gap-2">
          <Button
            onclick={saveManualRevision}
            disabled={isSavingRevision || savedDraft || draftContent === latestDraftVersion?.content}
            size="sm"
            >{isSavingRevision
              ? $t('chat.savingDraftRevision')
              : $t('chat.saveDraftRevision')}</Button
          >
          <span class="text-caption text-fg-muted"
            >{$t('chat.draftVersionCount').replace('{count}', String(draftVersions.length))}</span
          >
        </div>
        {#if draftVersions.length > 0}
          <details class="text-caption text-fg-muted">
            <summary class="cursor-pointer text-label text-fg"
              >{$t('chat.draftVersionHistory')}</summary
            >
            <div class="mt-1 flex flex-col items-start">
              {#each draftVersions as version (version.id)}
                <Button variant="ghost" size="sm" onclick={() => restoreDraftVersion(version)}
                  >{$t('chat.restoreDraftVersion')
                    .replace('{version}', String(version.version_number))
                    .replace('{origin}', version.origin)}</Button
                >
              {/each}
            </div>
          </details>
        {/if}
        {#if draftEvidence.length}
          <p class="text-caption text-fg-muted">
            {$t('chat.draftEvidenceSources').replace(
              '{sources}',
              draftEvidence.map(sourceLabel).join(', ')
            )}
          </p>
        {/if}
        {#if draftClaims.length}
          <fieldset class="space-y-2 rounded-control border border-warning-line bg-warning-subtle p-2">
            <legend class="px-1 text-caption font-medium text-warning-fg">
              {$t('chat.resolveUnsupportedClaims')}
            </legend>
            {#each draftClaims as claim (claim.id)}
              <label class="block text-caption text-fg-muted">
                <span>{claimLabel(claim)}</span>
                <Select
                  controlSize="sm"
                  class="mt-1"
                  value={claimResolutions[claim.id] ?? ''}
                  onchange={(event: Event & { currentTarget: HTMLSelectElement }) => {
                    claimResolutions = {
                      ...claimResolutions,
                      [claim.id]: event.currentTarget.value as ClaimResolution | '',
                    };
                  }}
                >
                  <option value="">{$t('chat.selectClaimResolution')}</option>
                  <option value="user_provided">{$t('chat.claimUserProvided')}</option>
                  <option value="uncertain">{$t('chat.claimUncertain')}</option>
                  <option value="removed">{$t('chat.claimRemoved')}</option>
                </Select>
              </label>
            {/each}
          </fieldset>
        {/if}
        {#if savedDraft}
          <p class="text-caption text-success-fg" role="status">{$t('chat.reportDraftSaved')}</p>
        {:else}
          <!-- Native checkbox: there is no checkbox primitive yet. -->
          <label class="flex items-start gap-2 text-caption text-fg-muted">
            <input
              type="checkbox"
              class="mt-0.5"
              bind:checked={reviewedDraft}
              disabled={isSavingDraft}
            />
            <span>{$t('chat.confirmReportDraftReviewed')}</span>
          </label>
          {#if draftError}
            <p class="text-caption text-danger-fg" role="alert">{draftError}</p>
            <Button variant="secondary" size="sm" onclick={loadDraftVersions}>
              {$t('common.retry')}
            </Button>
          {/if}
          <!-- Secondary, not primary: the composer's Send is the view's one ink action. -->
          <Button
            onclick={() => saveReviewedDraft(reportDraft)}
            disabled={!canSaveDraft}
            loading={isSavingDraft}
            size="sm"
          >
            {isSavingDraft ? $t('chat.savingReportDraft') : $t('chat.saveReviewedReportDraft')}
          </Button>
        {/if}
      </div>
    {/if}
  </div>
{/if}
