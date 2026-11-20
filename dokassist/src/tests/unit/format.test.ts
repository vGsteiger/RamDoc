import { describe, it, expect, afterEach } from 'vitest';
import { get } from 'svelte/store';
import { language } from '$lib/stores/language';
import {
  ageInYears,
  fmt,
  formatClock,
  formatDate,
  formatDateTime,
  formatDuration,
  formatRelativeDate,
  formatTime,
  formatWeekday,
  toDate,
} from '$lib/format';

const at = (y: number, m: number, d: number, h = 0, min = 0) => new Date(y, m - 1, d, h, min);

afterEach(() => language.set('en'));

describe('toDate', () => {
  it('reads a date-only string as a local calendar date, not UTC midnight', () => {
    const d = toDate('1984-03-12');
    expect(d?.getFullYear()).toBe(1984);
    expect(d?.getMonth()).toBe(2);
    expect(d?.getDate()).toBe(12);
  });

  it('returns null for empty and invalid input', () => {
    expect(toDate(null)).toBeNull();
    expect(toDate(undefined)).toBeNull();
    expect(toDate('')).toBeNull();
    expect(toDate('not a date')).toBeNull();
    expect(toDate(new Date('nope'))).toBeNull();
  });

  it('accepts timestamps and Date objects', () => {
    const d = at(2026, 11, 17);
    expect(toDate(d)).toBe(d);
    expect(toDate(d.getTime())?.getTime()).toBe(d.getTime());
  });
});

describe('formatDate', () => {
  it('uses one numeric format in both languages: dd.mm.yyyy', () => {
    expect(formatDate('2026-11-17', { lang: 'de' })).toBe('17.11.2026');
    expect(formatDate('2026-11-17', { lang: 'en' })).toBe('17.11.2026');
    expect(formatDate('1984-03-02', { lang: 'en' })).toBe('02.03.1984');
  });

  it('localises month names for medium and long styles', () => {
    expect(formatDate('2026-11-17', { lang: 'de', style: 'long' })).toBe('17. November 2026');
    expect(formatDate('2026-11-17', { lang: 'en', style: 'long' })).toBe('17 November 2026');
    expect(formatDate('2026-03-05', { lang: 'de', style: 'medium' })).toMatch(/^5\. März 2026$/);
    expect(formatDate('2026-11-17', { lang: 'en', style: 'medium' })).toBe('17 Nov 2026');
  });

  it('returns the fallback instead of "Invalid Date"', () => {
    expect(formatDate('garbage')).toBe('');
    expect(formatDate(null, { fallback: '—' })).toBe('—');
  });

  it('follows the language store when no lang is given', () => {
    language.set('de');
    expect(formatDate('2026-11-17', { style: 'long' })).toBe('17. November 2026');
  });
});

describe('formatTime and formatDateTime', () => {
  it('uses a 24-hour clock in both languages', () => {
    expect(formatTime(at(2026, 11, 17, 14, 5))).toBe('14:05');
    expect(formatTime(at(2026, 11, 17, 9, 0), { lang: 'en' })).toBe('09:00');
  });

  it('joins date and time', () => {
    expect(formatDateTime(at(2026, 11, 17, 14, 5), { lang: 'de' })).toBe('17.11.2026, 14:05');
  });
});

describe('formatWeekday', () => {
  it('names the weekday in the UI language', () => {
    expect(formatWeekday('2026-11-17', { lang: 'de' })).toBe('Dienstag');
    expect(formatWeekday('2026-11-17', { lang: 'en' })).toBe('Tuesday');
  });
});

describe('formatRelativeDate', () => {
  const now = at(2026, 11, 17, 10);

  it('says today, yesterday and tomorrow in the UI language, capitalised', () => {
    expect(formatRelativeDate(at(2026, 11, 17, 23, 59), { lang: 'de', now })).toBe('Heute');
    expect(formatRelativeDate('2026-11-16', { lang: 'de', now })).toBe('Gestern');
    expect(formatRelativeDate('2026-11-18', { lang: 'de', now })).toBe('Morgen');
    expect(formatRelativeDate('2026-11-17', { lang: 'en', now })).toBe('Today');
    expect(formatRelativeDate('2026-11-16', { lang: 'en', now })).toBe('Yesterday');
  });

  it('uses the weekday within the surrounding week', () => {
    expect(formatRelativeDate('2026-11-13', { lang: 'en', now })).toBe('Friday');
  });

  it('falls back to the short date further away', () => {
    expect(formatRelativeDate('2026-10-01', { lang: 'en', now })).toBe('01.10.2026');
  });
});

describe('formatDuration', () => {
  it('formats minutes with localised units', () => {
    expect(formatDuration(50, { lang: 'de' })).toBe('50 Min.');
    expect(formatDuration(50, { lang: 'en' })).toBe('50 min');
    expect(formatDuration(90, { lang: 'de' })).toBe('1 Std. 30 Min.');
    expect(formatDuration(120, { lang: 'en' })).toBe('2 h');
    expect(formatDuration(0, { lang: 'en' })).toBe('0 min');
  });

  it('rejects negative and missing durations', () => {
    expect(formatDuration(-5)).toBe('');
    expect(formatDuration(null, { fallback: '—' })).toBe('—');
    expect(formatDuration(Number.NaN)).toBe('');
  });
});

describe('formatClock', () => {
  it('formats elapsed milliseconds', () => {
    expect(formatClock(42_000)).toBe('0:42');
    expect(formatClock(725_000)).toBe('12:05');
    expect(formatClock(3_729_000)).toBe('1:02:09');
  });
});

describe('ageInYears', () => {
  it('counts whole years, before and after the birthday', () => {
    const now = at(2026, 11, 17);
    expect(ageInYears('1984-03-12', now)).toBe(42);
    expect(ageInYears('1984-11-18', now)).toBe(41);
    expect(ageInYears('1984-11-17', now)).toBe(42);
    expect(ageInYears(null, now)).toBeNull();
  });
});

describe('fmt store', () => {
  it('re-binds formatters when the language changes', () => {
    language.set('en');
    expect(get(fmt).duration(50)).toBe('50 min');
    language.set('de');
    expect(get(fmt).duration(50)).toBe('50 Min.');
    expect(get(fmt).locale).toBe('de-CH');
  });
});
