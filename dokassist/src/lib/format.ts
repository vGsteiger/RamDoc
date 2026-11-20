/**
 * Locale-aware formatting for dates, times and durations.
 *
 * One date format app-wide: a numeric date is always `17.11.2026` (Swiss
 * convention, in both languages) and times are always 24-hour `14:05`. Only
 * month and weekday *names* follow the UI language. Views must use these
 * helpers (or the `fmt` store) instead of calling `toLocaleDateString` with a
 * hardcoded locale, so a date reads the same on every screen.
 *
 * Every function accepts a `Date`, an ISO string, a date-only `YYYY-MM-DD`
 * string (interpreted as a *local* calendar date, never shifted by the UTC
 * offset) or a millisecond timestamp. Empty or unparseable input returns the
 * `fallback` (default `''`), so callers never render "Invalid Date".
 *
 * In components prefer the store, which re-renders on language change:
 *
 *   import { fmt } from '$lib/format';
 *   {$fmt.date(patient.date_of_birth)}   // 12.03.1984
 *   {$fmt.relativeDate(session.session_date)}  // Today / Heute
 */
import { derived, get, type Readable } from 'svelte/store';
import { language, type LanguageCode } from '$lib/stores/language';

export type DateInput = Date | string | number | null | undefined;
export type DateStyle = 'short' | 'medium' | 'long';

export interface FormatOptions {
  /** Defaults to the current app language. */
  lang?: LanguageCode;
  /** Returned for null, empty or invalid input. Defaults to ''. */
  fallback?: string;
}

/** BCP-47 locale used for month and weekday names in each UI language. */
const NAME_LOCALE: Record<LanguageCode, string> = { de: 'de-CH', en: 'en-GB' };

export function localeFor(lang: LanguageCode): string {
  return NAME_LOCALE[lang] ?? NAME_LOCALE.en;
}

function currentLang(lang?: LanguageCode): LanguageCode {
  return lang ?? get(language);
}

const DATE_ONLY = /^(\d{4})-(\d{2})-(\d{2})$/;

/** Parses any supported input into a valid Date, or null. */
export function toDate(value: DateInput): Date | null {
  if (value === null || value === undefined || value === '') return null;
  if (value instanceof Date) return Number.isNaN(value.getTime()) ? null : value;
  if (typeof value === 'number') {
    const d = new Date(value);
    return Number.isNaN(d.getTime()) ? null : d;
  }
  const trimmed = value.trim();
  const dateOnly = DATE_ONLY.exec(trimmed);
  if (dateOnly) {
    // `new Date('1984-03-12')` is UTC midnight, which is the previous day
    // anywhere west of Greenwich. A calendar date has no time zone.
    const [, y, m, d] = dateOnly;
    const date = new Date(Number(y), Number(m) - 1, Number(d));
    return Number.isNaN(date.getTime()) ? null : date;
  }
  const date = new Date(trimmed);
  return Number.isNaN(date.getTime()) ? null : date;
}

const pad = (n: number) => String(n).padStart(2, '0');

/**
 * `short` → 17.11.2026 (both languages)
 * `medium` → 17. Nov. 2026 / 17 Nov 2026
 * `long` → 17. November 2026 / 17 November 2026
 */
export function formatDate(
  value: DateInput,
  options: FormatOptions & { style?: DateStyle } = {}
): string {
  const date = toDate(value);
  if (!date) return options.fallback ?? '';
  const style = options.style ?? 'short';
  if (style === 'short') {
    return `${pad(date.getDate())}.${pad(date.getMonth() + 1)}.${date.getFullYear()}`;
  }
  return new Intl.DateTimeFormat(localeFor(currentLang(options.lang)), {
    day: 'numeric',
    month: style === 'medium' ? 'short' : 'long',
    year: 'numeric',
  }).format(date);
}

/** 24-hour clock in both languages: 14:05. */
export function formatTime(value: DateInput, options: FormatOptions = {}): string {
  const date = toDate(value);
  if (!date) return options.fallback ?? '';
  return `${pad(date.getHours())}:${pad(date.getMinutes())}`;
}

/** 17.11.2026, 14:05 */
export function formatDateTime(
  value: DateInput,
  options: FormatOptions & { style?: DateStyle } = {}
): string {
  const date = toDate(value);
  if (!date) return options.fallback ?? '';
  return `${formatDate(date, options)}, ${formatTime(date)}`;
}

/** Weekday name: `long` → Dienstag / Tuesday, `short` → Di. / Tue. */
export function formatWeekday(
  value: DateInput,
  options: FormatOptions & { style?: 'short' | 'long' } = {}
): string {
  const date = toDate(value);
  if (!date) return options.fallback ?? '';
  return new Intl.DateTimeFormat(localeFor(currentLang(options.lang)), {
    weekday: options.style ?? 'long',
  }).format(date);
}

function startOfDay(d: Date): number {
  return new Date(d.getFullYear(), d.getMonth(), d.getDate()).getTime();
}

const capitalise = (s: string) => s.charAt(0).toLocaleUpperCase() + s.slice(1);

/**
 * Calendar-relative date for lists and timestamps:
 * Today / Yesterday / Tomorrow (Heute / Gestern / Morgen), the weekday name
 * within the surrounding six days, otherwise the short date.
 * `now` is injectable for tests.
 */
export function formatRelativeDate(
  value: DateInput,
  options: FormatOptions & { now?: Date } = {}
): string {
  const date = toDate(value);
  if (!date) return options.fallback ?? '';
  const lang = currentLang(options.lang);
  const now = options.now ?? new Date();
  const days = Math.round((startOfDay(date) - startOfDay(now)) / 86_400_000);
  if (Math.abs(days) <= 1) {
    const rtf = new Intl.RelativeTimeFormat(localeFor(lang), { numeric: 'auto' });
    return capitalise(rtf.format(days, 'day'));
  }
  if (Math.abs(days) < 7) return formatWeekday(date, { lang });
  return formatDate(date, { lang });
}

const UNITS: Record<LanguageCode, { h: string; min: string }> = {
  de: { h: 'Std.', min: 'Min.' },
  en: { h: 'h', min: 'min' },
};

/**
 * Human duration from minutes: 50 Min. / 50 min, 1 Std. 30 Min. / 1 h 30 min.
 * Negative or non-finite input returns the fallback.
 */
export function formatDuration(minutes: number | null | undefined, options: FormatOptions = {}) {
  if (minutes === null || minutes === undefined || !Number.isFinite(minutes) || minutes < 0) {
    return options.fallback ?? '';
  }
  const unit = UNITS[currentLang(options.lang)] ?? UNITS.en;
  const total = Math.round(minutes);
  const h = Math.floor(total / 60);
  const m = total % 60;
  if (h === 0) return `${m} ${unit.min}`;
  if (m === 0) return `${h} ${unit.h}`;
  return `${h} ${unit.h} ${m} ${unit.min}`;
}

/** Elapsed clock from milliseconds for recordings and timers: 0:42, 12:05, 1:02:09. */
export function formatClock(ms: number | null | undefined, options: { fallback?: string } = {}) {
  if (ms === null || ms === undefined || !Number.isFinite(ms) || ms < 0) {
    return options.fallback ?? '';
  }
  const totalSeconds = Math.floor(ms / 1000);
  const h = Math.floor(totalSeconds / 3600);
  const m = Math.floor((totalSeconds % 3600) / 60);
  const s = totalSeconds % 60;
  return h > 0 ? `${h}:${pad(m)}:${pad(s)}` : `${m}:${pad(s)}`;
}

/** Whole years between a birth date and `now` (default today), or null. */
export function ageInYears(birthDate: DateInput, now: Date = new Date()): number | null {
  const dob = toDate(birthDate);
  if (!dob) return null;
  let age = now.getFullYear() - dob.getFullYear();
  const beforeBirthday =
    now.getMonth() < dob.getMonth() ||
    (now.getMonth() === dob.getMonth() && now.getDate() < dob.getDate());
  if (beforeBirthday) age -= 1;
  return age >= 0 ? age : null;
}

export interface Formatters {
  lang: LanguageCode;
  locale: string;
  date: (value: DateInput, options?: Omit<FormatOptions, 'lang'> & { style?: DateStyle }) => string;
  time: (value: DateInput, options?: Omit<FormatOptions, 'lang'>) => string;
  dateTime: (
    value: DateInput,
    options?: Omit<FormatOptions, 'lang'> & { style?: DateStyle }
  ) => string;
  weekday: (
    value: DateInput,
    options?: Omit<FormatOptions, 'lang'> & { style?: 'short' | 'long' }
  ) => string;
  relativeDate: (
    value: DateInput,
    options?: Omit<FormatOptions, 'lang'> & { now?: Date }
  ) => string;
  duration: (minutes: number | null | undefined, options?: Omit<FormatOptions, 'lang'>) => string;
  clock: (ms: number | null | undefined, options?: { fallback?: string }) => string;
}

/** Formatters bound to the current UI language; `$fmt.date(x)` in components. */
export const fmt: Readable<Formatters> = derived(language, (lang): Formatters => ({
  lang,
  locale: localeFor(lang),
  date: (value, options = {}) => formatDate(value, { ...options, lang }),
  time: (value, options = {}) => formatTime(value, { ...options, lang }),
  dateTime: (value, options = {}) => formatDateTime(value, { ...options, lang }),
  weekday: (value, options = {}) => formatWeekday(value, { ...options, lang }),
  relativeDate: (value, options = {}) => formatRelativeDate(value, { ...options, lang }),
  duration: (minutes, options = {}) => formatDuration(minutes, { ...options, lang }),
  clock: (ms, options = {}) => formatClock(ms, options),
}));
