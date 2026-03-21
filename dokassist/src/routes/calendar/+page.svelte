<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { listAllSessions, type SessionWithPatient } from '$lib/api';

  let allSessions = $state<SessionWithPatient[]>([]);

  function getMonday(date: Date): Date {
    const d = new Date(date);
    const day = d.getDay(); // 0=Sun, 1=Mon … 6=Sat
    const diff = day === 0 ? -6 : 1 - day;
    d.setDate(d.getDate() + diff);
    d.setHours(0, 0, 0, 0);
    return d;
  }

  function addDays(date: Date, days: number): Date {
    const d = new Date(date);
    d.setDate(d.getDate() + days);
    return d;
  }

  function toLocalISODate(date: Date): string {
    const y = date.getFullYear();
    const m = String(date.getMonth() + 1).padStart(2, '0');
    const d = String(date.getDate()).padStart(2, '0');
    return `${y}-${m}-${d}`;
  }

  let currentWeekStart = $state(getMonday(new Date()));

  const weekEnd = $derived(addDays(currentWeekStart, 6));

  const weekLabel = $derived(
    `${currentWeekStart.toLocaleDateString('de-CH', { day: 'numeric', month: 'long' })} – ` +
      `${weekEnd.toLocaleDateString('de-CH', { day: 'numeric', month: 'long', year: 'numeric' })}`
  );

  const weekSessions = $derived.by(() => {
    const startStr = toLocalISODate(currentWeekStart);
    const endStr = toLocalISODate(weekEnd);

    const filtered = allSessions.filter((s) => {
      const d = s.session.session_date.slice(0, 10);
      return d >= startStr && d <= endStr;
    });

    const byDate = new Map<string, SessionWithPatient[]>();
    for (const s of filtered) {
      const d = s.session.session_date.slice(0, 10);
      if (!byDate.has(d)) byDate.set(d, []);
      byDate.get(d)!.push(s);
    }

    return [...byDate.entries()].sort((a, b) => a[0].localeCompare(b[0]));
  });

  function prevWeek() {
    currentWeekStart = addDays(currentWeekStart, -7);
  }

  function nextWeek() {
    currentWeekStart = addDays(currentWeekStart, 7);
  }

  function goToToday() {
    currentWeekStart = getMonday(new Date());
  }

  function formatDate(isoDate: string): string {
    const d = new Date(isoDate + 'T00:00:00');
    return d.toLocaleDateString('de-CH', {
      weekday: 'long',
      day: 'numeric',
      month: 'long',
      year: 'numeric',
    });
  }

  const SESSION_TYPE_LABELS: Record<string, string> = {
    initial: 'Erstgespräch',
    followup: 'Folgegespräch',
    crisis: 'Krisenintervention',
    group: 'Gruppentherapie',
    family: 'Familiengespräch',
    supervision: 'Supervision',
    other: 'Sonstige',
  };

  onMount(async () => {
    allSessions = await listAllSessions(500);
  });
</script>

<div class="p-8 max-w-2xl">
  <h1 class="text-2xl font-bold text-gray-900 dark:text-gray-100 mb-6">Kalender</h1>

  <!-- Week navigation -->
  <div class="flex items-center gap-3 mb-6">
    <button
      onclick={prevWeek}
      class="px-3 py-1.5 rounded-lg bg-gray-200 dark:bg-gray-700 hover:bg-gray-300 dark:hover:bg-gray-600 text-gray-900 dark:text-gray-100 text-sm transition-colors"
    >
      ‹
    </button>
    <span class="text-gray-700 dark:text-gray-200 text-sm font-medium flex-1 text-center"
      >{weekLabel}</span
    >
    <button
      onclick={nextWeek}
      class="px-3 py-1.5 rounded-lg bg-gray-200 dark:bg-gray-700 hover:bg-gray-300 dark:hover:bg-gray-600 text-gray-900 dark:text-gray-100 text-sm transition-colors"
    >
      ›
    </button>
    <button
      onclick={goToToday}
      class="px-3 py-1.5 rounded-lg bg-gray-200 dark:bg-gray-700 hover:bg-gray-300 dark:hover:bg-gray-600 text-gray-900 dark:text-gray-100 text-sm transition-colors"
    >
      Heute
    </button>
  </div>

  <!-- Sessions -->
  {#if weekSessions.length === 0}
    <p class="text-gray-500 dark:text-gray-400 text-sm">Keine Sitzungen diese Woche.</p>
  {:else}
    <div class="space-y-6">
      {#each weekSessions as [date, sessions]}
        <div>
          <h2
            class="text-xs font-semibold text-gray-500 dark:text-gray-400 uppercase tracking-wide mb-2"
          >
            {formatDate(date)}
          </h2>
          <div class="space-y-2">
            {#each sessions as item}
              <button
                onclick={() => goto(`/patients/${item.session.patient_id}/sessions`)}
                class="w-full text-left bg-white dark:bg-gray-800 hover:bg-gray-100 dark:hover:bg-gray-700 border border-gray-200 dark:border-gray-700 rounded-lg px-4 py-3 flex items-center gap-3 transition-colors"
              >
                <div class="flex-1 min-w-0">
                  <p class="text-sm font-medium text-gray-900 dark:text-gray-100 truncate">
                    {item.patient_name}
                  </p>
                </div>
                <span
                  class="text-xs px-2 py-0.5 rounded-full bg-blue-100 dark:bg-blue-900 text-blue-700 dark:text-blue-200 shrink-0"
                >
                  {SESSION_TYPE_LABELS[item.session.session_type] ?? item.session.session_type}
                </span>
                {#if item.session.duration_minutes}
                  <span class="text-xs text-gray-500 dark:text-gray-400 shrink-0"
                    >{item.session.duration_minutes} Min.</span
                  >
                {/if}
              </button>
            {/each}
          </div>
        </div>
      {/each}
    </div>
  {/if}
</div>
