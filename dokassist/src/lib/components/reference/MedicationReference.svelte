<script lang="ts">
  import { onDestroy, onMount } from 'svelte';
  import { resolve } from '$app/paths';
  import { Columns2, Pill, Search, X } from 'lucide-svelte';
  import {
    getMedicationReferenceVersion,
    searchMedicationReference,
    parseError,
    type AppError,
    type SubstanceSummary,
  } from '$lib/api';
  import ErrorDisplay from '$lib/components/ErrorDisplay.svelte';
  import MedicationInfoPanel from '$lib/components/MedicationInfoPanel.svelte';
  import {
    Alert,
    Button,
    EmptyState,
    IconButton,
    Input,
    ListRow,
    Skeleton,
    StatusIndicator,
  } from '$lib/components/ui';
  import { t } from '$lib/translations';

  /*
   * Reference → Medications: search the installed Swissmedic AIPS data,
   * read a substance's details, and pin one to compare side by side.
   */
  let { initialQuery = '' }: { initialQuery?: string } = $props();

  let version: string | null = $state(null);
  let versionLoaded = $state(false);
  let query = $state('');
  let results: SubstanceSummary[] = $state([]);
  let selectedId: string | null = $state(null);
  let pinnedId: string | null = $state(null);
  let searching = $state(false);
  let error: AppError | null = $state(null);
  let sequence = 0;
  let timer: ReturnType<typeof setTimeout> | null = null;

  onMount(() => {
    getMedicationReferenceVersion()
      .then((v) => (version = v))
      .catch(() => (version = null))
      .finally(() => {
        versionLoaded = true;
        if (version && initialQuery) {
          query = initialQuery;
          search(0);
        }
      });
  });

  onDestroy(() => {
    if (timer) clearTimeout(timer);
  });

  async function run(q: string, seq: number) {
    try {
      const found = await searchMedicationReference(q);
      if (seq !== sequence) return;
      results = found;
      // Open the first hit so a search always shows something to read.
      if (!found.some((r) => r.id === selectedId)) selectedId = found[0]?.id ?? null;
    } catch (err) {
      if (seq !== sequence) return;
      results = [];
      error = parseError(err);
    } finally {
      if (seq === sequence) searching = false;
    }
  }

  function search(delay = 300) {
    const q = query.trim();
    const seq = ++sequence;
    error = null;
    if (timer) clearTimeout(timer);
    if (q.length < 2) {
      results = [];
      searching = false;
      selectedId = null;
      return;
    }
    searching = true;
    timer = setTimeout(() => {
      timer = null;
      void run(q, seq);
    }, delay);
  }

  function secondary(m: SubstanceSummary) {
    return [m.atc_code, ...m.trade_names.slice(0, 2)].filter(Boolean).join(' · ');
  }

  let comparing = $derived(Boolean(pinnedId && selectedId && pinnedId !== selectedId));
</script>

<div class="space-y-5">
  <div class="flex flex-wrap items-center justify-between gap-3">
    <p class="max-w-measure text-body text-fg-muted">
      {$t('literature.medicationReferenceDescription')}
    </p>
    {#if version}
      <StatusIndicator
        tone="ok"
        label={$t('literature.medicationReferenceVersion').replace('{version}', version)}
      />
    {/if}
  </div>

  {#if error}
    <ErrorDisplay {error} showDetails={true} />
  {/if}

  {#if !versionLoaded}
    <Skeleton class="block h-control w-full max-w-xl" />
  {:else if !version}
    <Alert tone="info" title={$t('reference.medicationsMissingTitle')}>
      <p>{$t('literature.medicationReferenceMissing')}</p>
      <div class="mt-3">
        <Button href={resolve('/settings')} size="sm">{$t('literature.openSettings')}</Button>
      </div>
    </Alert>
  {:else}
    <div class="relative max-w-xl">
      <Search
        size={16}
        class="pointer-events-none absolute left-2.5 top-1/2 -translate-y-1/2 text-fg-subtle"
        aria-hidden="true"
      />
      <Input
        type="search"
        bind:value={query}
        oninput={() => search()}
        aria-label={$t('literature.medicationSearchPlaceholder')}
        placeholder={$t('literature.medicationSearchPlaceholder')}
        class="pl-8"
      />
    </div>

    {#if query.trim().length < 2}
      <EmptyState
        icon={Pill}
        title={$t('reference.medicationsStartTitle')}
        description={$t('reference.medicationsStartDesc')}
      />
    {:else if searching && results.length === 0}
      <p class="text-label text-fg-muted" role="status">{$t('literature.medicationSearching')}</p>
    {:else if results.length === 0}
      <EmptyState
        icon={Search}
        title={$t('literature.medicationNoResults')}
        description={$t('reference.medicationsNoResultsDesc')}
      />
    {:else}
      <div class="grid gap-5 lg:grid-cols-[minmax(14rem,18rem)_minmax(0,1fr)]">
        <section aria-labelledby="medication-results-heading" class="min-w-0">
          <h2
            id="medication-results-heading"
            class="mb-1 px-3 text-caption font-medium text-fg-subtle"
          >
            {$t('reference.resultsCount').replace('{count}', String(results.length))}
          </h2>
          <div class="space-y-0.5">
            {#each results as medication (medication.id)}
              <ListRow
                selected={selectedId === medication.id}
                onclick={() => (selectedId = medication.id)}
                class="py-1.5"
              >
                <span class="flex min-w-0 flex-col text-left">
                  <span class="truncate text-body text-fg">{medication.name_de}</span>
                  <span class="truncate text-caption text-fg-muted">{secondary(medication)}</span>
                </span>
                {#snippet leading()}
                  {#if pinnedId === medication.id}
                    <Columns2 size={14} aria-label={$t('reference.pinned')} />
                  {:else}
                    <Pill size={14} aria-hidden="true" />
                  {/if}
                {/snippet}
              </ListRow>
            {/each}
          </div>
        </section>

        <div class="grid min-w-0 gap-4 {comparing ? 'xl:grid-cols-2' : ''}" aria-live="polite">
          {#if comparing}
            <MedicationInfoPanel substanceId={pinnedId} variant="full">
              {#snippet actions()}
                <IconButton
                  size="sm"
                  label={$t('reference.removeFromComparison')}
                  onclick={() => (pinnedId = null)}
                >
                  <X size={14} />
                </IconButton>
              {/snippet}
            </MedicationInfoPanel>
          {/if}
          <MedicationInfoPanel substanceId={selectedId} variant="full">
            {#snippet actions(detail)}
              {#if pinnedId !== detail.id}
                <Button size="sm" onclick={() => (pinnedId = detail.id)}>
                  <Columns2 size={14} aria-hidden="true" />
                  {$t('reference.compare')}
                </Button>
              {:else}
                <Button size="sm" variant="ghost" onclick={() => (pinnedId = null)}>
                  {$t('reference.unpin')}
                </Button>
              {/if}
            {/snippet}
          </MedicationInfoPanel>
          {#if pinnedId && !comparing}
            <p class="text-caption text-fg-muted">{$t('reference.compareHint')}</p>
          {/if}
        </div>
      </div>
    {/if}
  {/if}
</div>
