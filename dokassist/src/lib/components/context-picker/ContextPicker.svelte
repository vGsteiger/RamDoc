<script lang="ts">
  import { getPatient, globalSearch, type Patient } from '$lib/api';
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
  }

  let {
    value = $bindable(createContextPlan()),
    patients: initialPatients = undefined,
    labels: labelOverrides = {},
    sourceLabels = {},
    disabled = false,
    lockedPatientId = undefined,
    onplanchange,
  }: Props = $props();

  let patients = $state<Patient[]>([]);
  let query = $state('');
  let loadingPatients = $state(false);
  let refreshVersion = 0;
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
        patients = await Promise.all(ids.map((id) => getPatient(id)));
      } finally {
        loadingPatients = false;
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

<section
  class="w-full rounded-card border border-line bg-surface-raised p-3 sm:p-4"
  aria-labelledby="context-picker-title"
>
  <div class="flex flex-col gap-3 sm:flex-row sm:items-start sm:justify-between">
    <div>
      <h2 id="context-picker-title" class="text-heading text-fg">{labels.title}</h2>
      <p class="mt-1 text-caption text-fg-muted">{labels.comparisonModeHint}</p>
    </div>
    {#if !lockedPatientId}<button
        type="button"
        class="min-h-10 rounded-control border border-line px-3 text-body text-fg hover:bg-surface-hover disabled:opacity-50"
        aria-pressed={value.comparisonMode}
        {disabled}
        onclick={() => update(setComparisonMode(value, !value.comparisonMode))}
        >{labels.comparisonMode}</button
      >{/if}
  </div>

  <div class="mt-4 grid gap-4 lg:grid-cols-[minmax(0,1fr)_minmax(18rem,0.8fr)]">
    <div>
      <label for="context-picker-search" class="field-label">{labels.patientSearch}</label>
      <input
        id="context-picker-search"
        type="search"
        bind:value={query}
        {disabled}
        placeholder={labels.patientSearchPlaceholder}
        class="min-h-10 w-full rounded-control border border-line bg-surface-raised px-3 text-body text-fg focus:border-accent focus:outline-none focus:ring-2 focus:ring-accent/25 disabled:bg-surface-sunken"
      />
      <div
        class="mt-2 max-h-40 overflow-y-auto rounded-control border border-line"
        role="listbox"
        aria-label={labels.patientSearch}
      >
        {#if loadingPatients}
          <p class="p-3 text-body text-fg-muted">{labels.loading}</p>
        {:else if matches.length === 0}
          <p class="p-3 text-body text-fg-muted">{labels.noPatients}</p>
        {:else}
          {#each matches.slice(0, 20) as patient (patient.id)}
            <button
              type="button"
              class="flex min-h-10 w-full items-center justify-between gap-3 px-3 text-left text-body text-fg hover:bg-surface-hover disabled:opacity-50"
              role="option"
              aria-selected={value.selectedPatients.some((item) => item.id === patient.id)}
              disabled={disabled || (!!lockedPatientId && patient.id !== lockedPatientId)}
              onclick={() => choose(patient)}
            >
              <span class="truncate">{patient.first_name} {patient.last_name}</span>
              {#if patient.date_of_birth}<span class="shrink-0 text-caption text-fg-subtle"
                  >{patient.date_of_birth}</span
                >{/if}
            </button>
          {/each}
        {/if}
      </div>
    </div>

    <div>
      <p class="field-label">{labels.selectedPatients}</p>
      <div class="flex min-h-10 flex-wrap gap-2" aria-live="polite">
        {#each value.selectedPatients as patient (patient.id)}
          <button
            type="button"
            class="min-h-9 max-w-full truncate rounded-full border border-accent-line bg-accent-subtle px-3 text-label text-accent-fg hover:bg-surface-selected disabled:opacity-50"
            disabled={disabled || patient.id === lockedPatientId}
            onclick={() => update(removeContextPatient(value, patient.id), true)}
            aria-label={`${labels.clearPatients}: ${patient.first_name} ${patient.last_name}`}
            >{patient.first_name} {patient.last_name} ×</button
          >
        {/each}
      </div>
    </div>
  </div>

  <fieldset class="mt-4 border-t border-line-subtle pt-4" {disabled}>
    <legend class="text-heading text-fg">{labels.sources}</legend>
    <div class="mt-3 grid gap-2 sm:grid-cols-2 lg:grid-cols-3">
      {#each value.sources as item (item.kind)}
        <div class="flex min-h-12 items-center gap-2 rounded-control border border-line px-3">
          <input
            id={`context-source-${item.kind}`}
            type="checkbox"
            checked={item.included}
            disabled={item.pinned}
            onchange={() => update(toggleContextSource(value, item.kind), true)}
          />
          <label
            class="min-w-0 flex-1 truncate text-body text-fg"
            for={`context-source-${item.kind}`}>{sourceLabel(item.kind)}</label
          >
          <button
            type="button"
            class="min-h-9 min-w-9 rounded-control px-2 text-caption text-fg-muted hover:bg-surface-hover"
            aria-pressed={item.pinned}
            aria-label={item.pinned
              ? `${labels.unpin}: ${sourceLabel(item.kind)}`
              : `${labels.pin}: ${sourceLabel(item.kind)}`}
            onclick={() => update(togglePinnedContextSource(value, item.kind), true)}
            >{item.pinned ? '★' : '☆'}</button
          >
        </div>
      {/each}
    </div>
  </fieldset>

  <div
    class="mt-4 flex flex-wrap gap-x-4 gap-y-1 border-t border-line-subtle pt-3 text-caption text-fg-muted"
    aria-live="polite"
  >
    <span>{labels.included}: {value.planning.included.length}</span>
    <span>{labels.retrieved}: {value.planning.retrieved}</span>
    {#if value.planning.isRefreshing}<span>{labels.loading}</span>{/if}
  </div>
</section>
