<script lang="ts">
  import { ChevronDown, ChevronUp, Minus, TrendingDown, TrendingUp } from 'lucide-svelte';
  import type { OutcomeScore } from '$lib/api';
  import OutcomeScoreTrendChart from '$lib/components/OutcomeScoreTrendChart.svelte';
  import { Badge, Button } from '$lib/components/ui';
  import { fmt } from '$lib/format';
  import { t } from '$lib/translations';
  import { interpretationLabel } from '$lib/translations/labels';
  import { severityTone, summariseScores } from './workspace';

  /*
   * Latest score per questionnaire with the change since the previous
   * administration, plus the existing trend charts behind a toggle. All
   * supported scales (PHQ-9, GAD-7, BDI-II) score higher = worse, so the
   * arrow states direction only and the text carries the meaning.
   */
  let { scores }: { scores: OutcomeScore[] } = $props();

  let showTrend = $state(false);
  let summaries = $derived(summariseScores(scores));
  const trendId = $props.id();
</script>

<ul class="divide-y divide-line-subtle">
  {#each summaries as summary (summary.scale)}
    {@const delta = summary.previous ? summary.latest.score - summary.previous.score : null}
    <li class="flex flex-wrap items-center gap-x-3 gap-y-1 py-2">
      <span class="w-16 shrink-0 text-label font-medium text-fg">{summary.scale}</span>
      <span class="text-heading text-fg" data-numeric>{summary.latest.score}</span>
      {#if summary.latest.interpretation}
        <Badge tone={severityTone(summary.latest.interpretation)}>
          {$interpretationLabel(summary.latest.interpretation)}
        </Badge>
      {/if}
      <span class="ml-auto flex items-center gap-1 text-caption text-fg-muted" data-numeric>
        {#if delta !== null && summary.previous}
          {#if delta < 0}
            <TrendingDown size={14} aria-hidden="true" />
          {:else if delta > 0}
            <TrendingUp size={14} aria-hidden="true" />
          {:else}
            <Minus size={14} aria-hidden="true" />
          {/if}
          {$t('outcomeScores.changeSince')
            .replace('{delta}', delta > 0 ? `+${delta}` : String(delta))
            .replace('{date}', $fmt.date(summary.previous.administered_at))}
        {:else}
          {$fmt.date(summary.latest.administered_at)}
        {/if}
      </span>
    </li>
  {/each}
</ul>

<div class="mt-2">
  <Button
    size="sm"
    variant="ghost"
    aria-expanded={showTrend}
    aria-controls={trendId}
    onclick={() => (showTrend = !showTrend)}
  >
    {#if showTrend}
      <ChevronUp size={14} aria-hidden="true" />
    {:else}
      <ChevronDown size={14} aria-hidden="true" />
    {/if}
    {$t('outcomeScores.trends')}
  </Button>
</div>

{#if showTrend}
  <div id={trendId} class="mt-3 space-y-4">
    {#each summaries as summary (summary.scale)}
      <div>
        <h4 class="mb-2 text-label font-medium text-fg-muted">
          {summary.scale}
          <span class="font-normal text-fg-subtle" data-numeric>
            ({summary.scores.length}
            {summary.scores.length === 1
              ? $t('common.measurementOne')
              : $t('common.measurementMany')})
          </span>
        </h4>
        <OutcomeScoreTrendChart scores={summary.scores} scaleType={summary.scale} />
      </div>
    {/each}
  </div>
{/if}
