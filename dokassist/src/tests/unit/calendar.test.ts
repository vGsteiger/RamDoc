import { describe, it, expect } from 'vitest';
import type { SessionWithPatient } from '$lib/api';
import {
  addMonths,
  dayHeaders,
  groupByDate,
  layoutDay,
  monthCells,
  parseISODate,
  parseView,
  rangeLabel,
  sessionStartMinutes,
  sessionStatus,
  startOfWeek,
  toISODate,
  weekdayNames,
} from '$lib/calendar';

function session(
  id: string,
  date: string,
  time: string | null,
  extra: Partial<SessionWithPatient['session']> = {}
): SessionWithPatient {
  return {
    patient_name: `Patient ${id}`,
    session: {
      id,
      patient_id: 'p1',
      session_date: date,
      session_type: 'Psychotherapie',
      duration_minutes: 50,
      scheduled_time: time ? `${date}T${time}:00` : null,
      notes: null,
      amdp_data: null,
      clinical_summary: null,
      created_at: '',
      updated_at: '',
      ...extra,
    },
  };
}

describe('startOfWeek', () => {
  it('returns the Monday of the week, Sundays included', () => {
    // Tuesday 17 Nov 2026 → Monday 16 Nov
    expect(toISODate(startOfWeek(new Date(2026, 10, 17, 15, 30)))).toBe('2026-11-16');
    // Sunday 22 Nov 2026 belongs to the week starting 16 Nov
    expect(toISODate(startOfWeek(new Date(2026, 10, 22)))).toBe('2026-11-16');
    // Monday stays put, across a month boundary for the next one
    expect(toISODate(startOfWeek(new Date(2026, 10, 30)))).toBe('2026-11-30');
    expect(toISODate(startOfWeek(new Date(2026, 11, 2)))).toBe('2026-11-30');
  });
});

describe('dayHeaders', () => {
  const now = new Date(2026, 10, 18, 10, 0); // Wednesday

  it('lists seven days from Monday with localised weekday names and dates', () => {
    const en = dayHeaders(new Date(2026, 10, 16), 7, 'en', now);
    expect(en.map((d) => d.iso)).toEqual([
      '2026-11-16',
      '2026-11-17',
      '2026-11-18',
      '2026-11-19',
      '2026-11-20',
      '2026-11-21',
      '2026-11-22',
    ]);
    expect(en.map((d) => d.day)).toEqual([16, 17, 18, 19, 20, 21, 22]);
    expect(en[0].weekday).toMatch(/^Mon/);
    expect(en[6].weekday).toMatch(/^Sun/);

    const de = dayHeaders(new Date(2026, 10, 16), 7, 'de', now);
    expect(de[0].weekday).toMatch(/^Mo/);
    expect(de[2].weekday).toMatch(/^Mi/);
  });

  it('marks exactly one day as today, and only within the visible range', () => {
    const week = dayHeaders(new Date(2026, 10, 16), 7, 'en', now);
    expect(week.filter((d) => d.isToday).map((d) => d.iso)).toEqual(['2026-11-18']);
    const nextWeek = dayHeaders(new Date(2026, 10, 23), 7, 'en', now);
    expect(nextWeek.some((d) => d.isToday)).toBe(false);
  });

  it('flags weekends and supports a five-day work week', () => {
    const week = dayHeaders(new Date(2026, 10, 16), 7, 'en', now);
    expect(week.map((d) => d.isWeekend)).toEqual([false, false, false, false, false, true, true]);
    const work = dayHeaders(new Date(2026, 10, 16), 5, 'en', now);
    expect(work).toHaveLength(5);
    expect(work.at(-1)?.iso).toBe('2026-11-20');
  });
});

describe('weekdayNames and monthCells', () => {
  it('starts the month header on Monday in both languages', () => {
    expect(weekdayNames('en')[0]).toMatch(/^Mon/);
    expect(weekdayNames('de')[6]).toMatch(/^So/);
  });

  it('pads the month grid so the 1st falls on its weekday', () => {
    // 1 Nov 2026 is a Sunday → six leading blanks
    const cells = monthCells(new Date(2026, 10, 17));
    expect(cells.slice(0, 6)).toEqual(Array(6).fill(null));
    expect(toISODate(cells[6] as Date)).toBe('2026-11-01');
    expect(cells.filter(Boolean)).toHaveLength(30);
  });

  it('adds months without overflowing short months', () => {
    expect(toISODate(addMonths(new Date(2026, 0, 31), 1))).toBe('2026-02-01');
  });
});

describe('rangeLabel', () => {
  it('follows the UI language instead of hardcoding de-CH', () => {
    const anchor = new Date(2026, 10, 18);
    expect(rangeLabel('month', anchor, 'en')).toBe('November 2026');
    expect(rangeLabel('month', anchor, 'de')).toBe('November 2026');
    expect(rangeLabel('week', anchor, 'en')).toMatch(/16.*22 November 2026/);
    expect(rangeLabel('day', anchor, 'en')).toMatch(/^Wednesday/);
    expect(rangeLabel('day', anchor, 'de')).toMatch(/^Mittwoch/);
  });
});

describe('parse helpers', () => {
  it('accepts only known views and real dates', () => {
    expect(parseView('agenda')).toBe('agenda');
    expect(parseView('year')).toBeNull();
    expect(parseISODate('2026-11-17')?.getDate()).toBe(17);
    expect(parseISODate('2026-02-30')).toBeNull();
    expect(parseISODate('17.11.2026')).toBeNull();
  });
});

describe('sessions', () => {
  it('reads the start time and keeps the original status rule', () => {
    const now = new Date(2026, 10, 18, 12, 0);
    const past = session('a', '2026-11-18', '09:00');
    const future = session('b', '2026-11-18', '14:00');
    const done = session('c', '2026-11-18', '08:00', { notes: 'Notiz' });
    const blankNote = session('d', '2026-11-18', '08:00', { notes: '   ' });
    expect(sessionStartMinutes(past)).toBe(540);
    expect(sessionStartMinutes(session('e', '2026-11-18', null))).toBeNull();
    expect(sessionStatus(past, now)).toBe('note-pending');
    expect(sessionStatus(future, now)).toBe('scheduled');
    expect(sessionStatus(done, now)).toBe('completed');
    expect(sessionStatus(blankNote, now)).toBe('note-pending');
  });

  it('groups by date and sorts untimed sessions first', () => {
    const grouped = groupByDate([
      session('late', '2026-11-18', '15:00'),
      session('untimed', '2026-11-18', null),
      session('early', '2026-11-18', '08:00'),
      session('other', '2026-11-19', '09:00'),
    ]);
    expect(grouped.get('2026-11-18')?.map((s) => s.session.id)).toEqual([
      'untimed',
      'early',
      'late',
    ]);
    expect(grouped.get('2026-11-19')).toHaveLength(1);
  });

  it('sizes events by duration and splits overlapping ones into columns', () => {
    const placed = layoutDay([
      session('a', '2026-11-18', '09:00'),
      session('b', '2026-11-18', '09:30', { duration_minutes: 30 }),
      session('c', '2026-11-18', '11:00', { duration_minutes: null }),
      session('untimed', '2026-11-18', null),
    ]);
    const byId = Object.fromEntries(placed.map((p) => [p.item.session.id, p]));
    expect(Object.keys(byId)).toEqual(['a', 'b', 'c']);
    expect(byId.a).toMatchObject({ start: 540, end: 590, column: 0, columns: 2 });
    expect(byId.b).toMatchObject({ start: 570, end: 600, column: 1, columns: 2 });
    // No duration → the default 50 minutes; alone in its cluster.
    expect(byId.c).toMatchObject({ start: 660, end: 710, column: 0, columns: 1 });
  });
});
