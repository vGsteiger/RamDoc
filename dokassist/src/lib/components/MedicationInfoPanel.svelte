<script lang="ts">
  import type { Snippet } from 'svelte';
  import { AlertTriangle } from 'lucide-svelte';
  import { getMedicationReferenceDetail, type SubstanceDetail } from '$lib/api';
  import { Skeleton } from '$lib/components/ui';
  import { t } from '$lib/translations';

  /*
   * Swissmedic AIPS reference details for one substance.
   *
   *   variant="inline" (default) — compact and collapsible, under the
   *     substance field of the medication form.
   *   variant="full" — the Reference page detail pane: always open, body text,
   *     with an optional `actions` slot in the header (e.g. Compare).
   */
  interface Props {
    substanceId: string | null;
    variant?: 'inline' | 'full';
    actions?: Snippet<[SubstanceDetail]>;
  }

  let { substanceId, variant = 'inline', actions }: Props = $props();

  let detail = $state<SubstanceDetail | null>(null);
  let loading = $state(false);
  let open = $state(true);

  $effect(() => {
    if (!substanceId) {
      detail = null;
      return;
    }

    loading = true;
    open = true;
    const currentSubstanceId = substanceId;
    getMedicationReferenceDetail(substanceId)
      .then((d) => {
        if (currentSubstanceId === substanceId) {
          detail = d;
        }
      })
      .catch(() => {
        if (currentSubstanceId === substanceId) {
          detail = null;
        }
      })
      .finally(() => {
        if (currentSubstanceId === substanceId) {
          loading = false;
        }
      });
  });

  let full = $derived(variant === 'full');
</script>

{#snippet heading(d: SubstanceDetail)}
  <span class="flex min-w-0 flex-wrap items-baseline gap-x-2 gap-y-0.5">
    <span class="{full ? 'text-title' : 'text-body'} font-semibold text-fg">{d.name_de}</span>
    {#if d.atc_code}
      <span class="font-mono text-caption text-fg-muted">{d.atc_code}</span>
    {/if}
    {#if d.trade_names.length > 0}
      <span class="min-w-0 text-label text-fg-muted">
        {d.trade_names.slice(0, full ? 6 : 3).join(' · ')}
      </span>
    {/if}
  </span>
{/snippet}

{#snippet sections(d: SubstanceDetail)}
  <dl class="space-y-3 {full ? 'text-body' : 'text-label'}">
    {#if d.indication}
      <div>
        <dt class="font-medium text-fg">{$t('medications.indication')}</dt>
        <dd class="mt-0.5 leading-relaxed text-fg-muted">{d.indication}</dd>
      </div>
    {/if}
    {#if d.side_effects}
      <div>
        <dt class="font-medium text-fg">{$t('medications.sideEffects')}</dt>
        <dd class="mt-0.5 leading-relaxed text-fg-muted">{d.side_effects}</dd>
      </div>
    {/if}
    {#if d.contraindications}
      <div>
        <dt class="flex items-center gap-1.5 font-medium text-danger-fg">
          <AlertTriangle size={14} aria-hidden="true" />
          {$t('medications.contraindications')}
        </dt>
        <dd class="mt-0.5 leading-relaxed text-fg-muted">{d.contraindications}</dd>
      </div>
    {/if}
  </dl>
  {#if d.source_version}
    <p class="mt-3 text-caption text-fg-subtle">
      {$t('common.source')}: Swissmedic AIPS {d.source_version}
    </p>
  {/if}
{/snippet}

{#if loading}
  <div
    class="{full ? '' : 'mt-2'} space-y-2 rounded-card border border-line bg-surface-raised p-3"
    role="status"
    aria-label={$t('medications.loadingReference')}
  >
    <Skeleton class="block h-4 w-1/3" />
    <Skeleton class="block h-3 w-full" />
    <Skeleton class="block h-3 w-2/3" />
  </div>
{:else if detail && full}
  <article class="rounded-card border border-line bg-surface-raised p-4 sm:p-5">
    <header class="flex flex-wrap items-start justify-between gap-3">
      {@render heading(detail)}
      {#if actions}
        <div class="flex shrink-0 items-center gap-2">{@render actions(detail)}</div>
      {/if}
    </header>
    <div class="mt-4 border-t border-line-subtle pt-4">
      {@render sections(detail)}
    </div>
  </article>
{:else if detail}
  <details
    class="group mt-2 overflow-hidden rounded-card border border-line bg-surface-raised"
    bind:open
  >
    <summary
      class="flex cursor-pointer list-none items-center justify-between gap-2 px-3 py-2 transition-colors duration-150 ease-standard hover:bg-surface-hover [&::-webkit-details-marker]:hidden"
    >
      {@render heading(detail)}
      <span class="shrink-0 text-caption text-fg-subtle group-open:rotate-90" aria-hidden="true"
        >▸</span
      >
    </summary>
    <div class="border-t border-line-subtle px-3 py-3">
      {@render sections(detail)}
    </div>
  </details>
{/if}
