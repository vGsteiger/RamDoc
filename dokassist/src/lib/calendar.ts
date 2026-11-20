/**
 * Pure date and layout logic for the calendar view (`/calendar`).
 *
 * Kept out of the component so the week header, the today marker, the status
 * rule and the event layout are unit-tested. All dates are local calendar
 * dates; weeks start on Monday (Swiss convention, both UI languages).
 */
import type { SessionWithPatient } from '$lib/api';
import { formatWeekday, localeFor } from '$lib/format';
import type { LanguageCode } from '$lib/stores/language';

export type CalendarView = 'day' | 'week' | 'month' | 'agenda';
export const CALENDAR_VIEWS: CalendarView[] = ['day', 'week', 'month', 'agenda'];

export type SessionStatus = 'scheduled' | 'completed' | 'note-pending';

/** First and last hour row of the time grid (07:00 … 21:00 inclusive). */
export const FIRST_HOUR = 7;
export const LAST_HOUR = 21;
/** Assumed length of a session without a duration, in minutes. */
export const DEFAULT_DURATION = 50;

export function parseView(value: string | null | undefined): CalendarView | null {
  return CALENDAR_VIEWS.includes(value as CalendarView) ? (value as CalendarView) : null;
}

export function startOfDay(date: Date): Date {
  return new Date(date.getFullYear(), date.getMonth(), date.getDate());
}

export function addDays(date: Date, days: number): Date {
  const d = new Date(date);
  d.setDate(d.getDate() + days);
  return d;
}

/** Monday of the week containing `date`, at local midnight. */
export function startOfWeek(date: Date): Date {
  const d = startOfDay(date);
  const day = d.getDay(); // 0 = Sunday
  return addDays(d, day === 0 ? -6 : 1 - day);
}

export function addMonths(date: Date, months: number): Date {
  // Clamp to the 1st so 31 January + 1 month does not overflow into March.
  return new Date(date.getFullYear(), date.getMonth() + months, 1);
}

/** Local `YYYY-MM-DD`, never shifted by the UTC offset. */
export function toISODate(date: Date): string {
  const y = date.getFullYear();
  const m = String(date.getMonth() + 1).padStart(2, '0');
  const d = String(date.getDate()).padStart(2, '0');
  return `${y}-${m}-${d}`;
}

/** Parses `YYYY-MM-DD` as a local date, or null. */
export function parseISODate(value: string | null | undefined): Date | null {
  const match = /^(\d{4})-(\d{2})-(\d{2})$/.exec(value ?? '');
  if (!match) return null;
  const date = new Date(Number(match[1]), Number(match[2]) - 1, Number(match[3]));
  return Number.isNaN(date.getTime()) || toISODate(date) !== value ? null : date;
}

export function isSameDay(a: Date, b: Date): boolean {
  return toISODate(a) === toISODate(b);
}

export interface DayHeader {
  date: Date;
  iso: string;
  /** Localised short weekday: "Mo." / "Mon". */
  weekday: string;
  /** Day of the month. */
  day: number;
  isToday: boolean;
  isWeekend: boolean;
}

/**
 * The header row of the week/day grid: one entry per visible day, starting
 * at `start`, with the weekday name in the UI language and a today flag.
 */
export function dayHeaders(
  start: Date,
  count: number,
  lang: LanguageCode,
  now: Date = new Date()
): DayHeader[] {
  return Array.from({ length: count }, (_, i) => {
    const date = addDays(startOfDay(start), i);
    const dow = date.getDay();
    return {
      date,
      iso: toISODate(date),
      weekday: formatWeekday(date, { lang, style: 'short' }),
      day: date.getDate(),
      isToday: isSameDay(date, now),
      isWeekend: dow === 0 || dow === 6,
    };
  });
}

/** Short weekday names Monday → Sunday in the UI language (month header). */
export function weekdayNames(lang: LanguageCode): string[] {
  // 2024-01-01 was a Monday.
  return Array.from({ length: 7 }, (_, i) =>
    formatWeekday(new Date(2024, 0, 1 + i), { lang, style: 'short' })
  );
}

/** Month grid cells (Monday-aligned), `null` for the leading padding. */
export function monthCells(anchor: Date): (Date | null)[] {
  const first = new Date(anchor.getFullYear(), anchor.getMonth(), 1);
  const daysInMonth = new Date(anchor.getFullYear(), anchor.getMonth() + 1, 0).getDate();
  const lead = (first.getDay() + 6) % 7;
  const cells: (Date | null)[] = Array(lead).fill(null);
  for (let d = 1; d <= daysInMonth; d++) {
    cells.push(new Date(anchor.getFullYear(), anchor.getMonth(), d));
  }
  return cells;
}

/**
 * Header label for the visible range, in the UI language:
 * day → "Dienstag, 17. November 2026"; week → "16.–22. November 2026";
 * month → "November 2026".
 */
export function rangeLabel(view: CalendarView, anchor: Date, lang: LanguageCode, days = 7): string {
  const locale = localeFor(lang);
  if (view === 'day') {
    return new Intl.DateTimeFormat(locale, {
      weekday: 'long',
      day: 'numeric',
      month: 'long',
      year: 'numeric',
    }).format(anchor);
  }
  if (view === 'month') {
    return new Intl.DateTimeFormat(locale, { month: 'long', year: 'numeric' }).format(anchor);
  }
  const start = startOfWeek(anchor);
  const end = addDays(start, days - 1);
  const f = new Intl.DateTimeFormat(locale, { day: 'numeric', month: 'long', year: 'numeric' });
  return f.formatRange(start, end);
}

/** Minutes after midnight of the session's scheduled start, or null if untimed. */
export function sessionStartMinutes(session: SessionWithPatient): number | null {
  const match = /T(\d{2}):(\d{2})/.exec(session.session.scheduled_time ?? '');
  if (!match) return null;
  return Number(match[1]) * 60 + Number(match[2]);
}

/** `HH:MM` for minutes after midnight. */
export function minutesToTime(minutes: number): string {
  const h = Math.floor(minutes / 60);
  const m = minutes % 60;
  return `${String(h).padStart(2, '0')}:${String(m).padStart(2, '0')}`;
}

/**
 * Completed once it has a note; notes pending once its scheduled time has
 * passed without one; otherwise scheduled. (Unchanged from the original
 * calendar rule.)
 */
export function sessionStatus(session: SessionWithPatient, now: Date = new Date()): SessionStatus {
  if (session.session.notes && session.session.notes.trim()) return 'completed';
  if (session.session.scheduled_time && new Date(session.session.scheduled_time) < now) {
    return 'note-pending';
  }
  return 'scheduled';
}

/** Groups sessions by their `YYYY-MM-DD` session date, sorted by start time. */
export function groupByDate(sessions: SessionWithPatient[]): Map<string, SessionWithPatient[]> {
  const byDate = new Map<string, SessionWithPatient[]>();
  for (const s of sessions) {
    const key = s.session.session_date.slice(0, 10);
    const list = byDate.get(key);
    if (list) list.push(s);
    else byDate.set(key, [s]);
  }
  for (const list of byDate.values()) {
    list.sort((a, b) => (sessionStartMinutes(a) ?? -1) - (sessionStartMinutes(b) ?? -1));
  }
  return byDate;
}

export interface PlacedEvent {
  item: SessionWithPatient;
  start: number;
  end: number;
  /** Overlap column and the number of columns in its overlap cluster. */
  column: number;
  columns: number;
}

/**
 * Places a day's timed sessions on the time grid: each event spans its real
 * duration, and overlapping events share the width side by side. Untimed
 * sessions are skipped (they render in the all-day row).
 */
export function layoutDay(sessions: SessionWithPatient[]): PlacedEvent[] {
  const timed = sessions
    .map((item) => {
      const start = sessionStartMinutes(item);
      if (start === null) return null;
      const duration = item.session.duration_minutes || DEFAULT_DURATION;
      return { item, start, end: start + Math.max(duration, 15), column: 0, columns: 1 };
    })
    .filter((e): e is PlacedEvent => e !== null)
    .sort((a, b) => a.start - b.start || b.end - a.end);

  let cluster: PlacedEvent[] = [];
  let clusterEnd = -1;
  const flush = () => {
    const columns = Math.max(1, ...cluster.map((e) => e.column + 1));
    for (const e of cluster) e.columns = columns;
    cluster = [];
  };

  for (const event of timed) {
    if (event.start >= clusterEnd && cluster.length) flush();
    const taken = new Set(cluster.filter((e) => e.end > event.start).map((e) => e.column));
    let column = 0;
    while (taken.has(column)) column++;
    event.column = column;
    cluster.push(event);
    clusterEnd = Math.max(clusterEnd, event.end);
  }
  if (cluster.length) flush();
  return timed;
}
