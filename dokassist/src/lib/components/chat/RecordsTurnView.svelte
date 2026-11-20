<script lang="ts">
  /*
   * A "Records only · cited" turn (D4a): the clinician's question and the
   * audited records answer, styled as a chat turn. Citations are the
   * pipeline's own `[E1]` markers, shown as numbers and linked to the source
   * record; the evidence list and the citation warnings come from the
   * manifest and audit unchanged.
   */
  import { resolve } from '$app/paths';
  import { AlertTriangle } from 'lucide-svelte';
  import { t } from '$lib/translations';
  import { chatActivityStage } from '$lib/chat-activity';
  import {
    linkPatientHistoryCitations,
    patientHistoryCitationHref,
  } from '$lib/patient-history-citations';
  import { stripThinkTags } from '$lib/llm/strip-think';
  import { Alert, ThinkingIndicator } from '$lib/components/ui';
  import { citationNumber, type RecordsTurn } from './patient-context';

  let { turn }: { turn: RecordsTurn } = $props();

  let visibleAnswer = $derived(turn.pending ? stripThinkTags(turn.answer) : turn.answer);
  let parts = $derived(linkPatientHistoryCitations(visibleAnswer, turn.manifest?.entries ?? []));
  let citedEntries = $derived(
    turn.manifest && turn.audit
      ? turn.audit.citations
          .filter((check) => check.in_manifest)
          .map((check) => ({
            check,
            entry: turn.manifest?.entries.find((entry) => entry.citation === check.citation),
          }))
          .filter((pair) => pair.entry !== undefined)
      : []
  );
  let citationWarnings = $derived(
    turn.audit ? [...turn.audit.unsupported_citations, ...turn.audit.stale_citations] : []
  );

  function citationLabel(label: string, date: string): string {
    return $t('patientHistory.openCitation').replace('{label}', label).replace('{date}', date);
  }
</script>

<div class="space-y-5" data-testid="records-turn">
  <div class="flex justify-end">
    <div
      class="min-w-0 max-w-[85%] whitespace-pre-wrap break-words rounded-card border border-line-subtle bg-surface-sunken px-3.5 py-2 text-body text-fg"
    >
      {turn.question}
    </div>
  </div>

  <div class="min-w-0 space-y-3">
    <p class="flex flex-wrap items-center gap-x-2 text-caption text-fg-subtle">
      <span class="font-medium text-fg-muted">{$t('chat.recordsTurnLabel')}</span>
      <span aria-hidden="true">·</span>
      <span>{$t('chat.recordsNotSaved')}</span>
    </p>

    {#if turn.error}
      <Alert tone="danger" class="whitespace-pre-wrap">{turn.error}</Alert>
    {:else if visibleAnswer}
      <div class="whitespace-pre-wrap break-words text-body text-fg">
        {#each parts as part, index (index)}
          {#if part.kind === 'citation'}<a
              href={resolve(patientHistoryCitationHref(part.entry))}
              aria-label={citationLabel(part.entry.label, part.entry.occurred_at)}
              title={citationLabel(part.entry.label, part.entry.occurred_at)}
              class="mx-px inline-flex min-w-4 items-center justify-center rounded-control border border-line px-1 align-super text-caption leading-none text-link no-underline hover:border-line-strong hover:text-link-hover"
              data-numeric>{citationNumber(part.entry.citation)}</a
            >{:else}{part.text}{/if}
        {/each}
      </div>
    {/if}

    {#if turn.pending}
      <ThinkingIndicator
        stage={chatActivityStage(turn.answer)}
        startedAt={turn.startedAt}
        label={chatActivityStage(turn.answer) === 'writing'
          ? $t('chat.activity.writing')
          : $t('patientHistory.askingRecord')}
      />
    {/if}

    {#if turn.manifest && !turn.pending}
      {#if citationWarnings.length > 0}
        <p class="flex items-start gap-2 text-label text-warning-fg">
          <AlertTriangle size={14} class="mt-0.5 shrink-0" aria-hidden="true" />
          <span>
            {$t('patientHistory.citationWarning').replace(
              '{citations}',
              citationWarnings.join(', ')
            )}
          </span>
        </p>
      {/if}
      <details class="rounded-card border border-line-subtle px-3 py-2 text-label text-fg-muted">
        <summary class="cursor-pointer text-fg-muted">
          {$t('patientHistory.evidence')
            .replace('{count}', String(turn.manifest.entries.length))
            .replace('{used}', String(turn.manifest.prompt_tokens))
            .replace('{budget}', String(turn.manifest.token_budget))}
        </summary>
        <ol class="mt-2 space-y-2">
          {#each citedEntries as { check, entry } (check.citation)}
            <li class="flex gap-2">
              <a
                href={resolve(patientHistoryCitationHref(entry!))}
                aria-label={citationLabel(entry!.label, entry!.occurred_at)}
                class="inline-flex h-5 min-w-5 shrink-0 items-center justify-center rounded-control border border-line px-1 text-caption text-link hover:border-line-strong hover:text-link-hover"
                data-numeric>{citationNumber(check.citation)}</a
              >
              <div class="min-w-0">
                <span class="text-fg">{entry?.label}</span>
                <span>
                  {$t('patientHistory.excerptRange')
                    .replace('{date}', String(entry?.occurred_at ?? ''))
                    .replace('{from}', String(entry?.char_start ?? ''))
                    .replace('{to}', String(entry?.char_end ?? ''))
                    .replace('{revision}', String(entry?.revision ?? ''))}
                </span>
                {#if !check.traceable}
                  <span class="text-warning-fg">{$t('patientHistory.sourceChanged')}</span>
                {/if}
                <div class="text-caption text-fg-subtle">
                  {$t('patientHistory.selectedBecause').replace(
                    '{reasons}',
                    entry?.selection_reasons?.join('; ') ?? '—'
                  )}
                </div>
              </div>
            </li>
          {/each}
          {#if citedEntries.length === 0}
            <li class="italic">{$t('patientHistory.noExcerpts')}</li>
          {/if}
        </ol>
        {#if turn.manifest.omitted.length > 0}
          <p class="mt-2 text-caption">
            {$t('patientHistory.omittedExcerpts').replace(
              '{count}',
              String(turn.manifest.omitted.length)
            )}
          </p>
        {/if}
      </details>
    {/if}
  </div>
</div>
