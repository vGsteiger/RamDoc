import type { Patient } from '$lib/api';

export const CONTEXT_SOURCE_KINDS = [
  'overview',
  'diagnoses',
  'medications',
  'sessions',
  'files',
] as const;

export type ContextSourceKind = (typeof CONTEXT_SOURCE_KINDS)[number];

export interface ContextSourcePlan {
  kind: ContextSourceKind;
  included: boolean;
  pinned: boolean;
  /** Number of records found by the latest planning read. */
  retrieved: number | null;
  /** Number of records that fit in the current context budget. */
  summarized: number | null;
}

export interface ContextPlanningMetadata {
  included: ContextSourceKind[];
  retrieved: number;
  summarized: number;
  isRefreshing: boolean;
  refreshedAt: string | null;
}

/**
 * A serialisable hand-off between the context picker and a chat/report surface.
 * `comparisonMode` is deliberate: normal mode has exactly one selected patient.
 */
export interface ContextPlan {
  comparisonMode: boolean;
  selectedPatients: Patient[];
  sources: ContextSourcePlan[];
  planning: ContextPlanningMetadata;
}

export interface ContextPickerLabels {
  title: string;
  patientSearch: string;
  patientSearchPlaceholder: string;
  comparisonMode: string;
  comparisonModeHint: string;
  selectedPatients: string;
  clearPatients: string;
  sources: string;
  pin: string;
  unpin: string;
  included: string;
  retrieved: string;
  summarized: string;
  loading: string;
  noPatients: string;
}

export const DEFAULT_CONTEXT_PICKER_LABELS: ContextPickerLabels = {
  title: 'Context',
  patientSearch: 'Find patient',
  patientSearchPlaceholder: 'Search patients',
  comparisonMode: 'Compare patients',
  comparisonModeHint: 'Comparison mode keeps multiple patient contexts explicit.',
  selectedPatients: 'Selected patients',
  clearPatients: 'Clear selected patients',
  sources: 'Include in context',
  pin: 'Pin source',
  unpin: 'Unpin source',
  included: 'Included',
  retrieved: 'Retrieved',
  summarized: 'Summarized',
  loading: 'Refreshing context plan',
  noPatients: 'No matching patients',
};
