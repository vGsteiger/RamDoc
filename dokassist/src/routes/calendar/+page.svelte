<script lang="ts">
  import { get } from 'svelte/store';
  import { onMount } from 'svelte';
  import { goto, replaceState } from '$app/navigation';
  import { page } from '$app/state';
  import {
    AlertTriangle,
    CheckCircle2,
    ChevronLeft,
    ChevronRight,
    Circle,
    Plus,
  } from 'lucide-svelte';
  import {
    listAllSessions,
    listPatients,
    updateSession,
    type SessionWithPatient,
    type Patient,
  } from '$lib/api';
  import {
    Button,
    Dialog,
    EmptyState,
    Field,
    IconButton,
    Input,
    PageShell,
    SegmentedControl,
    Select,
    StatusIndicator,
    Textarea,
  } from '$lib/components/ui';
  import {
    FIRST_HOUR,
    LAST_HOUR,
    DEFAULT_DURATION,
    addDays,
    addMonths,
    dayHeaders,
    groupByDate,
    layoutDay,
    minutesToTime,
    monthCells,
    parseISODate,
    parseView,
    rangeLabel,
    sessionStartMinutes,
    sessionStatus,
    startOfDay,
    startOfWeek,
    toISODate,
    weekdayNames,
    type CalendarView,
    type SessionStatus,
  } from '$lib/calendar';
  import { fmt } from '$lib/format';
  import { language } from '$lib/stores/language';
  import { addToast } from '$lib/stores/toast';
  import { t } from '$lib/translations';
  import { sessionTypeLabel } from '$lib/translations/labels';

  /** Height of one hour row in the time grid, px. */
  const HOUR_PX = 64;
  const HOURS = Array.from({ length: LAST_HOUR - FIRST_HOUR + 1 }, (_, i) => FIRST_HOUR + i);
  const GRID_HEIGHT = HOURS.length * HOUR_PX;
  /** Below this content width the week view becomes a Mon–Fri work week. */
  const COMPACT_WIDTH = 760;

  let allSessions = $state<SessionWithPatient[]>([]);
  let view = $state<CalendarView>('week');
  let anchor = $state(startOfDay(new Date()));
  let now = $state(new Date());
  let contentWidth = $state(0);
  let mounted = $state(false);

  // New-session dialog (from a time slot or the header button)
  let newOpen = $state(false);
  let modalDate = $state('');
  let modalTime = $state('');
  let modalPatientId = $state('');
  let allPatients = $state<Patient[]>([]);

  // Session details and mark-completed dialogs
  let detailOpen = $state(false);
  let detailSession = $state<SessionWithPatient | null>(null);
  let completeOpen = $state(false);
  let completeSession = $state<SessionWithPatient | null>(null);
  let completeNotes = $state('');
  let isSavingCompletion = $state(false);

  const STATUS: Record<
    SessionStatus,
    { tone: 'idle' | 'ok' | 'warn'; key: string; card: string; icon: typeof Circle }
  > = {
    scheduled: {
      tone: 'idle',
      key: 'calendar.scheduled',
      card: 'border-line-strong bg-surface-raised',
      icon: Circle,
    },
    completed: {
      tone: 'ok',
      key: 'calendar.completed',
      card: 'border-success-line bg-success-subtle',
      icon: CheckCircle2,
    },
    'note-pending': {
      tone: 'warn',
      key: 'calendar.notePending',
      card: 'border-warning-line bg-warning-subtle',
      icon: AlertTriangle,
    },
  };
  const STATUS_ICON_TONE: Record<SessionStatus, string> = {
    scheduled: 'text-fg-subtle',
    completed: 'text-success-fg',
    'note-pending': 'text-warning-fg',
  };

  let compact = $derived(contentWidth > 0 && contentWidth < COMPACT_WIDTH);
  let dayCount = $derived(view === 'day' ? 1 : view === 'week' && compact ? 5 : 7);
  let rangeStart = $derived(view === 'day' ? anchor : startOfWeek(anchor));
  let headers = $derived(dayHeaders(rangeStart, dayCount, $language, now));
  let weekHeaders = $derived(dayHeaders(startOfWeek(anchor), 7, $language, now));
  let byDate = $derived(groupByDate(allSessions));
  let label = $derived(rangeLabel(view, anchor, $language, view === 'week' ? dayCount : 7));
  let nowMinutes = $derived(now.getHours() * 60 + now.getMinutes());
  let gridColumns = $derived(`3.25rem repeat(${dayCount}, minmax(0, 1fr))`);

  let untimedByDay = $derived(
    headers.map((h) => (byDate.get(h.iso) ?? []).filter((s) => sessionStartMinutes(s) === null))
  );
  let hasUntimed = $derived(untimedByDay.some((list) => list.length > 0));

  let weekendHidden = $derived(
    view === 'week' && compact
      ? weekHeaders
          .filter((h) => h.isWeekend)
          .reduce((n, h) => n + (byDate.get(h.iso)?.length ?? 0), 0)
      : 0
  );

  let agendaDays = $derived(
    weekHeaders
      .map((h) => ({ header: h, sessions: byDate.get(h.iso) ?? [] }))
      .filter((d) => d.sessions.length > 0)
  );

  let viewOptions = $derived([
    { value: 'day', label: $t('calendar.day') },
    { value: 'week', label: $t('calendar.week') },
    { value: 'month', label: $t('calendar.month') },
    { value: 'agenda', label: $t('calendar.agenda') },
  ]);

  let navLabels = $derived(
    view === 'day'
      ? [$t('calendar.prevDay'), $t('calendar.nextDay')]
      : view === 'month'
        ? [$t('calendar.prevMonth'), $t('calendar.nextMonth')]
        : [$t('calendar.prevWeek'), $t('calendar.nextWeek')]
  );

  function step(direction: -1 | 1) {
    if (view === 'day') anchor = addDays(anchor, direction);
    else if (view === 'month') anchor = addMonths(anchor, direction);
    else anchor = addDays(anchor, 7 * direction);
  }

  function goToToday() {
    anchor = startOfDay(new Date());
  }

  function showDay(date: Date) {
    anchor = startOfDay(date);
    view = 'day';
  }

  function statusOf(s: SessionWithPatient): SessionStatus {
    return sessionStatus(s, now);
  }

  function timeRange(s: SessionWithPatient): string {
    const start = sessionStartMinutes(s);
    if (start === null) return $t('sessions.noTime');
    const end = start + (s.session.duration_minutes || DEFAULT_DURATION);
    return `${minutesToTime(start)}–${minutesToTime(end % (24 * 60))}`;
  }

  function eventLabel(s: SessionWithPatient): string {
    return [
      timeRange(s),
      s.patient_name,
      $sessionTypeLabel(s.session.session_type),
      $t(STATUS[statusOf(s)].key),
    ].join(', ');
  }

  async function ensurePatients() {
    if (allPatients.length === 0) allPatients = await listPatients();
  }

  async function openNewSession(date: Date, minutes: number | null) {
    modalDate = toISODate(date);
    modalTime = minutes === null ? '' : minutesToTime(minutes);
    modalPatientId = '';
    await ensurePatients();
    newOpen = true;
  }

  function confirmNewSession() {
    if (!modalPatientId || !modalDate) return;
    newOpen = false;
    const time = modalTime ? `&time=${modalTime}` : '';
    goto(`/patients/${modalPatientId}/sessions/new?date=${modalDate}${time}`);
  }

  function openDetails(s: SessionWithPatient) {
    detailSession = s;
    detailOpen = true;
  }

  function openCompleteModal(s: SessionWithPatient) {
    detailOpen = false;
    completeSession = s;
    completeNotes = s.session.notes ?? '';
    completeOpen = true;
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
      completeOpen = false;
      completeSession = null;
      completeNotes = '';
    } catch {
      addToast(get(t)('sessions.updateFailed'), 'error');
    } finally {
      isSavingCompletion = false;
    }
  }

  function longDate(date: Date) {
    return $fmt.date(date, { style: 'long' });
  }

  function sessionHref(s: SessionWithPatient) {
    return `/patients/${s.session.patient_id}/sessions/${s.session.id}`;
  }

  /** Half-hour slots for the new-session time picker. */
  const TIME_SLOTS = Array.from({ length: (LAST_HOUR - FIRST_HOUR + 1) * 2 }, (_, i) =>
    minutesToTime(FIRST_HOUR * 60 + i * 30)
  );

  onMount(() => {
    const params = page.url.searchParams;
    view = parseView(params.get('view')) ?? 'week';
    anchor = parseISODate(params.get('date')) ?? startOfDay(new Date());
    mounted = true;

    void listAllSessions(500).then((sessions) => (allSessions = sessions));
    const timer = setInterval(() => (now = new Date()), 60_000);
    return () => clearInterval(timer);
  });

  // Keep the view and date in the URL so a reload or a shared link reopens
  // the same calendar.
  $effect(() => {
    const search = `?view=${view}&date=${toISODate(anchor)}`;
    if (mounted && page.url.search !== search) {
      try {
        replaceState(search, {});
      } catch {
        // The router may not be ready on the very first frame; harmless.
      }
    }
  });
</script>

{#snippet statusIcon(status: SessionStatus, size = 12)}
  {@const Icon = STATUS[status].icon}
  <Icon {size} class="shrink-0 {STATUS_ICON_TONE[status]}" aria-hidden="true" />
{/snippet}

{#snippet legend()}
  <div class="mt-4 flex flex-wrap gap-x-5 gap-y-2" role="group" aria-label={$t('calendar.legend')}>
    {#each ['scheduled', 'note-pending', 'completed'] as const as status (status)}
      <StatusIndicator tone={STATUS[status].tone} label={$t(STATUS[status].key)} />
    {/each}
  </div>
{/snippet}

{#snippet timeGrid()}
  <div class="rounded-card border border-line bg-surface-raised">
    <!-- Weekday + date header; sticks while the hours scroll. -->
    <div
      class="sticky top-0 z-20 grid rounded-t-card border-b border-line bg-surface-raised"
      style:grid-template-columns={gridColumns}
    >
      <div></div>
      {#each headers as h (h.iso)}
        <div
          class="flex min-w-0 items-center justify-center gap-1.5 border-l border-line-subtle px-1 py-2 {h.isToday
            ? 'text-fg'
            : 'text-fg-muted'}"
          aria-current={h.isToday ? 'date' : undefined}
        >
          <span class="truncate text-label {h.isToday ? 'font-medium' : ''}">{h.weekday}</span>
          <span
            class="flex size-6 shrink-0 items-center justify-center rounded-full text-label {h.isToday
              ? 'bg-accent font-medium text-on-accent'
              : 'text-fg'}"
            data-numeric>{h.day}</span
          >
          {#if h.isToday}<span class="sr-only">{$t('calendar.todayMarker')}</span>{/if}
        </div>
      {/each}
    </div>

    {#if hasUntimed}
      <div class="grid border-b border-line" style:grid-template-columns={gridColumns}>
        <div class="px-1 py-1.5 text-right text-caption leading-tight text-fg-subtle">
          {$t('sessions.noTime')}
        </div>
        {#each headers as h, i (h.iso)}
          <div class="min-w-0 space-y-0.5 border-l border-line-subtle p-0.5">
            {#each untimedByDay[i] as s (s.session.id)}
              {@const status = statusOf(s)}
              <button
                type="button"
                class="flex w-full min-w-0 items-center gap-1 rounded-control border px-1.5 py-0.5 text-left text-caption text-fg transition-colors duration-150 ease-standard hover:border-line-strong {STATUS[
                  status
                ].card}"
                aria-label={eventLabel(s)}
                title={eventLabel(s)}
                onclick={() => openDetails(s)}
              >
                {@render statusIcon(status)}
                <span class="truncate font-medium">{s.patient_name}</span>
              </button>
            {/each}
          </div>
        {/each}
      </div>
    {/if}

    <div class="grid" style:grid-template-columns={gridColumns}>
      <div class="relative" style:height="{GRID_HEIGHT}px" aria-hidden="true">
        {#each HOURS as hour, i (hour)}
          <span
            class="absolute right-1.5 pt-0.5 text-caption text-fg-subtle"
            style:top="{i * HOUR_PX}px"
            data-numeric>{minutesToTime(hour * 60)}</span
          >
        {/each}
      </div>

      {#each headers as h (h.iso)}
        <div
          class="relative min-w-0 border-l border-line-subtle {h.isToday ? 'bg-surface' : ''}"
          style:height="{GRID_HEIGHT}px"
        >
          <!-- Empty slots: click to schedule. The header's New session
               button offers the same without pointing at a slot. -->
          {#each HOURS as hour, i (hour)}
            <button
              type="button"
              class="group absolute inset-x-0 flex items-start justify-end border-t border-line-subtle px-1 text-caption text-fg-subtle transition-colors duration-150 ease-standard hover:bg-surface-hover focus-visible:bg-surface-hover"
              style:top="{i * HOUR_PX}px"
              style:height="{HOUR_PX}px"
              aria-label={$t('calendar.newSessionAt')
                .replace('{time}', minutesToTime(hour * 60))
                .replace('{date}', $fmt.date(h.date))}
              onclick={() => openNewSession(h.date, hour * 60)}
            >
              <Plus
                size={12}
                class="mt-1 opacity-0 group-hover:opacity-100 group-focus-visible:opacity-100"
                aria-hidden="true"
              />
            </button>
          {/each}

          {#each layoutDay(byDate.get(h.iso) ?? []) as ev (ev.item.session.id)}
            {@const status = statusOf(ev.item)}
            {@const top = Math.max(0, ((ev.start - FIRST_HOUR * 60) * HOUR_PX) / 60)}
            {@const height = Math.max(
              18,
              Math.min(((ev.end - ev.start) * HOUR_PX) / 60, GRID_HEIGHT - top) - 2
            )}
            <button
              type="button"
              class="absolute z-10 flex min-w-0 flex-col overflow-hidden rounded-control border px-1.5 py-0.5 text-left text-caption leading-tight text-fg transition-colors duration-150 ease-standard hover:border-fg-subtle {STATUS[
                status
              ].card}"
              style:top="{Math.min(top, GRID_HEIGHT - 18) + 1}px"
              style:height="{height}px"
              style:left="calc({(ev.column / ev.columns) * 100}% + 2px)"
              style:width="calc({100 / ev.columns}% - 4px)"
              aria-label={eventLabel(ev.item)}
              title={eventLabel(ev.item)}
              onclick={() => openDetails(ev.item)}
            >
              {#if height >= 46}
                <span class="flex min-w-0 items-center gap-1 text-fg-muted" data-numeric>
                  {@render statusIcon(status)}
                  <span class="truncate">{minutesToTime(ev.start)}</span>
                </span>
                <span class="truncate font-medium">{ev.item.patient_name}</span>
                <span class="truncate text-fg-muted">
                  {$sessionTypeLabel(ev.item.session.session_type)}
                </span>
              {:else}
                <span class="flex min-w-0 items-center gap-1">
                  {@render statusIcon(status)}
                  <span class="shrink-0 text-fg-muted" data-numeric>{minutesToTime(ev.start)}</span>
                  <span class="truncate font-medium">{ev.item.patient_name}</span>
                </span>
                {#if height >= 30}
                  <span class="truncate text-fg-muted">
                    {$sessionTypeLabel(ev.item.session.session_type)}
                  </span>
                {/if}
              {/if}
            </button>
          {/each}

          {#if h.isToday && nowMinutes >= FIRST_HOUR * 60 && nowMinutes < (LAST_HOUR + 1) * 60}
            <div
              class="pointer-events-none absolute inset-x-0 z-10 h-0.5 bg-accent"
              style:top="{((nowMinutes - FIRST_HOUR * 60) * HOUR_PX) / 60}px"
            >
              <span class="absolute -left-1 -top-1 size-2.5 rounded-full bg-accent"></span>
              <span class="sr-only">{$t('calendar.now')} {minutesToTime(nowMinutes)}</span>
            </div>
          {/if}
        </div>
      {/each}
    </div>
  </div>
{/snippet}

{#snippet monthGrid()}
  {@const names = weekdayNames($language)}
  {@const todayIso = toISODate(now)}
  <div class="rounded-card border border-line bg-surface-raised">
    <div class="grid grid-cols-7 border-b border-line">
      {#each names as name, i (i)}
        <div class="px-2 py-2 text-center text-label text-fg-muted">{name}</div>
      {/each}
    </div>
    <div class="grid grid-cols-7">
      {#each monthCells(anchor) as day, i (i)}
        {#if day === null}
          <div class="min-h-24 border-b border-r border-line-subtle bg-surface-sunken"></div>
        {:else}
          {@const iso = toISODate(day)}
          {@const sessions = byDate.get(iso) ?? []}
          {@const isToday = iso === todayIso}
          <div
            class="flex min-h-24 min-w-0 flex-col gap-0.5 border-b border-r border-line-subtle p-1 {isToday
              ? 'bg-surface'
              : ''}"
            aria-current={isToday ? 'date' : undefined}
          >
            <Button
              variant="ghost"
              size="sm"
              class="self-start"
              aria-label={$t('calendar.showDay').replace('{date}', $fmt.date(day))}
              onclick={() => showDay(day)}
            >
              <span
                class="flex size-6 items-center justify-center rounded-full {isToday
                  ? 'bg-accent text-on-accent'
                  : 'text-fg'}"
                data-numeric>{day.getDate()}</span
              >
            </Button>
            {#each sessions.slice(0, 3) as s (s.session.id)}
              {@const status = statusOf(s)}
              {@const start = sessionStartMinutes(s)}
              <button
                type="button"
                class="flex min-w-0 items-center gap-1 rounded-control px-1 py-0.5 text-left text-caption text-fg transition-colors duration-150 ease-standard hover:bg-surface-hover"
                aria-label={eventLabel(s)}
                title={eventLabel(s)}
                onclick={() => openDetails(s)}
              >
                {@render statusIcon(status)}
                {#if start !== null}
                  <span class="shrink-0 text-fg-muted" data-numeric>{minutesToTime(start)}</span>
                {/if}
                <span class="truncate">{s.patient_name}</span>
              </button>
            {/each}
            {#if sessions.length > 3}
              <Button variant="ghost" size="sm" class="self-start" onclick={() => showDay(day)}>
                {$t('calendar.more').replace('{count}', String(sessions.length - 3))}
              </Button>
            {/if}
          </div>
        {/if}
      {/each}
    </div>
  </div>
{/snippet}

{#snippet agenda()}
  {#if agendaDays.length === 0}
    <EmptyState title={$t('calendar.noSessionsWeek')} description={$t('calendar.noSessionsHint')} />
  {:else}
    <div class="space-y-6">
      {#each agendaDays as day (day.header.iso)}
        <section aria-labelledby="agenda-{day.header.iso}">
          <h2
            id="agenda-{day.header.iso}"
            class="mb-2 flex items-center gap-2 text-heading text-fg"
            aria-current={day.header.isToday ? 'date' : undefined}
          >
            {$fmt.weekday(day.header.date)}, {longDate(day.header.date)}
            {#if day.header.isToday}
              <span
                class="rounded-control border border-line px-1.5 py-0.5 text-caption font-medium text-fg"
                >{$t('calendar.todayMarker')}</span
              >
            {/if}
          </h2>
          <ul class="divide-y divide-line-subtle rounded-card border border-line bg-surface-raised">
            {#each day.sessions as s (s.session.id)}
              {@const status = statusOf(s)}
              <li class="flex flex-wrap items-center gap-x-4 gap-y-2 px-4 py-3">
                <span class="w-24 shrink-0 text-label text-fg-muted" data-numeric>
                  {timeRange(s)}
                </span>
                <div class="min-w-0 flex-1 basis-40">
                  <a
                    href={sessionHref(s)}
                    class="block truncate text-body font-medium text-fg hover:underline"
                    >{s.patient_name}</a
                  >
                  <p class="truncate text-label text-fg-muted">
                    {$sessionTypeLabel(s.session.session_type)}{#if s.session.duration_minutes}
                      · {$fmt.duration(s.session.duration_minutes)}{/if}
                  </p>
                </div>
                <StatusIndicator tone={STATUS[status].tone} label={$t(STATUS[status].key)} />
                <div class="flex shrink-0 items-center gap-2">
                  {#if status !== 'completed'}
                    <Button
                      size="sm"
                      onclick={() => openCompleteModal(s)}
                      aria-label={$t('calendar.markCompletedFor').replace('{name}', s.patient_name)}
                    >
                      {$t('calendar.markCompletedAction')}
                    </Button>
                  {/if}
                  <Button size="sm" variant="ghost" href={sessionHref(s)}>
                    {$t('calendar.openSession')}
                  </Button>
                </div>
              </li>
            {/each}
          </ul>
        </section>
      {/each}
    </div>
  {/if}
{/snippet}

<PageShell title={$t('calendar.title')} width="full">
  {#snippet actions()}
    <Button variant="primary" onclick={() => openNewSession(anchor, null)}>
      <Plus size={14} aria-hidden="true" />
      {$t('calendar.newSession')}
    </Button>
  {/snippet}

  <div bind:clientWidth={contentWidth}>
    <div class="mb-4 flex flex-wrap items-center justify-between gap-3">
      <div class="flex min-w-0 flex-wrap items-center gap-2">
        <div class="flex items-center gap-1">
          <IconButton label={navLabels[0]} onclick={() => step(-1)}>
            <ChevronLeft size={16} />
          </IconButton>
          <Button onclick={goToToday}>{$t('calendar.today')}</Button>
          <IconButton label={navLabels[1]} onclick={() => step(1)}>
            <ChevronRight size={16} />
          </IconButton>
        </div>
        <h2 class="min-w-0 text-heading text-fg" aria-live="polite">{label}</h2>
      </div>
      <SegmentedControl
        options={viewOptions}
        value={view}
        onchange={(v) => (view = v as CalendarView)}
        label={$t('calendar.viewLabel')}
      />
    </div>

    {#if weekendHidden > 0}
      <div class="mb-3 flex flex-wrap items-center gap-2 text-label text-fg-muted">
        <span>{$t('calendar.weekendHidden').replace('{count}', String(weekendHidden))}</span>
        <Button size="sm" variant="ghost" onclick={() => (view = 'agenda')}>
          {$t('calendar.showAgenda')}
        </Button>
      </div>
    {/if}

    {#if view === 'month'}
      {@render monthGrid()}
      {@render legend()}
    {:else if view === 'agenda'}
      {@render agenda()}
    {:else}
      {@render timeGrid()}
      {@render legend()}
    {/if}
  </div>
</PageShell>

<!-- New session: pick the patient (and adjust date/time) for a slot -->
<Dialog bind:open={newOpen} title={$t('sessions.newSession')} size="sm">
  <div class="space-y-3">
    <div class="grid grid-cols-2 gap-3">
      <Field label={$t('calendar.date')} for="modal-date">
        <Input id="modal-date" type="date" bind:value={modalDate} />
      </Field>
      <Field label={$t('calendar.time')} for="modal-time">
        <Select id="modal-time" bind:value={modalTime}>
          <option value="">{$t('sessions.noTime')}</option>
          {#each TIME_SLOTS as slot (slot)}
            <option value={slot}>{slot}</option>
          {/each}
          {#if modalTime && !TIME_SLOTS.includes(modalTime)}
            <option value={modalTime}>{modalTime}</option>
          {/if}
        </Select>
      </Field>
    </div>
    <Field label={$t('common.patient')} for="modal-patient">
      <Select id="modal-patient" bind:value={modalPatientId}>
        <option value="">{$t('common.selectPatient')}</option>
        {#each allPatients as patient (patient.id)}
          <option value={patient.id}>{patient.last_name}, {patient.first_name}</option>
        {/each}
      </Select>
    </Field>
  </div>
  {#snippet footer()}
    <Button onclick={() => (newOpen = false)}>{$t('common.cancel')}</Button>
    <Button variant="primary" onclick={confirmNewSession} disabled={!modalPatientId || !modalDate}>
      {$t('common.continue')}
    </Button>
  {/snippet}
</Dialog>

<!-- Session details -->
<Dialog bind:open={detailOpen} title={detailSession?.patient_name} size="sm">
  {#if detailSession}
    {@const s = detailSession}
    {@const status = statusOf(s)}
    <dl class="grid grid-cols-[auto_minmax(0,1fr)] gap-x-4 gap-y-2 text-body">
      <dt class="text-fg-muted">{$t('calendar.date')}</dt>
      <dd class="text-fg">
        {$fmt.weekday(s.session.session_date)}, {$fmt.date(s.session.session_date)}
      </dd>
      <dt class="text-fg-muted">{$t('calendar.time')}</dt>
      <dd class="text-fg" data-numeric>{timeRange(s)}</dd>
      <dt class="text-fg-muted">{$t('calendar.type')}</dt>
      <dd class="text-fg">{$sessionTypeLabel(s.session.session_type)}</dd>
      {#if s.session.duration_minutes}
        <dt class="text-fg-muted">{$t('calendar.duration')}</dt>
        <dd class="text-fg">{$fmt.duration(s.session.duration_minutes)}</dd>
      {/if}
      <dt class="text-fg-muted">{$t('calendar.status')}</dt>
      <dd><StatusIndicator tone={STATUS[status].tone} label={$t(STATUS[status].key)} /></dd>
    </dl>
  {/if}
  {#snippet footer()}
    {#if detailSession && statusOf(detailSession) !== 'completed'}
      {@const s = detailSession}
      <Button onclick={() => openCompleteModal(s)}>{$t('calendar.markCompletedAction')}</Button>
    {/if}
    {#if detailSession}
      <Button variant="primary" href={sessionHref(detailSession)}>
        {$t('calendar.openSession')}
      </Button>
    {/if}
  {/snippet}
</Dialog>

<!-- Mark completed -->
<Dialog
  bind:open={completeOpen}
  title={$t('sessions.complete')}
  description={completeSession
    ? `${completeSession.patient_name} · ${$fmt.date(completeSession.session.session_date)}`
    : undefined}
  size="sm"
  onClose={() => (completeSession = null)}
>
  <Field label={$t('sessions.notesOptional')} for="complete-notes">
    <Textarea
      id="complete-notes"
      bind:value={completeNotes}
      rows={4}
      placeholder={$t('sessions.notesPlaceholder')}
    />
  </Field>
  {#snippet footer()}
    <Button
      onclick={() => {
        completeOpen = false;
        completeSession = null;
      }}
      disabled={isSavingCompletion}
    >
      {$t('common.cancel')}
    </Button>
    <Button variant="primary" onclick={confirmComplete} loading={isSavingCompletion}>
      {isSavingCompletion ? $t('common.saving') : $t('calendar.markCompletedAction')}
    </Button>
  {/snippet}
</Dialog>
