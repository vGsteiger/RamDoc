/**
 * Merge and filter logic for the unified patient Documents view
 * (`/patients/{id}/documents`). Pure functions, so the view stays thin and the
 * ordering/filtering rules are unit-tested.
 *
 * The view reuses the four existing list APIs (files, reports, letters,
 * emails); nothing here talks to the backend.
 */
import type { Email, FileRecord, Letter, Report } from '$lib/api';

export type DocumentKind = 'file' | 'report' | 'letter' | 'email';

/** The `?type=` filter values, in SegmentedControl order. */
export const DOCUMENT_FILTERS = ['all', 'files', 'reports', 'letters', 'emails'] as const;
export type DocumentFilter = (typeof DOCUMENT_FILTERS)[number];

const KIND_FOR_FILTER: Record<Exclude<DocumentFilter, 'all'>, DocumentKind> = {
  files: 'file',
  reports: 'report',
  letters: 'letter',
  emails: 'email',
};

/**
 * Display status shared across document kinds. Files and reports have no
 * workflow state, so they carry `null`. Letters map `finalized` to `final`.
 */
export type DocumentStatus = 'draft' | 'final' | 'sent' | null;

export type DocumentItem =
  | { kind: 'file'; id: string; date: string; status: null; record: FileRecord }
  | { kind: 'report'; id: string; date: string; status: null; record: Report }
  | { kind: 'letter'; id: string; date: string; status: DocumentStatus; record: Letter }
  | { kind: 'email'; id: string; date: string; status: DocumentStatus; record: Email };

export interface DocumentSources {
  files?: FileRecord[];
  reports?: Report[];
  letters?: Letter[];
  emails?: Email[];
}

/** Reads `?type=`; anything unknown (or missing) falls back to `all`. */
export function parseDocumentFilter(value: string | null | undefined): DocumentFilter {
  return (DOCUMENT_FILTERS as readonly string[]).includes(value ?? '')
    ? (value as DocumentFilter)
    : 'all';
}

const SQLITE_TIMESTAMP = /^\d{4}-\d{2}-\d{2}[ T]\d{2}:\d{2}(:\d{2}(\.\d+)?)?$/;

/**
 * Backend timestamps come from SQLite's `datetime('now')`, which is UTC but
 * carries no zone (`2026-03-08 10:00:00`). JavaScript would read that as local
 * time, so a zone-less timestamp is treated as UTC. ISO strings that already
 * carry `Z` or an offset are left alone. Returns null for unparseable input.
 */
export function parseTimestamp(value: string | null | undefined): Date | null {
  if (!value) return null;
  const trimmed = value.trim();
  const normalized = SQLITE_TIMESTAMP.test(trimmed) ? `${trimmed.replace(' ', 'T')}Z` : trimmed;
  const date = new Date(normalized);
  return Number.isNaN(date.getTime()) ? null : date;
}

function time(value: string): number {
  return parseTimestamp(value)?.getTime() ?? 0;
}

export function letterStatus(status: Letter['status']): DocumentStatus {
  if (status === 'finalized') return 'final';
  if (status === 'sent') return 'sent';
  return 'draft';
}

export function emailStatus(status: string): DocumentStatus {
  return status === 'sent' ? 'sent' : 'draft';
}

/**
 * The date a document is listed and sorted by: when it was sent if it was,
 * otherwise when it was last changed (letters, emails) or created.
 */
function emailDate(email: Email): string {
  return (email.status === 'sent' && email.sent_at) || email.updated_at || email.created_at;
}

function letterDate(letter: Letter): string {
  return letter.sent_at || letter.finalized_at || letter.updated_at || letter.created_at;
}

/** One list, newest first. Ties keep a stable order (kind, then id). */
export function mergeDocuments(sources: DocumentSources): DocumentItem[] {
  const items: DocumentItem[] = [
    ...(sources.files ?? []).map((record): DocumentItem => ({
      kind: 'file',
      id: record.id,
      date: record.created_at,
      status: null,
      record,
    })),
    ...(sources.reports ?? []).map((record): DocumentItem => ({
      kind: 'report',
      id: record.id,
      date: record.generated_at || record.created_at,
      status: null,
      record,
    })),
    ...(sources.letters ?? []).map((record): DocumentItem => ({
      kind: 'letter',
      id: record.id,
      date: letterDate(record),
      status: letterStatus(record.status),
      record,
    })),
    ...(sources.emails ?? []).map((record): DocumentItem => ({
      kind: 'email',
      id: record.id,
      date: emailDate(record),
      status: emailStatus(record.status),
      record,
    })),
  ];

  return items.sort(
    (a, b) =>
      time(b.date) - time(a.date) || a.kind.localeCompare(b.kind) || a.id.localeCompare(b.id)
  );
}

export function filterDocuments(items: DocumentItem[], filter: DocumentFilter): DocumentItem[] {
  if (filter === 'all') return items;
  const kind = KIND_FOR_FILTER[filter];
  return items.filter((item) => item.kind === kind);
}

/** Where a row opens. Files have no page; the view opens them in FileViewer. */
export function documentHref(patientId: string, item: DocumentItem): string | null {
  const base = `/patients/${patientId}`;
  switch (item.kind) {
    case 'report':
      return `${base}/reports/${item.id}`;
    case 'letter':
      return `${base}/letters/${item.id}`;
    case 'email':
      return `${base}/email/${item.id}`;
    default:
      return null;
  }
}

/** The documents view URL for a filter; `all` has no query string. */
export function documentsUrl(patientId: string, filter: DocumentFilter = 'all'): string {
  const base = `/patients/${patientId}/documents`;
  return filter === 'all' ? base : `${base}?type=${filter}`;
}
