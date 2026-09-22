import type { ChatMessageRow } from '$lib/api';

/**
 * Evidence labels are deliberately derived from the typed tool boundary, not
 * from bracketed text the model happened to write. A tool response can show
 * what was available to the model; it cannot prove a free-form conclusion.
 */
export type ProvenanceStatus = 'supported' | 'inferred' | 'unsupported';

export interface ProvenanceSource {
  toolName: string;
  status: 'supported';
}

export interface ChatProvenance {
  sources: ProvenanceSource[];
  status: ProvenanceStatus;
  /** Citation-looking text has no evidentiary force in universal chat. */
  unverifiedCitations: string[];
}

const PATIENT_READ_TOOLS = new Set([
  'get_patient',
  'get_calendar_events',
  'list_diagnoses',
  'list_medications',
  'list_treatment_plans',
  'search',
]);

function hasToolError(message: ChatMessageRow): boolean {
  try {
    const value: unknown = JSON.parse(message.content);
    return typeof value === 'object' && value !== null && 'error' in value;
  } catch {
    return true;
  }
}

export function evidenceFromToolResult(message: ChatMessageRow): ProvenanceSource | null {
  if (message.role !== 'tool_result' || !message.tool_name || hasToolError(message)) return null;
  return PATIENT_READ_TOOLS.has(message.tool_name)
    ? { toolName: message.tool_name, status: 'supported' }
    : null;
}

/** Tool outputs between a user turn and its answer are the only trusted inputs. */
export function evidenceForTurn(
  messages: ChatMessageRow[],
  answerIndex: number
): ProvenanceSource[] {
  const sources = new Map<string, ProvenanceSource>();
  for (let index = answerIndex - 1; index >= 0; index -= 1) {
    const message = messages[index];
    if (message.role === 'assistant' || message.role === 'user') break;
    const source = evidenceFromToolResult(message);
    if (source) sources.set(source.toolName, source);
  }
  return [...sources.values()];
}

/** Do not elevate citation-shaped assistant prose to a real citation. */
export function citationLikeTokens(content: string): string[] {
  return [...new Set(content.match(/\[[A-Za-z][A-Za-z0-9_-]{0,40}\]/g) ?? [])];
}

export function provenanceForAnswer(
  messages: ChatMessageRow[],
  answerIndex: number,
  patientGrounded: boolean
): ChatProvenance {
  const sources = evidenceForTurn(messages, answerIndex);
  const unverifiedCitations = citationLikeTokens(messages[answerIndex]?.content ?? '');
  if (!patientGrounded) {
    return { sources, unverifiedCitations, status: sources.length ? 'inferred' : 'unsupported' };
  }
  // A source validates that the input existed, while the LLM's prose remains
  // an inference unless a manifest-backed patient-history query is used.
  return { sources, unverifiedCitations, status: sources.length ? 'inferred' : 'unsupported' };
}

export type ClaimResolution = 'user_provided' | 'uncertain' | 'removed';

export interface UnsupportedClaim {
  id: string;
  kind: 'unverified_citation' | 'missing_typed_source';
  citation?: string;
}

/**
 * This is intentionally conservative. Generic chat has no claim-to-span audit
 * like the patient-history evidence endpoint, so it blocks saving when the
 * draft has no typed patient reads or invents citation-looking markers.
 */
export function unsupportedDraftClaims(
  content: string,
  evidence: ProvenanceSource[]
): UnsupportedClaim[] {
  const claims: UnsupportedClaim[] = citationLikeTokens(content).map((citation) => ({
    id: `citation:${citation}`,
    kind: 'unverified_citation' as const,
    citation,
  }));
  if (evidence.length === 0 && content.trim()) {
    claims.unshift({
      id: 'missing-typed-source',
      kind: 'missing_typed_source',
    });
  }
  return claims;
}

export function unresolvedClaims(
  claims: UnsupportedClaim[],
  resolutions: Record<string, ClaimResolution | ''>
): UnsupportedClaim[] {
  return claims.filter((claim) => !resolutions[claim.id]);
}
