<script lang="ts">
  import { Pin, X } from 'lucide-svelte';
  import { getPatient, globalSearch, type Patient } from '$lib/api';
  import { fmt } from '$lib/format';
  import { Button, IconButton, Input, ListRow } from '$lib/components/ui';
  import {
    createContextPlan,
    refreshContextPlan,
    removeContextPatient,
    selectContextPatient,
    setComparisonMode,
    toggleContextSource,
    togglePinnedContextSource,
  } from './state';
  import {
    DEFAULT_CONTEXT_PICKER_LABELS,
    type ContextPickerLabels,
    type ContextPlan,
    type ContextSourceKind,
  } from './types';

  interface Props {
    value?: ContextPlan;
    patients?: Patient[];
    labels?: Partial<ContextPickerLabels>;
    sourceLabels?: Partial<Record<ContextSourceKind, string>>;
    disabled?: boolean;
    lockedPatientId?: string;
    onplanchange?: (plan: ContextPlan) => void;
    /** Shows a close button in the header (when rendered in a popover). */
    onclose?: () => void;
    closeLabel?: string;
  }

  let {
    value = $bindable(createContextPlan()),
    patients: initialPatients = undefined,
    labels: labelOverrides = {},
    sourceLabels = {},
    disabled = false,
    lockedPatientId = undefined,
    onplanchange,
    onclose = undefined,
    closeLabel = undefined,
  }: Props = $props();

  let patients = $state<Patient[]>([]);
  let query = $state('');
  let loadingPatients = $state(false);
  let refreshVersion = 0;
  let searchVersion = 0;
  let labels = $derived({ ...DEFAULT_CONTEXT_PICKER_LABELS, ...labelOverrides });
  let matches = $derived(patients);

  $effect(() => {
    if (initialPatients !== undefined) {
      patients = initialPatients;
      loadingPatients = false;
    }
  });

  $effect(() => {
    const needle = query.trim();
    const version = ++searchVersion;
    if (initialPatients !== undefined) return;
    if (!needle) {
      patients = [];
      loadingPatients = false;
      return;
    }
    const timeout = setTimeout(async () => {
      loadingPatients = true;
      try {
        const results = await globalSearch(needle, 50);
        const ids = results
          .filter((result) => result.result_type === 'patient')
          .map((result) => result.entity_id);
        const found = await Promise.all(ids.map((id) => getPatient(id)));
        if (version === searchVersion) patients = found;
      } finally {
        if (version === searchVersion) loadingPatients = false;
      }
    }, 200);
    return () => clearTimeout(timeout);
  });

  function update(plan: ContextPlan, refresh = false) {
    value = plan;
    onplanchange?.(value);
    if (refresh && value.selectedPatients.length > 0) void refreshPlan();
  }

  async function refreshPlan() {
    const version = ++refreshVersion;
    value = { ...value, planning: { ...value.planning, isRefreshing: true } };
    onplanchange?.(value);
    try {
      const refreshed = await refreshContextPlan(value);
      if (version === refreshVersion) update(refreshed);
    } catch {
      if (version === refreshVersion) {
        update({ ...value, planning: { ...value.planning, isRefreshing: false } });
      }
    }
  }

  function choose(patient: Patient) {
    if (lockedPatientId && patient.id !== lockedPatientId) return;
    update(selectContextPatient(value, patient), true);
    query = '';
  }

  function sourceLabel(kind: ContextSourceKind) {
    return sourceLabels[kind] ?? kind[0].toUpperCase() + kind.slice(1);
  }
</script>

<section class="@container w-full min-w-0" aria-labelledby="context-picker-title">
  <div class="flex items-start justify-between gap-3">
    <div class="min-w-0">
      <h2 id="context-picker-title" class="text-heading text-fg">{labels.title}</h2>
      <p class="mt-0.5 text-caption text-fg-muted">{labels.comparisonModeHint}</p>
    </div>
    <div class="flex shrink-0 items-center gap-1">
      {#if !lockedPatientId}
        <Button
          size="sm"
          variant={value.comparisonMode ? 'secondary' : 'ghost'}
          aria-pressed={value.comparisonMode}
          {disabled}
          onclick={() => update(setComparisonMode(value, !value.comparisonMode))}
          >{labels.comparisonMode}</Button
        >
      {/if}
      {#if onclose && closeLabel}
        <IconButton label={closeLabel} size="sm" onclick={onclose}><X size={16} /></IconButton>
      {/if}
    </div>
  </div>

  <div class="mt-3 grid gap-4 @xl:grid-cols-[minmax(0,1fr)_minmax(12rem,0.8fr)]">
    <div class="min-w-0">
      <label for="context-picker-search" class="field-label">{labels.patientSearch}</label>
      <Input
        id="context-picker-search"
        type="search"
        bind:value={query}
        {disabled}
        placeholder={labels.patientSearchPlaceholder}
      />
      <div class="mt-2 max-h-40 overflow-y-auto rounded-control border border-line-subtle py-0.5">
        {#if loadingPatients}
          <p class="px-3 py-2 text-label text-fg-muted">{labels.loading}</p>
        {:else if matches.length === 0}
          <p class="px-3 py-2 text-label text-fg-muted">{labels.noPatients}</p>
        {:else}
          {#each matches.slice(0, 20) as patient (patient.id)}
            <ListRow
              title={`${patient.first_name} ${patient.last_name}`}
              meta={patient.date_of_birth ? $fmt.date(patient.date_of_birth) : undefined}
              disabled={disabled || (!!lockedPatientId && patient.id !== lockedPatientId)}
              onclick={() => choose(patient)}
            />
          {/each}
        {/if}
      </div>
    </div>

    <div class="min-w-0">
      <p class="field-label">{labels.selectedPatients}</p>
      <div class="flex min-h-control flex-wrap gap-1.5" aria-live="polite">
        {#each value.selectedPatients as patient (patient.id)}
          <Button
            size="sm"
            class="max-w-full"
            disabled={disabled || patient.id === lockedPatientId}
            onclick={() => update(removeContextPatient(value, patient.id), true)}
            aria-label={`${labels.clearPatients}: ${patient.first_name} ${patient.last_name}`}
          >
            <span class="min-w-0 truncate">{patient.first_name} {patient.last_name}</span>
            <X size={12} class="shrink-0 text-fg-subtle" aria-hidden="true" />
          </Button>
        {/each}
      </div>
    </div>
  </div>

  <fieldset class="mt-4 border-t border-line-subtle pt-3" {disabled}>
    <legend class="sr-only">{labels.sources}</legend>
    <p class="mb-2 text-label font-medium text-fg" aria-hidden="true">{labels.sources}</p>
    <div class="grid gap-1.5 @md:grid-cols-2 @2xl:grid-cols-3">
      {#each value.sources as item (item.kind)}
        <div
          class="flex min-h-control items-center gap-2 rounded-control border border-line-subtle pl-2.5 pr-1"
        >
          <input
            id={`context-source-${item.kind}`}
            type="checkbox"
            checked={item.included}
            disabled={item.pinned}
            onchange={() => update(toggleContextSource(value, item.kind), true)}
          />
          <label
            class="min-w-0 flex-1 truncate text-label text-fg"
            for={`context-source-${item.kind}`}>{sourceLabel(item.kind)}</label
          >
          <IconButton
            size="sm"
            aria-pressed={item.pinned}
            label={item.pinned
              ? `${labels.unpin}: ${sourceLabel(item.kind)}`
              : `${labels.pin}: ${sourceLabel(item.kind)}`}
            onclick={() => update(togglePinnedContextSource(value, item.kind), true)}
          >
            <Pin
              size={14}
              fill={item.pinned ? 'currentColor' : 'none'}
              class={item.pinned ? 'text-fg' : ''}
              aria-hidden="true"
            />
          </IconButton>
        </div>
      {/each}
    </div>
  </fieldset>

  <div
    class="mt-3 flex flex-wrap gap-x-4 gap-y-1 text-caption text-fg-subtle"
    aria-live="polite"
    data-numeric
  >
    <span>{labels.included}: {value.planning.included.length}</span>
    <span>{labels.retrieved}: {value.planning.retrieved}</span>
    {#if value.planning.isRefreshing}<span>{labels.loading}</span>{/if}
  </div>
</section>
