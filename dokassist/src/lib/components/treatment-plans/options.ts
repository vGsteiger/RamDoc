/**
 * Stored slugs for treatment plans. These are persisted values (see
 * `planStatusLabel` and friends in $lib/translations/labels): only their
 * display labels are translated, never the values themselves.
 */
export const PLAN_STATUSES = ['active', 'completed', 'revised', 'discontinued'] as const;
export const GOAL_STATUSES = ['pending', 'in_progress', 'achieved', 'revised', 'discontinued'];
export const INTERVENTION_TYPES = ['psychotherapy', 'medication', 'referral', 'other'];

export type StatusTone = 'neutral' | 'info' | 'success' | 'warning';

/** Badge tone for a plan or goal status. Colour is never the only signal: the label is always shown. */
export function statusTone(status: string): StatusTone {
  switch (status) {
    case 'active':
    case 'in_progress':
      return 'success';
    case 'completed':
    case 'achieved':
      return 'info';
    case 'revised':
      return 'warning';
    default:
      return 'neutral';
  }
}

export function today(): string {
  return new Date().toISOString().split('T')[0];
}

export function errorMessage(prefix: string, err: unknown): string {
  return `${prefix}: ${err instanceof Error ? err.message : String(err)}`;
}
