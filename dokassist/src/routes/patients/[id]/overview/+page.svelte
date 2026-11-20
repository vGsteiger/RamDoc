<script lang="ts">
  import { onMount } from 'svelte';
  import { ArrowRight } from 'lucide-svelte';
  import {
    listScoresForPatient,
    listSessionsForPatient,
    type OutcomeScore,
    type Session,
  } from '$lib/api';
  import { Button, Card, ListRow, SectionHeader, PageShell, Skeleton } from '$lib/components/ui';
  import OutcomeSummary from '$lib/components/patient/OutcomeSummary.svelte';
  import { getPatientWorkspace } from '$lib/components/patient/state.svelte';
  import {
    activeDiagnoses,
    activeMedications,
    patientPath,
  } from '$lib/components/patient/workspace';
  import { fmt } from '$lib/format';
  import { t } from '$lib/translations';
  import { sessionTypeLabel } from '$lib/translations/labels';

  /*
   * The record summary: active diagnoses, current medications, the latest
   * questionnaire scores with their trend, recent sessions, sessions still
   * missing notes, and the master data. Records-only Q&A moved into chat
   * (D4a), so this page no longer hosts its own question box.
   */
  const workspace = getPatientWorkspace();
  const RECENT = 5;

  let patient = $derived(workspace?.patient ?? null);
  let id = $derived(workspace?.id ?? '');
  let diagnoses = $derived(activeDiagnoses(workspace?.diagnoses ?? []));
  let medications = $derived(activeMedications(workspace?.medications ?? []));

  let sessions = $state<Session[]>([]);
  let scores = $state<OutcomeScore[]>([]);
  let loading = $state(true);

  let recentSessions = $derived(
    [...sessions].sort((a, b) => b.session_date.localeCompare(a.session_date)).slice(0, RECENT)
  );
  let incomplete = $derived(sessions.filter((s) => !s.notes || !s.notes.trim()));

  onMount(async () => {
    if (!workspace) return;
    const [loadedSessions, loadedScores] = await Promise.allSettled([
      listSessionsForPatient(workspace.id),
      listScoresForPatient(workspace.id),
    ]);
    if (loadedSessions.status === 'fulfilled') sessions = loadedSessions.value;
    else console.error('Error loading sessions:', loadedSessions.reason);
    if (loadedScores.status === 'fulfilled') scores = loadedScores.value;
    else console.error('Error loading outcome scores:', loadedScores.reason);
    loading = false;
  });

  function genderLabel(gender: string): string {
    return ['male', 'female', 'other'].includes(gender) ? $t(`patients.${gender}`) : gender;
  }

  function sessionMeta(session: Session): string {
    const parts = [$fmt.date(session.session_date)];
    if (session.duration_minutes) parts.push($fmt.duration(session.duration_minutes));
    return parts.join(' · ');
  }
</script>

{#snippet viewAll(tab: string, label: string)}
  <Button size="sm" variant="ghost" href={patientPath(id, tab)}>
    {label}
    <ArrowRight size={14} aria-hidden="true" />
  </Button>
{/snippet}

{#snippet detail(label: string, value: string | null | undefined, numeric = false)}
  {#if value}
    <div class="min-w-0">
      <dt class="text-caption text-fg-muted">{label}</dt>
      <dd
        class="text-body text-fg whitespace-pre-line [overflow-wrap:anywhere]"
        data-numeric={numeric || undefined}
      >
        {value}
      </dd>
    </div>
  {/if}
{/snippet}

<PageShell>
  {#if patient}
    <div class="grid grid-cols-1 gap-4 lg:grid-cols-2">
      <Card>
        <SectionHeader title={$t('patients.activeDiagnoses')}>
          {#snippet actions()}{@render viewAll('diagnoses', $t('patients.viewAll'))}{/snippet}
        </SectionHeader>
        {#if diagnoses.length === 0}
          <p class="text-label text-fg-muted">{$t('patients.noActiveDiagnoses')}</p>
        {:else}
          <ul class="-mx-3">
            {#each diagnoses as diagnosis (diagnosis.id)}
              <li>
                <ListRow href={patientPath(id, 'diagnoses')}>
                  <span class="shrink-0 font-mono text-label text-fg">{diagnosis.icd10_code}</span>
                  <span class="min-w-0 truncate text-body text-fg-muted"
                    >{diagnosis.description}</span
                  >
                </ListRow>
              </li>
            {/each}
          </ul>
        {/if}
      </Card>

      <Card>
        <SectionHeader title={$t('patients.currentMedications')}>
          {#snippet actions()}{@render viewAll('medications', $t('patients.viewAll'))}{/snippet}
        </SectionHeader>
        {#if medications.length === 0}
          <p class="text-label text-fg-muted">{$t('patients.noCurrentMedications')}</p>
        {:else}
          <ul class="-mx-3">
            {#each medications as medication (medication.id)}
              <li>
                <ListRow
                  title={medication.substance}
                  secondary={[medication.dosage, medication.frequency].filter(Boolean).join(' · ')}
                  href={patientPath(id, 'medications')}
                />
              </li>
            {/each}
          </ul>
        {/if}
      </Card>

      <Card>
        <SectionHeader title={$t('outcomeScores.title')} />
        {#if loading}
          <Skeleton class="h-16 w-full" />
        {:else if scores.length === 0}
          <p class="text-label text-fg-muted">{$t('outcomeScores.noScores')}</p>
        {:else}
          <OutcomeSummary {scores} />
        {/if}
      </Card>

      <Card>
        <SectionHeader title={$t('patients.recentSessions')}>
          {#snippet actions()}{@render viewAll('sessions', $t('patients.viewAll'))}{/snippet}
        </SectionHeader>
        {#if loading}
          <Skeleton class="h-16 w-full" />
        {:else if recentSessions.length === 0}
          <p class="text-label text-fg-muted">{$t('sessions.noSessions')}</p>
        {:else}
          <ul class="-mx-3">
            {#each recentSessions as session (session.id)}
              <li>
                <ListRow
                  title={$sessionTypeLabel(session.session_type)}
                  meta={sessionMeta(session)}
                  href={patientPath(id, `sessions/${session.id}`)}
                />
              </li>
            {/each}
          </ul>
        {/if}
      </Card>

      {#if !loading && incomplete.length > 0}
        <Card>
          <SectionHeader
            title={$t('patients.incompleteNotes')}
            description={$t('patients.incompleteNotesHint')}
          />
          <ul class="-mx-3">
            {#each incomplete as session (session.id)}
              <li>
                <ListRow
                  title={$sessionTypeLabel(session.session_type)}
                  meta={$fmt.date(session.session_date)}
                  href={patientPath(id, `sessions/${session.id}`)}
                />
              </li>
            {/each}
          </ul>
        </Card>
      {/if}

      <Card class="lg:col-span-2">
        <SectionHeader title={$t('patients.details')}>
          {#snippet actions()}
            <Button size="sm" variant="ghost" href={patientPath(id, 'edit')}>
              {$t('patients.editPatient')}
            </Button>
          {/snippet}
        </SectionHeader>
        <dl class="grid grid-cols-1 gap-x-6 gap-y-3 sm:grid-cols-2 lg:grid-cols-3">
          {@render detail($t('patients.dateOfBirth'), $fmt.date(patient.date_of_birth), true)}
          {@render detail($t('patients.ahvNumber'), patient.ahv_number, true)}
          {@render detail(
            $t('patients.gender'),
            patient.gender ? genderLabel(patient.gender) : null
          )}
          {@render detail($t('patients.phone'), patient.phone, true)}
          {@render detail($t('patients.email'), patient.email)}
          {@render detail($t('patients.address'), patient.address)}
          {@render detail($t('patients.insurance'), patient.insurance)}
          {@render detail($t('patients.gpName'), patient.gp_name)}
          {@render detail($t('patients.gpAddress'), patient.gp_address)}
        </dl>
        {#if patient.notes}
          <div class="mt-4 border-t border-line-subtle pt-3">
            <p class="text-caption text-fg-muted">{$t('patients.notes')}</p>
            <p class="text-body text-fg whitespace-pre-line">{patient.notes}</p>
          </div>
        {/if}
        <p class="mt-4 border-t border-line-subtle pt-3 text-caption text-fg-subtle" data-numeric>
          {$t('patients.createdOn').replace('{date}', $fmt.date(patient.created_at))}
          <span aria-hidden="true" class="px-1">·</span>
          {$t('patients.updatedOn').replace('{date}', $fmt.date(patient.updated_at))}
        </p>
      </Card>
    </div>
  {/if}
</PageShell>
