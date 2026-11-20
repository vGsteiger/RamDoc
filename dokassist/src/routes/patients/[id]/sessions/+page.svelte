<script lang="ts">
  import { t } from '$lib/translations';
  import { get } from 'svelte/store';
  import { onMount } from 'svelte';
  import { page } from '$app/state';
  import { listSessionsForPatient, type Session } from '$lib/api';
  import SessionCard from '$lib/components/SessionCard.svelte';
  import {
    Alert,
    Button,
    EmptyState,
    ListSkeleton,
    PageShell,
    SectionHeader,
  } from '$lib/components/ui';
  import { CalendarClock, Plus } from 'lucide-svelte';

  const patientId = $derived(page.params.id!);

  let sessions = $state<Session[]>([]);
  let loading = $state(true);
  let error = $state<string | null>(null);

  onMount(async () => {
    await loadSessions();
  });

  async function loadSessions() {
    try {
      loading = true;
      error = null;
      sessions = await listSessionsForPatient(patientId);
    } catch (err) {
      error =
        get(t)('common.loadFailed') + ': ' + (err instanceof Error ? err.message : String(err));
      console.error('Failed to load sessions:', err);
    } finally {
      loading = false;
    }
  }
</script>

<PageShell>
  <SectionHeader title={$t('sessions.title')}>
    {#snippet actions()}
      <Button size="sm" href={`/patients/${patientId}/sessions/new`}>
        <Plus size={14} aria-hidden="true" />
        {$t('sessions.newSession')}
      </Button>
    {/snippet}
  </SectionHeader>

  {#if loading}
    <ListSkeleton count={4} />
  {:else if error}
    <Alert tone="danger">{error}</Alert>
  {:else if sessions.length === 0}
    <EmptyState icon={CalendarClock} title={$t('sessions.noSessions')}>
      {#snippet action()}
        <Button href={`/patients/${patientId}/sessions/new`}>{$t('sessions.addFirst')}</Button>
      {/snippet}
    </EmptyState>
  {:else}
    <ul class="-mx-3" aria-label={$t('sessions.title')}>
      {#each sessions as session (session.id)}
        <li>
          <SessionCard {session} href={`/patients/${patientId}/sessions/${session.id}`} />
        </li>
      {/each}
    </ul>
  {/if}
</PageShell>
