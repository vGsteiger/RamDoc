import type { AnswerAudit, EvidenceManifest } from '$lib/api';

/**
 * Mask a Swiss AHV number for display beside a chat, keeping the country
 * prefix and the check digits: `756.1234.5678.97` → `756.••••.••••.97`.
 * Anything that is not 13 digits is masked except its last two characters.
 */
export function maskAhv(ahv: string): string {
  const digits = ahv.replace(/\D/g, '');
  if (digits.length === 13) {
    return `${digits.slice(0, 3)}.••••.••••.${digits.slice(11)}`;
  }
  const trimmed = ahv.trim();
  if (trimmed.length <= 2) return '••';
  return '•'.repeat(trimmed.length - 2) + trimmed.slice(-2);
}

/** Assistant (the agent) or "Records only · cited" (the audited history query). */
export type ChatMode = 'assistant' | 'records';

/**
 * One "Records only · cited" exchange in a chat. It is answered by the audited
 * `query_patient_history` pipeline and is not persisted as chat messages:
 * there is no API to append such a turn to a chat session.
 */
export interface RecordsTurn {
  id: string;
  patientId: string;
  question: string;
  createdAt: string;
  answer: string;
  manifest: EvidenceManifest | null;
  audit: AnswerAudit | null;
  error: string;
  pending: boolean;
  startedAt: number;
}

/** "E3" → "3": citations are shown as plain numbers in the chat. */
export function citationNumber(citation: string): string {
  return citation.replace(/^E/i, '');
}
