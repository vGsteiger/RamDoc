import type { EvidenceManifestEntry } from '$lib/api';

export type PatientHistoryAnswerPart =
  { kind: 'text'; text: string } | { kind: 'citation'; text: string; entry: EvidenceManifestEntry };

/** Split an answer into plain text and citations backed by its evidence manifest. */
export function linkPatientHistoryCitations(
  answer: string,
  entries: EvidenceManifestEntry[]
): PatientHistoryAnswerPart[] {
  const entriesByCitation = new Map(entries.map((entry) => [entry.citation.toUpperCase(), entry]));
  const parts: PatientHistoryAnswerPart[] = [];
  const citationPattern = /\[(E\d+)\]/gi;
  let textStart = 0;

  for (const match of answer.matchAll(citationPattern)) {
    const matchStart = match.index;
    if (matchStart === undefined) continue;

    const entry = entriesByCitation.get(match[1].toUpperCase());
    if (!entry) continue;

    if (matchStart > textStart) {
      parts.push({ kind: 'text', text: answer.slice(textStart, matchStart) });
    }
    parts.push({ kind: 'citation', text: match[0], entry });
    textStart = matchStart + match[0].length;
  }

  if (textStart < answer.length) {
    parts.push({ kind: 'text', text: answer.slice(textStart) });
  }

  return parts;
}

/**
 * Route to the most specific patient record view available for an evidence source.
 *
 * Returns a SvelteKit 3 pathname: percent-encoded and without a leading slash, for
 * `resolve()` from `$app/paths`, which adds the base path. (A leading slash would make
 * `resolve()` treat it as a route ID such as `/patients/[id]`.)
 */
export function patientHistoryCitationHref(entry: EvidenceManifestEntry): `patients/${string}` {
  const patientBase: `patients/${string}` = `patients/${encodeURIComponent(entry.patient_id)}`;

  switch (entry.record_kind) {
    case 'session':
      return `${patientBase}/sessions/${encodeURIComponent(entry.record_id)}`;
    case 'file':
      return `${patientBase}/files`;
    case 'diagnosis':
      return `${patientBase}/diagnoses`;
    case 'medication':
      return `${patientBase}/medications`;
    case 'treatment_plan':
    case 'treatment_goal':
    case 'treatment_intervention':
      return `${patientBase}/treatment-plans`;
    case 'patient':
    case 'outcome_score':
      return patientBase;
  }

  return patientBase;
}
