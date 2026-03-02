<script lang="ts">
  import { onMount } from 'svelte';
  import { page } from '$app/stores';
  import { goto } from '$app/navigation';
  import { listSessionsForPatient, type Session } from '$lib/api';
  import SessionCard from '$lib/components/SessionCard.svelte';

  const patientId = $derived($page.params.id);

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
      error = 'Fehler beim Laden der Sitzungen: ' + (err instanceof Error ? err.message : String(err));
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
  <div class="flex justify-between items-center mb-6">
    <h1 class="text-2xl font-bold text-gray-100">Sitzungen</h1>
    <button
      class="px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 transition-colors"
      onclick={handleNewSession}
    >
      + Neue Sitzung
    </button>
  </div>

  {#if loading}
    <div class="flex justify-center items-center py-12">
      <div class="text-gray-400">Lädt...</div>
    </div>
  {:else if error}
    <div class="bg-red-500/10 border border-red-500/30 text-red-400 p-4 rounded-lg">
      {error}
    </div>
  {:else if sessions.length === 0}
    <div class="text-center py-12">
      <p class="text-gray-400 mb-4">Noch keine Sitzungen vorhanden</p>
      <button
        class="px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 transition-colors"
        onclick={handleNewSession}
      >
        Erste Sitzung erfassen
      </button>
    </div>
  {:else}
    <div class="grid gap-4">
      {#each sessions as session (session.id)}
        <SessionCard {session} onclick={() => handleSessionClick(session.id)} />
      {/each}
    </div>
  {/if}
</div>
