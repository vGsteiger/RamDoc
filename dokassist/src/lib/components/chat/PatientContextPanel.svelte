<script lang="ts">
  /*
   * D4: the patient context beside a patient-attached chat. Read-only: the
   * identification, active diagnoses, current medication, the latest outcome
   * score per scale and the last three sessions, each with a link into the
   * record. Editing stays on the patient pages.
   */
  import { resolve } from '$app/paths';
  import { X } from 'lucide-svelte';
  import {
    getPatient,
    listDiagnosesForPatient,
    listMedicationsForPatient,
    listScoresForPatient,
    listSessionsForPatient,
    type Diagnosis,
    type Medication,
    type OutcomeScore,
    type Patient,
    type Session,
  } from '$lib/api';
  import { t } from '$lib/translations';
  import { errorText, sessionTypeLabel } from '$lib/translations/labels';
  import { ageInYears, fmt, toDate } from '$lib/format';
  import { maskAhv } from './patient-context';
  import { Alert, Button, IconButton, ListRow, Skeleton } from '$lib/components/ui';

  let {
    patientId,
    onclose = undefined,
    id = undefined,
  }: {
    patientId: string;
    /** Shows a close button (the overlay sheet below 1280 px). */
    onclose?: () => void;
    id?: string;
  } = $props();

  let patient = $state<Patient | null>(null);
  let diagnoses = $state<Diagnosis[]>([]);
  let medications = $state<Medication[]>([]);
  let scores = $state<OutcomeScore[]>([]);
  let sessions = $state<Session[]>([]);
  let loading = $state(true);
  let error = $state('');
  let loadVersion = 0;

  let age = $derived(patient ? ageInYears(patient.date_of_birth) : null);
  let activeDiagnoses = $derived(diagnoses.filter((d) => d.status === 'active'));
  let currentMedications = $derived(
    medications.filter((m) => {
      const end = toDate(m.end_date);
      return !end || end.getTime() >= Date.now();
    })
  );
  /** The newest score per scale, newest first. */
  let latestScores = $derived(
    [...scores]
      .sort((a, b) => b.administered_at.localeCompare(a.administered_at))
      .filter((score, i, all) => all.findIndex((s) => s.scale_type === score.scale_type) === i)
      .slice(0, 4)
  );
  let recentSessions = $derived(
    [...sessions].sort((a, b) => b.session_date.localeCompare(a.session_date)).slice(0, 3)
  );

  let recordHref = $derived(resolve('/patients/[id]', { id: patientId }));

  async function load(pid: string) {
    const version = ++loadVersion;
    loading = true;
    error = '';
    try {
      const [p, d, m, sc, se] = await Promise.all([
        getPatient(pid),
        listDiagnosesForPatient(pid),
        listMedicationsForPatient(pid),
        listScoresForPatient(pid),
        listSessionsForPatient(pid),
      ]);
      if (version !== loadVersion) return;
      patient = p;
      diagnoses = d ?? [];
      medications = m ?? [];
      scores = sc ?? [];
      sessions = se ?? [];
    } catch (err) {
      if (version === loadVersion) error = $errorText(err, $t('chat.patientPanel.loadError'));
    } finally {
      if (version === loadVersion) loading = false;
    }
  }

  $effect(() => {
    void load(patientId);
  });
</script>

{#snippet sectionTitle(title: string, href: string, linkLabel: string)}
  <div class="mb-1 flex min-h-control-sm items-center justify-between gap-2">
    <h3 class="text-caption font-medium text-fg-muted">{title}</h3>
    <a {href} class="text-caption text-link underline hover:text-link-hover">{linkLabel}</a>
  </div>
{/snippet}

<aside
  {id}
  class="flex h-full min-h-0 flex-col bg-surface"
  aria-label={$t('chat.patientPanel.title')}
  data-testid="patient-context-panel"
>
  <div class="flex min-h-12 shrink-0 items-center justify-between gap-2 border-b border-line px-4">
    <h2 class="text-label font-medium text-fg-muted">{$t('chat.patientPanel.title')}</h2>
    {#if onclose}
      <IconButton label={$t('chat.patientPanel.close')} size="sm" onclick={onclose}>
        <X size={16} />
      </IconButton>
    {/if}
  </div>

  <div class="min-h-0 flex-1 space-y-5 overflow-y-auto px-4 py-4">
    {#if loading && !patient}
      <div class="space-y-2" aria-hidden="true">
        <Skeleton class="h-6 w-2/3" />
        <Skeleton class="h-4 w-1/2" />
        <Skeleton class="h-row w-full" />
        <Skeleton class="h-row w-full" />
      </div>
    {:else if error}
      <Alert tone="warning">{error}</Alert>
    {:else if patient}
      <section>
        <p class="text-heading font-semibold text-fg [overflow-wrap:anywhere]">
          {patient.first_name}
          {patient.last_name}
        </p>
        <p class="mt-1 flex flex-wrap gap-x-1.5 text-label text-fg-muted" data-numeric>
          {#if age !== null}
            <span>{$t('chat.patientPanel.age').replace('{age}', String(age))}</span>
            <span aria-hidden="true">·</span>
          {/if}
          <span>
            {$t('chat.patientPanel.born').replace('{date}', $fmt.date(patient.date_of_birth))}
          </span>
        </p>
        {#if patient.ahv_number}
          <p class="mt-0.5 text-label text-fg-muted">
            {$t('chat.patientPanel.ahv')}
            <span class="font-mono" data-numeric data-testid="patient-ahv"
              >{maskAhv(patient.ahv_number)}</span
            >
          </p>
        {/if}
        <Button size="sm" class="mt-3" href={recordHref}>{$t('chat.patientPanel.openRecord')}</Button>
      </section>

      <section>
        {@render sectionTitle(
          $t('chat.patientPanel.diagnoses'),
          resolve('/patients/[id]/diagnoses', { id: patientId }),
          $t('chat.patientPanel.allDiagnoses')
        )}
        {#if activeDiagnoses.length === 0}
          <p class="text-label text-fg-subtle">{$t('chat.patientPanel.none')}</p>
        {:else}
          <ul class="space-y-1.5">
            {#each activeDiagnoses as diagnosis (diagnosis.id)}
              <li class="flex gap-2 text-label">
                <span class="shrink-0 font-mono text-fg">{diagnosis.icd10_code}</span>
                <span class="min-w-0 text-fg-muted">{diagnosis.description}</span>
              </li>
            {/each}
          </ul>
        {/if}
      </section>

      <section>
        {@render sectionTitle(
          $t('chat.patientPanel.medications'),
          resolve('/patients/[id]/medications', { id: patientId }),
          $t('chat.patientPanel.allMedications')
        )}
        {#if currentMedications.length === 0}
          <p class="text-label text-fg-subtle">{$t('chat.patientPanel.none')}</p>
        {:else}
          <ul class="space-y-1.5">
            {#each currentMedications as medication (medication.id)}
              <li class="flex flex-wrap gap-x-1.5 text-label" data-numeric>
                <span class="text-fg">{medication.substance}</span>
                <span class="text-fg-muted">{medication.dosage}</span>
                {#if medication.frequency}
                  <span class="text-fg-subtle" aria-hidden="true">·</span>
                  <span class="text-fg-muted">{medication.frequency}</span>
                {/if}
              </li>
            {/each}
          </ul>
        {/if}
      </section>

      {#if latestScores.length > 0}
        <section>
          <h3 class="mb-1 text-caption font-medium text-fg-muted">
            {$t('chat.patientPanel.scores')}
          </h3>
          <dl class="grid grid-cols-[auto_1fr] gap-x-3 gap-y-1 text-label">
            {#each latestScores as score (score.id)}
              <dt class="text-fg">{score.scale_type}</dt>
              <dd class="text-fg-muted" data-numeric>
                <span class="text-fg">{score.score}</span>
                · {$fmt.date(score.administered_at)}
              </dd>
            {/each}
          </dl>
        </section>
      {/if}

      <section>
        {@render sectionTitle(
          $t('chat.patientPanel.sessions'),
          resolve('/patients/[id]/sessions', { id: patientId }),
          $t('chat.patientPanel.allSessions')
        )}
        {#if recentSessions.length === 0}
          <p class="text-label text-fg-subtle">{$t('chat.patientPanel.none')}</p>
        {:else}
          <div class="-mx-3">
            {#each recentSessions as session (session.id)}
              <ListRow
                title={$fmt.date(session.session_date)}
                secondary={$sessionTypeLabel(session.session_type)}
                href={resolve('/patients/[id]/sessions/[sessionId]', {
                  id: patientId,
                  sessionId: session.id,
                })}
              />
            {/each}
          </div>
        {/if}
      </section>
    {/if}
  </div>
</aside>
