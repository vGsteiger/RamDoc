<script lang="ts">
  import { get } from 'svelte/store';
  import { t } from '$lib/translations';
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import {
    listAllSessions,
    listPatients,
    updateSession,
    type SessionWithPatient,
    type Patient,
  } from '$lib/api';
  import { addToast } from '$lib/stores/toast';

  let allSessions = $state<SessionWithPatient[]>([]);
  let viewMode = $state<'week' | 'month'>('week');

  // Patient-picker modal state (new session from time slot)
  let showNewSessionModal = $state(false);
  let modalDate = $state('');
  let modalTime = $state('');
  let modalPatientId = $state('');
  let allPatients = $state<Patient[]>([]);

  // Mark-completed modal state
  let showCompleteModal = $state(false);
  let completeSession = $state<SessionWithPatient | null>(null);
  let completeNotes = $state('');
  let isSavingCompletion = $state(false);

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

  // Generate 7 days for the week view
  const weekDays = $derived.by(() => {
    const days = [];
    for (let i = 0; i < 7; i++) {
      days.push(addDays(currentWeekStart, i));
    }
    return days;
  });

  // Hours for the week view (7:00 - 21:00)
  const hours = Array.from({ length: 15 }, (_, i) => i + 7); // 7 to 21

  // Get sessions for the current week, organized by date and time
  const weekSessions = $derived.by(() => {
    const startStr = toLocalISODate(currentWeekStart);
    const endStr = toLocalISODate(weekEnd);

    const filtered = allSessions.filter((s) => {
      const d = s.session.session_date.slice(0, 10);
      return d >= startStr && d <= endStr;
    });

    // Group by date and time
    const byDateTime = new Map<string, Map<number, SessionWithPatient[]>>();

    for (const s of filtered) {
      const dateStr = s.session.session_date.slice(0, 10);

      // Extract hour from scheduled_time if available
      let hour = -1; // -1 means no specific time
      if (s.session.scheduled_time) {
        const timeMatch = s.session.scheduled_time.match(/T(\d{2}):/);
        if (timeMatch) {
          hour = parseInt(timeMatch[1], 10);
        }
      }

      if (!byDateTime.has(dateStr)) {
        byDateTime.set(dateStr, new Map());
      }

      const dateMap = byDateTime.get(dateStr)!;
      if (!dateMap.has(hour)) {
        dateMap.set(hour, []);
      }
      dateMap.get(hour)!.push(s);
    }

    return byDateTime;
  });

  // Get month view data
  const monthStart = $derived(
    new Date(currentWeekStart.getFullYear(), currentWeekStart.getMonth(), 1)
  );
  const monthEnd = $derived(
    new Date(currentWeekStart.getFullYear(), currentWeekStart.getMonth() + 1, 0)
  );

  const monthLabel = $derived(
    currentWeekStart.toLocaleDateString('de-CH', { month: 'long', year: 'numeric' })
  );

  // Generate calendar days for month view (including padding for week alignment)
  const monthDays = $derived.by(() => {
    const days: (Date | null)[] = [];
    const firstDay = new Date(monthStart);
    const lastDay = new Date(monthEnd);

    // Pad beginning with nulls to align with Monday start
    const firstDayOfWeek = firstDay.getDay();
    const paddingStart = firstDayOfWeek === 0 ? 6 : firstDayOfWeek - 1;
    for (let i = 0; i < paddingStart; i++) {
      days.push(null);
    }

    // Add all days of the month
    for (let d = new Date(firstDay); d <= lastDay; d.setDate(d.getDate() + 1)) {
      days.push(new Date(d));
    }

    return days;
  });

  const monthSessions = $derived.by(() => {
    const startStr = toLocalISODate(monthStart);
    const endStr = toLocalISODate(monthEnd);

    const filtered = allSessions.filter((s) => {
      const d = s.session.session_date.slice(0, 10);
      return d >= startStr && d <= endStr;
    });

    // Group sessions by date
    const byDate = new Map<string, SessionWithPatient[]>();
    for (const s of filtered) {
      const d = s.session.session_date.slice(0, 10);
      if (!byDate.has(d)) {
        byDate.set(d, []);
      }
      byDate.get(d)!.push(s);
    }

    return byDate;
  });

  let expandedDate = $state<string | null>(null);

  function toggleDayExpansion(dateStr: string) {
    expandedDate = expandedDate === dateStr ? null : dateStr;
  }

  function prevWeek() {
    currentWeekStart = addDays(currentWeekStart, -7);
  }

  function nextWeek() {
    currentWeekStart = addDays(currentWeekStart, 7);
  }

  function prevMonth() {
    currentWeekStart = new Date(currentWeekStart.getFullYear(), currentWeekStart.getMonth() - 1, 1);
  }

  function nextMonth() {
    currentWeekStart = new Date(currentWeekStart.getFullYear(), currentWeekStart.getMonth() + 1, 1);
  }

  function goToToday() {
    currentWeekStart = getMonday(new Date());
  }

  function formatTime(hour: number): string {
    return `${hour.toString().padStart(2, '0')}:00`;
  }

  function getSessionStatus(
    session: SessionWithPatient
  ): 'scheduled' | 'completed' | 'note-pending' {
    // A session is completed if it has notes
    // Note pending if it has a scheduled_time but no notes yet
    // Scheduled if it has a future scheduled_time
    if (session.session.notes && session.session.notes.trim()) {
      return 'completed';
    }
    if (session.session.scheduled_time) {
      const scheduledDate = new Date(session.session.scheduled_time);
      if (scheduledDate < new Date()) {
        return 'note-pending';
      }
    }
    return 'scheduled';
  }

  function getStatusColor(status: 'scheduled' | 'completed' | 'note-pending'): string {
    switch (status) {
      case 'completed':
        return 'bg-success-subtle/20 text-success-fg border-success-line/30';
      case 'note-pending':
        return 'bg-warning-subtle/20 text-warning-fg border-warning-line/30';
      case 'scheduled':
      default:
        return 'bg-accent-subtle/20 text-accent-fg border-accent-line/30';
    }
  }

  async function handleTimeSlotClick(date: Date, hour: number) {
    modalDate = toLocalISODate(date);
    modalTime = formatTime(hour);
    modalPatientId = '';
    if (allPatients.length === 0) {
      allPatients = await listPatients();
    }
    showNewSessionModal = true;
  }

  function confirmNewSession() {
    if (!modalPatientId) return;
    showNewSessionModal = false;
    goto(`/patients/${modalPatientId}/sessions/new?date=${modalDate}&time=${modalTime}`);
  }

  function openCompleteModal(session: SessionWithPatient) {
    completeSession = session;
    completeNotes = session.session.notes ?? '';
    showCompleteModal = true;
  }

  async function confirmComplete() {
    if (!completeSession) return;
    isSavingCompletion = true;
    try {
      await updateSession(completeSession.session.id, {
        notes: completeNotes.trim() || '—',
      });
      allSessions = await listAllSessions(500);
      addToast(get(t)('sessions.markedCompleted'));
      showCompleteModal = false;
      completeSession = null;
      completeNotes = '';
    } catch {
      addToast(get(t)('sessions.updateFailed'), 'error');
    } finally {
      isSavingCompletion = false;
    }
  }

  onMount(async () => {
    allSessions = await listAllSessions(500);
  });
</script>

<div class="p-8 max-w-7xl mx-auto">
  <div class="flex items-center justify-between mb-6">
    <h1 class="text-display font-semibold text-fg">{$t('calendar.title')}</h1>

    <!-- View mode toggle -->
    <div class="flex gap-2">
      <button
        onclick={() => (viewMode = 'week')}
        class="h-7 px-2.5 rounded-control text-body transition-colors {viewMode === 'week'
          ? 'bg-accent text-on-accent'
          : 'bg-surface-selected text-fg hover:bg-surface-selected'}"
      >
        {$t('calendar.week')}
      </button>
      <button
        onclick={() => (viewMode = 'month')}
        class="h-7 px-2.5 rounded-control text-body transition-colors {viewMode === 'month'
          ? 'bg-accent text-on-accent'
          : 'bg-surface-selected text-fg hover:bg-surface-selected'}"
      >
        {$t('calendar.month')}
      </button>
    </div>
  </div>

  <!-- Navigation -->
  <div class="flex items-center gap-3 mb-6">
    <button
      onclick={viewMode === 'week' ? prevWeek : prevMonth}
      class="h-7 px-2.5 rounded-control bg-surface-selected hover:bg-surface-selected text-fg text-body transition-colors"
    >
      ‹
    </button>
    <span class="text-fg-muted text-body font-medium flex-1 text-center">
      {viewMode === 'week' ? weekLabel : monthLabel}
    </span>
    <button
      onclick={viewMode === 'week' ? nextWeek : nextMonth}
      class="h-7 px-2.5 rounded-control bg-surface-selected hover:bg-surface-selected text-fg text-body transition-colors"
    >
      ›
    </button>
    <button
      onclick={goToToday}
      class="h-7 px-2.5 rounded-control bg-surface-selected hover:bg-surface-selected text-fg text-body transition-colors"
    >
      {$t('calendar.today')}
    </button>
  </div>

  <!-- Sessions -->
  {#if viewMode === 'week'}
    <!-- Week view grid -->
    <div
      class="grid grid-cols-8 gap-px bg-surface-selected border-x border-b border-line rounded-b-card overflow-hidden"
    >
      {#each hours as hour}
        <!-- Hour label -->
        <div class="bg-surface-raised p-2 text-caption text-fg-muted text-right">
          {formatTime(hour)}
        </div>

        <!-- Day cells for this hour -->
        {#each weekDays as day}
          {@const dateStr = toLocalISODate(day)}
          {@const daySessions = weekSessions.get(dateStr)?.get(hour) || []}
          {@const untimed = weekSessions.get(dateStr)?.get(-1) || []}

          <div class="bg-surface-raised p-1 min-h-[60px] relative group">
            <!-- Empty slot - clickable to create new session -->
            {#if daySessions.length === 0}
              <button
                onclick={() => handleTimeSlotClick(day, hour)}
                aria-label={$t('calendar.newSessionAt')
                  .replace('{time}', formatTime(hour))
                  .replace('{date}', toLocalISODate(day))}
                class="absolute inset-0 opacity-0 group-hover:opacity-100 bg-accent-subtle hover:bg-accent-subtle/20 transition-opacity flex items-center justify-center text-caption text-accent-fg"
              >
                +
              </button>
            {/if}

            <!-- Sessions in this time slot -->
            {#each daySessions as session}
              {@const status = getSessionStatus(session)}
              <div
                class="w-full mb-1 p-1 rounded-card border text-caption {getStatusColor(
                  status
                )} group/card relative"
              >
                <button
                  onclick={() => goto(`/patients/${session.session.patient_id}/sessions`)}
                  class="w-full text-left hover:opacity-80 transition-opacity"
                >
                  <span class="sr-only"
                    >{status === 'completed'
                      ? $t('calendar.completed')
                      : status === 'note-pending'
                        ? $t('calendar.notePending')
                        : $t('calendar.scheduled')} —
                  </span>
                  <div class="font-medium truncate">{session.patient_name}</div>
                  {#if session.session.duration_minutes}
                    <div class="text-[10px] opacity-70">{session.session.duration_minutes} min</div>
                  {/if}
                </button>
                {#if status !== 'completed'}
                  <button
                    onclick={() => openCompleteModal(session)}
                    aria-label={$t('calendar.markCompletedFor').replace(
                      '{name}',
                      session.patient_name
                    )}
                    class="absolute top-0.5 right-0.5 opacity-0 group-hover/card:opacity-100 w-4 h-4 flex items-center justify-center rounded-control bg-success text-on-success text-[10px] hover:bg-success transition-colors"
                    title={$t('calendar.markCompleted')}>✓</button
                  >
                {/if}
              </div>
            {/each}

            <!-- Show untimed sessions in the first hour only -->
            {#if hour === 7}
              {#each untimed as session}
                {@const status = getSessionStatus(session)}
                <div
                  class="w-full mb-1 p-1 rounded-card border text-caption {getStatusColor(
                    status
                  )} opacity-60 group/card relative"
                >
                  <button
                    onclick={() => goto(`/patients/${session.session.patient_id}/sessions`)}
                    class="w-full text-left hover:opacity-80 transition-opacity"
                  >
                    <span class="sr-only"
                      >{status === 'completed'
                        ? $t('calendar.completed')
                        : status === 'note-pending'
                          ? $t('calendar.notePending')
                          : $t('calendar.scheduled')} —
                    </span>
                    <div class="font-medium truncate">{session.patient_name}</div>
                    <div class="text-[10px]">{$t('sessions.noTime')}</div>
                  </button>
                  {#if status !== 'completed'}
                    <button
                      onclick={() => openCompleteModal(session)}
                      aria-label={$t('calendar.markCompletedFor').replace(
                        '{name}',
                        session.patient_name
                      )}
                      class="absolute top-0.5 right-0.5 opacity-0 group-hover/card:opacity-100 w-4 h-4 flex items-center justify-center rounded-control bg-success text-on-success text-[10px] hover:bg-success transition-colors"
                      title={$t('calendar.markCompleted')}>✓</button
                    >
                  {/if}
                </div>
              {/each}
            {/if}
          </div>
        {/each}
      {/each}
    </div>

    <!-- Legend -->
    <div class="mt-4 flex gap-4 text-caption text-fg-muted">
      <div class="flex items-center gap-2">
        <span class="w-3 h-3 rounded-card border bg-accent-subtle border-accent-line"></span>
        <span>{$t('calendar.scheduled')}</span>
      </div>
      <div class="flex items-center gap-2">
        <span class="w-3 h-3 rounded-card border bg-success-subtle border-success-line"></span>
        <span>{$t('calendar.completed')}</span>
      </div>
      <div class="flex items-center gap-2">
        <span class="w-3 h-3 rounded-card border bg-warning-subtle border-warning-line"></span>
        <span>{$t('calendar.notePending')}</span>
      </div>
    </div>
  {:else}
    <!-- Month view - calendar grid with session counts -->
    <div>
      <!-- Month calendar grid -->
      <div
        class="grid grid-cols-7 gap-px bg-surface-selected border border-line rounded-card overflow-hidden"
      >
        <!-- Day headers -->
        {#each ['Mo', 'Di', 'Mi', 'Do', 'Fr', 'Sa', 'So'] as dayName}
          <div class="bg-surface-raised p-2 text-center text-caption font-semibold text-fg">
            {dayName}
          </div>
        {/each}

        <!-- Calendar days -->
        {#each monthDays as day}
          {#if day === null}
            <div class="bg-surface-sunken p-2 min-h-[80px]"></div>
          {:else}
            {@const dateStr = toLocalISODate(day)}
            {@const daySessions = monthSessions.get(dateStr) || []}
            {@const isToday = dateStr === toLocalISODate(new Date())}

            <div class="bg-surface-raised p-2 min-h-[80px] flex flex-col">
              <div class="flex items-center justify-between mb-1">
                <span class="text-body {isToday ? 'font-semibold text-accent-fg' : 'text-fg'}">
                  {day.getDate()}
                </span>
                {#if daySessions.length > 0}
                  <button
                    onclick={() => toggleDayExpansion(dateStr)}
                    class="text-caption px-1.5 py-0.5 rounded-control bg-accent-subtle text-accent-fg hover:bg-accent-subtle transition-colors"
                  >
                    {daySessions.length}
                  </button>
                {/if}
              </div>

              <!-- Show abbreviated session list or expanded view -->
              {#if expandedDate === dateStr && daySessions.length > 0}
                <div class="space-y-1 mt-1">
                  {#each daySessions as session}
                    {@const status = getSessionStatus(session)}
                    <button
                      onclick={() => goto(`/patients/${session.session.patient_id}/sessions`)}
                      class="w-full text-left p-1 rounded-control border text-[10px] {getStatusColor(
                        status
                      )} hover:opacity-80 transition-opacity"
                    >
                      <div class="font-medium truncate">{session.patient_name}</div>
                      {#if session.session.scheduled_time}
                        {@const timeMatch =
                          session.session.scheduled_time.match(/T(\d{2}):(\d{2})/)}
                        {#if timeMatch}
                          <div class="opacity-70">{timeMatch[1]}:{timeMatch[2]}</div>
                        {/if}
                      {/if}
                    </button>
                  {/each}
                </div>
              {:else if daySessions.length > 0}
                <!-- Show mini dots for sessions -->
                <div class="flex flex-wrap gap-1 mt-1">
                  {#each daySessions.slice(0, 3) as session}
                    {@const status = getSessionStatus(session)}
                    <div
                      class="w-2 h-2 rounded-full {status === 'completed'
                        ? 'bg-success'
                        : status === 'note-pending'
                          ? 'bg-warning'
                          : 'bg-accent'}"
                    ></div>
                  {/each}
                </div>
              {/if}
            </div>
          {/if}
        {/each}
      </div>

      <!-- Legend -->
      <div class="mt-4 flex gap-4 text-caption text-fg-muted">
        <div class="flex items-center gap-2">
          <span class="w-3 h-3 rounded-card border bg-accent-subtle border-accent-line"></span>
          <span>{$t('calendar.scheduled')}</span>
        </div>
        <div class="flex items-center gap-2">
          <span class="w-3 h-3 rounded-card border bg-success-subtle border-success-line"></span>
          <span>{$t('calendar.completed')}</span>
        </div>
        <div class="flex items-center gap-2">
          <span class="w-3 h-3 rounded-card border bg-warning-subtle border-warning-line"></span>
          <span>{$t('calendar.notePending')}</span>
        </div>
      </div>
    </div>
  {/if}
</div>

<!-- Patient-picker modal for new session from time slot -->
{#if showNewSessionModal}
  <div
    class="fixed inset-0 bg-scrim flex items-center justify-center z-50"
    role="dialog"
    aria-modal="true"
    aria-label={$t('calendar.newSession')}
  >
    <div class="bg-surface-raised rounded-card shadow-modal p-6 w-96 max-w-full mx-4">
      <h2 class="text-heading font-semibold text-fg mb-1">{$t('sessions.newSession')}</h2>
      <p class="text-body text-fg-muted mb-4">
        {modalDate.split('-').reverse().join('.')} um {modalTime}
      </p>

      <label for="modal-patient" class="block text-body font-medium text-fg-muted mb-2">
        {$t('common.patient')}
      </label>
      <select
        id="modal-patient"
        bind:value={modalPatientId}
        class="w-full px-3 py-2 bg-surface-raised border border-line rounded-control text-fg focus:outline-none focus:ring-2 focus:ring-accent/30 mb-6"
      >
        <option value="">{$t('common.selectPatient')}</option>
        {#each allPatients as patient}
          <option value={patient.id}>{patient.last_name}, {patient.first_name}</option>
        {/each}
      </select>

      <div class="flex gap-3 justify-end">
        <button
          onclick={() => (showNewSessionModal = false)}
          class="h-8 px-3 text-body rounded-control bg-surface-selected hover:bg-surface-selected text-fg transition-colors"
        >
          {$t('common.cancel')}
        </button>
        <button
          onclick={confirmNewSession}
          disabled={!modalPatientId}
          class="h-8 px-3 text-body rounded-control bg-accent hover:bg-accent-hover disabled:opacity-50 disabled:cursor-not-allowed text-on-accent transition-colors"
        >
          {$t('common.continue')}
        </button>
      </div>
    </div>
  </div>
{/if}

<!-- Mark-completed modal -->
{#if showCompleteModal && completeSession}
  <div
    class="fixed inset-0 bg-scrim flex items-center justify-center z-50"
    role="dialog"
    aria-modal="true"
    aria-label={$t('calendar.completeSession')}
  >
    <div class="bg-surface-raised rounded-card shadow-modal p-6 w-96 max-w-full mx-4">
      <h2 class="text-heading font-semibold text-fg mb-1">{$t('sessions.complete')}</h2>
      <p class="text-body text-fg-muted mb-4">
        {completeSession.patient_name} · {completeSession.session.session_date
          .slice(0, 10)
          .split('-')
          .reverse()
          .join('.')}
      </p>

      <label for="complete-notes" class="block text-body font-medium text-fg-muted mb-2">
        {$t('sessions.notesOptional')}
      </label>
      <textarea
        id="complete-notes"
        bind:value={completeNotes}
        rows="4"
        placeholder={$t('sessions.notesPlaceholder')}
        class="w-full px-3 py-2 bg-surface-raised border border-line rounded-control text-fg focus:outline-none focus:ring-2 focus:ring-accent/30 resize-none mb-6"
      ></textarea>

      <div class="flex gap-3 justify-end">
        <button
          onclick={() => {
            showCompleteModal = false;
            completeSession = null;
          }}
          disabled={isSavingCompletion}
          class="h-8 px-3 text-body rounded-control bg-surface-selected hover:bg-surface-selected text-fg transition-colors disabled:opacity-50"
        >
          {$t('common.cancel')}
        </button>
        <button
          onclick={confirmComplete}
          disabled={isSavingCompletion}
          class="h-8 px-3 text-body rounded-control bg-success hover:bg-success-hover disabled:opacity-50 disabled:cursor-not-allowed text-on-success transition-colors"
        >
          {isSavingCompletion ? $t('common.saving') : $t('calendar.markCompletedAction')}
        </button>
      </div>
    </div>
  </div>
{/if}
