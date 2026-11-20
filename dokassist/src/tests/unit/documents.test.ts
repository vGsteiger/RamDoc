import { describe, it, expect } from 'vitest';
import type { Email, FileRecord, Letter, Report } from '$lib/api';
import {
  documentHref,
  documentsUrl,
  filterDocuments,
  mergeDocuments,
  parseDocumentFilter,
  parseTimestamp,
} from '$lib/components/documents/documents';

const file = (id: string, created_at: string): FileRecord => ({
  id,
  patient_id: 'p1',
  filename: `${id}.pdf`,
  vault_path: 'x',
  mime_type: 'application/pdf',
  size_bytes: 1,
  created_at,
});

const report = (id: string, generated_at: string): Report => ({
  id,
  patient_id: 'p1',
  report_type: 'Verlaufsbericht',
  content: '',
  generated_at,
  model_name: null,
  prompt_hash: null,
  session_ids: null,
  created_at: generated_at,
});

const letter = (id: string, status: Letter['status'], updated_at: string, extra = {}): Letter => ({
  id,
  patient_id: 'p1',
  letter_type: 'referral',
  template_language: 'de',
  recipient_name: null,
  recipient_address: null,
  subject: id,
  content: '',
  status,
  model_name: null,
  session_ids: null,
  created_at: '2026-01-01 08:00:00',
  updated_at,
  finalized_at: null,
  sent_at: null,
  ...extra,
});

const email = (id: string, status: string, updated_at: string, sent_at: string | null): Email => ({
  id,
  patient_id: 'p1',
  recipient_email: 'a@example.ch',
  subject: id,
  body: '',
  status,
  sent_at,
  created_at: '2026-01-01 08:00:00',
  updated_at,
});

describe('parseDocumentFilter', () => {
  it('accepts the known filters', () => {
    for (const f of ['all', 'files', 'reports', 'letters', 'emails']) {
      expect(parseDocumentFilter(f)).toBe(f);
    }
  });

  it('falls back to all for missing or unknown values', () => {
    expect(parseDocumentFilter(null)).toBe('all');
    expect(parseDocumentFilter(undefined)).toBe('all');
    expect(parseDocumentFilter('')).toBe('all');
    expect(parseDocumentFilter('file')).toBe('all');
    expect(parseDocumentFilter('constructor')).toBe('all');
  });
});

describe('parseTimestamp', () => {
  it('reads zone-less SQLite timestamps as UTC', () => {
    expect(parseTimestamp('2026-03-08 10:00:00')?.toISOString()).toBe('2026-03-08T10:00:00.000Z');
    expect(parseTimestamp('2026-03-08 10:00:00.250')?.toISOString()).toBe(
      '2026-03-08T10:00:00.250Z'
    );
  });

  it('leaves ISO strings with a zone alone', () => {
    expect(parseTimestamp('2026-03-08T10:00:00+01:00')?.toISOString()).toBe(
      '2026-03-08T09:00:00.000Z'
    );
    expect(parseTimestamp('2026-03-08T10:00:00Z')?.toISOString()).toBe('2026-03-08T10:00:00.000Z');
  });

  it('returns null for empty or invalid input', () => {
    expect(parseTimestamp(null)).toBeNull();
    expect(parseTimestamp('')).toBeNull();
    expect(parseTimestamp('not a date')).toBeNull();
  });
});

describe('mergeDocuments', () => {
  const sources = {
    files: [file('f1', '2026-01-03 09:00:00')],
    reports: [report('r1', '2026-01-05 09:00:00')],
    letters: [letter('l1', 'finalized', '2026-01-04 09:00:00')],
    emails: [
      email('e1', 'sent', '2026-01-01 09:00:00', '2026-01-06 09:00:00'),
      email('e2', 'draft', '2026-01-02 09:00:00', null),
    ],
  };

  it('merges all four kinds newest first', () => {
    const merged = mergeDocuments(sources);
    expect(merged.map((d) => d.id)).toEqual(['e1', 'r1', 'l1', 'f1', 'e2']);
  });

  it('sorts mixed timestamp formats on the same clock', () => {
    const merged = mergeDocuments({
      files: [file('f-old', '2026-01-03 09:00:00')],
      reports: [report('r-new', '2026-01-03T09:30:00Z')],
    });
    expect(merged.map((d) => d.id)).toEqual(['r-new', 'f-old']);
  });

  it('maps statuses to draft / final / sent', () => {
    const merged = mergeDocuments({
      files: [file('f1', '2026-01-01 00:00:00')],
      reports: [report('r1', '2026-01-01 00:00:00')],
      letters: [
        letter('l-draft', 'draft', '2026-01-01 00:00:00'),
        letter('l-final', 'finalized', '2026-01-01 00:00:00'),
        letter('l-sent', 'sent', '2026-01-01 00:00:00'),
      ],
      emails: [
        email('e-draft', 'draft', '2026-01-01 00:00:00', null),
        email('e-sent', 'sent', '2026-01-01 00:00:00', '2026-01-01 00:00:00'),
      ],
    });
    const status = Object.fromEntries(merged.map((d) => [d.id, d.status]));
    expect(status).toEqual({
      f1: null,
      r1: null,
      'l-draft': 'draft',
      'l-final': 'final',
      'l-sent': 'sent',
      'e-draft': 'draft',
      'e-sent': 'sent',
    });
  });

  it('dates a sent letter by its send date', () => {
    const [item] = mergeDocuments({
      letters: [letter('l1', 'sent', '2026-01-01 00:00:00', { sent_at: '2026-02-01 00:00:00' })],
    });
    expect(item.date).toBe('2026-02-01 00:00:00');
  });

  it('keeps a stable order for equal dates', () => {
    const at = '2026-01-01 00:00:00';
    const merged = mergeDocuments({
      reports: [report('r1', at)],
      files: [file('f2', at), file('f1', at)],
    });
    expect(merged.map((d) => d.id)).toEqual(['f1', 'f2', 'r1']);
  });

  it('tolerates missing sources', () => {
    expect(mergeDocuments({})).toEqual([]);
  });
});

describe('filterDocuments', () => {
  const merged = mergeDocuments({
    files: [file('f1', '2026-01-01 00:00:00'), file('f2', '2026-01-02 00:00:00')],
    reports: [report('r1', '2026-01-03 00:00:00')],
    emails: [email('e1', 'draft', '2026-01-04 00:00:00', null)],
  });

  it('returns everything for all', () => {
    expect(filterDocuments(merged, 'all')).toHaveLength(4);
  });

  it('keeps only the matching kind, in order', () => {
    expect(filterDocuments(merged, 'files').map((d) => d.id)).toEqual(['f2', 'f1']);
    expect(filterDocuments(merged, 'reports').map((d) => d.id)).toEqual(['r1']);
    expect(filterDocuments(merged, 'letters')).toEqual([]);
    expect(filterDocuments(merged, 'emails').map((d) => d.id)).toEqual(['e1']);
  });
});

describe('documentHref and documentsUrl', () => {
  const [r, l, e, f] = mergeDocuments({
    reports: [report('r1', '2026-01-04 00:00:00')],
    letters: [letter('l1', 'draft', '2026-01-03 00:00:00')],
    emails: [email('e1', 'draft', '2026-01-02 00:00:00', null)],
    files: [file('f1', '2026-01-01 00:00:00')],
  });

  it('links reports, letters and emails to their detail pages', () => {
    expect(documentHref('p1', r)).toBe('/patients/p1/reports/r1');
    expect(documentHref('p1', l)).toBe('/patients/p1/letters/l1');
    expect(documentHref('p1', e)).toBe('/patients/p1/email/e1');
  });

  it('has no page for files', () => {
    expect(documentHref('p1', f)).toBeNull();
  });

  it('builds the documents URL with an optional type', () => {
    expect(documentsUrl('p1')).toBe('/patients/p1/documents');
    expect(documentsUrl('p1', 'all')).toBe('/patients/p1/documents');
    expect(documentsUrl('p1', 'emails')).toBe('/patients/p1/documents?type=emails');
  });
});
