/**
 * Pure helpers for the patient workspace (D4, "patient as project").
 *
 * The workspace is one identification header plus route tabs. These helpers
 * decide which tab a URL belongs to, where `/patients/{id}` lands, and how
 * the header summarises the record. They hold no state, so they are unit
 * tested directly (src/tests/unit/patient-workspace.test.ts).
 */
import type { ChatSession, Diagnosis, Medication } from '$lib/api';

export type PatientTabId =
  'chats' | 'overview' | 'sessions' | 'diagnoses' | 'medications' | 'treatment-plans' | 'documents';

/** Tab order and the translation key of each label. Chats first (D4). */
export const PATIENT_TABS: { id: PatientTabId; labelKey: string }[] = [
  { id: 'chats', labelKey: 'patients.tabChats' },
  { id: 'overview', labelKey: 'patients.overview' },
  { id: 'sessions', labelKey: 'patients.sessions' },
  { id: 'diagnoses', labelKey: 'patients.diagnoses' },
  { id: 'medications', labelKey: 'patients.medications' },
  { id: 'treatment-plans', labelKey: 'patients.tabTreatmentPlan' },
  { id: 'documents', labelKey: 'patients.tabDocuments' },
];

export function patientPath(patientId: string, tab?: PatientTabId | string): string {
  const base = `/patients/${encodeURIComponent(patientId)}`;
  return tab ? `${base}/${tab}` : base;
}

/** The URL of each tab. Documents is owned by the documents workstream. */
export function patientTabHref(patientId: string, tab: PatientTabId): string {
  return patientPath(patientId, tab);
}

/**
 * Old document URLs (files, reports, letters, email) stay valid and belong
 * to the Documents tab; `/chat` (start/open a patient chat) belongs to Chats;
 * `/edit` is part of the Overview.
 */
const SEGMENT_TO_TAB: Record<string, PatientTabId> = {
  chats: 'chats',
  chat: 'chats',
  overview: 'overview',
  edit: 'overview',
  sessions: 'sessions',
  diagnoses: 'diagnoses',
  medications: 'medications',
  'treatment-plans': 'treatment-plans',
  documents: 'documents',
  files: 'documents',
  reports: 'documents',
  letters: 'documents',
  email: 'documents',
};

/** The tab a pathname belongs to, or null for the bare landing URL. */
export function activePatientTab(pathname: string, patientId: string): PatientTabId | null {
  const base = patientPath(patientId);
  if (pathname !== base && !pathname.startsWith(base + '/')) return null;
  const segment = pathname.slice(base.length + 1).split('/')[0];
  if (!segment) return null;
  return SEGMENT_TO_TAB[segment] ?? null;
}

/**
 * Where `/patients/{id}` lands. A patient with chats opens on Chats, like a
 * project opens on its conversations; a patient without any opens on the
 * Overview, so a first visit shows the record rather than an empty list.
 */
export function landingTab(chatCount: number): PatientTabId {
  return chatCount > 0 ? 'chats' : 'overview';
}

/**
 * Routes that carry their own primary action (a Save button), where the
 * header's "New chat" steps down to secondary so only one ink button shows.
 */
export function routeOwnsPrimary(pathname: string, patientId: string): boolean {
  const rest = pathname.slice(patientPath(patientId).length);
  return (
    rest === '/edit' ||
    rest.startsWith('/sessions/') ||
    rest.startsWith('/reports/') ||
    rest.startsWith('/letters/') ||
    rest.startsWith('/email/')
  );
}

/** 756.1234.5678.97 → 756.••••.••••.97. Anything else keeps its last two digits. */
export function maskAhv(ahv: string | null | undefined): string {
  if (!ahv) return '';
  const digits = ahv.replace(/\D/g, '');
  if (digits.length === 13) return `${digits.slice(0, 3)}.••••.••••.${digits.slice(11)}`;
  if (digits.length <= 2) return '••';
  return `${'•'.repeat(Math.min(digits.length - 2, 11))}${digits.slice(-2)}`;
}

/** A medication without an end date, or ending today or later, is current. */
export function isMedicationActive(medication: Medication, now: Date = new Date()): boolean {
  if (!medication.end_date) return true;
  const end = new Date(medication.end_date);
  const today = new Date(now);
  end.setHours(0, 0, 0, 0);
  today.setHours(0, 0, 0, 0);
  return end >= today;
}

export function activeDiagnoses(diagnoses: Diagnosis[]): Diagnosis[] {
  return diagnoses.filter((d) => d.status === 'active');
}

export function activeMedications(medications: Medication[], now?: Date): Medication[] {
  return medications.filter((m) => isMedicationActive(m, now));
}

/**
 * This patient's chats, newest first. Patient-scoped sessions come from the
 * patient-scoped list call; global sessions are included when they carry
 * this patient's id (a chat started elsewhere with the patient attached).
 */
export function chatsForPatient(
  patientScoped: ChatSession[],
  global: ChatSession[],
  patientId: string
): ChatSession[] {
  const seen = new Set<string>();
  return [...patientScoped, ...global.filter((c) => c.patient_id === patientId)]
    .filter((c) => c.patient_id === patientId && !seen.has(c.id) && seen.add(c.id))
    .sort((a, b) => b.updated_at.localeCompare(a.updated_at));
}

/** Deep link that opens an existing chat in the chat workspace. */
/** A fresh chat with this patient attached (D4: starting a chat from a patient). */
export function newPatientChatHref(patientId: string): string {
  return `/chat?patientId=${encodeURIComponent(patientId)}`;
}

export function chatHref(sessionId: string): string {
  return `/chat?sessionId=${encodeURIComponent(sessionId)}`;
}

/** Badge tone for a questionnaire interpretation (stored in English, e.g. "Moderately Severe"). */
export function severityTone(
  interpretation: string | null | undefined
): 'neutral' | 'success' | 'warning' | 'danger' {
  const lower = (interpretation ?? '').toLowerCase();
  if (!lower) return 'neutral';
  if (lower.includes('minimal')) return 'success';
  if (lower.includes('moderately severe') || lower.includes('severe')) return 'danger';
  if (lower.includes('mild') || lower.includes('moderate')) return 'warning';
  return 'neutral';
}

export interface ScaleSummary<S extends { scale_type: string; administered_at: string }> {
  scale: string;
  /** All scores of this scale, oldest first (the trend chart's order). */
  scores: S[];
  latest: S;
  previous: S | null;
}

/**
 * Latest score per questionnaire with the one before it, for the overview's
 * trend summary. Scales are ordered by their most recent administration.
 */
export function summariseScores<
  S extends { scale_type: string; administered_at: string; score: number },
>(scores: S[]): ScaleSummary<S>[] {
  const byScale = new Map<string, S[]>();
  for (const score of scores) {
    const list = byScale.get(score.scale_type) ?? [];
    list.push(score);
    byScale.set(score.scale_type, list);
  }
  return [...byScale.entries()]
    .map(([scale, list]) => {
      const sorted = [...list].sort((a, b) => a.administered_at.localeCompare(b.administered_at));
      return {
        scale,
        scores: sorted,
        latest: sorted[sorted.length - 1],
        previous: sorted.length > 1 ? sorted[sorted.length - 2] : null,
      };
    })
    .sort((a, b) => b.latest.administered_at.localeCompare(a.latest.administered_at));
}
