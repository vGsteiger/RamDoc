<script lang="ts">
  import { t } from '$lib/translations';
  import { get } from 'svelte/store';
  import { onMount } from 'svelte';
  import { page } from '$app/state';
  import { goto } from '$app/navigation';
  import { listSessionsForPatient, type Session } from '$lib/api';
  import SessionCard from '$lib/components/SessionCard.svelte';
  import { Alert, Button, EmptyState, PageHeader, Spinner } from '$lib/components/ui';
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

  function handleNewSession() {
    goto(`/patients/${patientId}/sessions/new`);
  }

  function handleSessionClick(sessionId: string) {
    goto(`/patients/${patientId}/sessions/${sessionId}`);
  }
</script>

<div class="p-8">
  <PageHeader title={$t('sessions.title')}>
    {#snippet actions()}
      <Button variant="primary" onclick={handleNewSession}>
        <Plus size={14} />
        {$t('sessions.newSession')}
      </Button>
    {/snippet}
  </PageHeader>

  {#if loading}
    <div class="flex justify-center py-12">
      <Spinner label={$t('common.loading')} />
    </div>
  {:else if error}
    <Alert tone="danger">{error}</Alert>
  {:else if sessions.length === 0}
    <EmptyState icon={CalendarClock} title={$t('sessions.noSessions')}>
      {#snippet action()}
        <Button variant="primary" onclick={handleNewSession}>{$t('sessions.addFirst')}</Button>
      {/snippet}
    </EmptyState>
  {:else}
    <div class="grid gap-2">
      {#each sessions as session (session.id)}
        <SessionCard {session} onclick={() => handleSessionClick(session.id)} />
      {/each}
    </div>
  {/if}
</div>
