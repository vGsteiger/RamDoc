import { describe, expect, it } from 'vitest';
import type { EvidenceManifestEntry, EvidenceRecordKind } from '$lib/api';
import {
  linkPatientHistoryCitations,
  patientHistoryCitationHref,
} from '$lib/patient-history-citations';

function entry(
  citation: string,
  recordKind: EvidenceRecordKind = 'session',
  recordId = 'session/1'
): EvidenceManifestEntry {
  return {
    citation,
    unit_id: `unit-${citation}`,
    patient_id: 'patient 1',
    record_kind: recordKind,
    record_id: recordId,
    section: 'notes',
    revision: 'r1',
    tier: 'hot',
    label: 'Session 1',
    occurred_at: '2026-08-13',
    char_start: 0,
    char_end: 10,
    text_sha256: 'abc',
    tokens: 4,
    prompt_token_start: 0,
    prompt_token_end: 4,
    protected_spans: [],
    selection: {
      lexical_rank: 1,
      lexical_bm25: 0.5,
      semantic_rank: null,
      semantic_similarity: null,
      fused_score: 1,
      recency_boost: 1,
      matched_terms: [],
      document_neighbor_of: [],
      temporal_neighbor_of: [],
      structured_truth: false,
    },
    selection_reasons: [],
  };
}

describe('linkPatientHistoryCitations', () => {
  it('links only citations backed by the manifest and preserves answer text', () => {
    const answer = 'Dose changed [E1], but [E99] is unsupported. [e2] confirms it.';
    const parts = linkPatientHistoryCitations(answer, [entry('E1'), entry('E2')]);

    expect(parts.map((part) => part.text).join('')).toBe(answer);
    expect(
      parts.filter((part) => part.kind === 'citation').map((part) => part.entry.citation)
    ).toEqual(['E1', 'E2']);
    expect(parts.some((part) => part.kind === 'text' && part.text.includes('[E99]'))).toBe(true);
  });

  it('returns one text part when no citation is traceable', () => {
    expect(linkPatientHistoryCitations('No citation [E7].', [])).toEqual([
      { kind: 'text', text: 'No citation [E7].' },
    ]);
  });
});

describe('patientHistoryCitationHref', () => {
  it('links session evidence directly to the session record', () => {
    expect(patientHistoryCitationHref(entry('E1'))).toBe(
      'patients/patient%201/sessions/session%2F1'
    );
  });

  it.each([
    ['file', '/files'],
    ['diagnosis', '/diagnoses'],
    ['medication', '/medications'],
    ['treatment_plan', '/treatment-plans'],
    ['treatment_goal', '/treatment-plans'],
    ['treatment_intervention', '/treatment-plans'],
    ['patient', ''],
    ['outcome_score', ''],
  ] satisfies [EvidenceRecordKind, string][])(
    'maps %s evidence to its patient view',
    (kind, suffix) => {
      expect(patientHistoryCitationHref(entry('E1', kind))).toBe(`patients/patient%201${suffix}`);
    }
  );

  it('falls back to the patient overview for an unknown runtime record kind', () => {
    const unknownEntry = {
      ...entry('E1'),
      record_kind: 'future_record_kind',
    } as unknown as EvidenceManifestEntry;

    expect(patientHistoryCitationHref(unknownEntry)).toBe('patients/patient%201');
  });
});
