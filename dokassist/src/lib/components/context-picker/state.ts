import {
  getPatient,
  listDiagnosesForPatient,
  listFiles,
  listMedicationsForPatient,
  listSessionsForPatient,
} from '$lib/api';
import {
  CONTEXT_SOURCE_KINDS,
  type ContextPlan,
  type ContextSourceKind,
  type ContextSourcePlan,
} from './types';
import type { Patient } from '$lib/api';

const DEFAULT_INCLUDED: ContextSourceKind[] = ['overview', 'diagnoses', 'medications', 'sessions'];

function source(kind: ContextSourceKind): ContextSourcePlan {
  return {
    kind,
    included: DEFAULT_INCLUDED.includes(kind),
    pinned: false,
    retrieved: null,
    summarized: null,
  };
}

export function createContextPlan(): ContextPlan {
  return {
    comparisonMode: false,
    selectedPatients: [],
    sources: CONTEXT_SOURCE_KINDS.map(source),
    planning: {
      included: [...DEFAULT_INCLUDED],
      retrieved: 0,
      summarized: 0,
      isRefreshing: false,
      refreshedAt: null,
    },
  };
}

function withPlanning(plan: ContextPlan, sources: ContextSourcePlan[]): ContextPlan {
  const included = sources.filter((item) => item.included).map((item) => item.kind);
  return {
    ...plan,
    sources,
    planning: {
      ...plan.planning,
      included,
      retrieved: sources.reduce(
        (count, item) => count + (item.included ? (item.retrieved ?? 0) : 0),
        0
      ),
      summarized: sources.reduce(
        (count, item) => count + (item.included ? (item.summarized ?? 0) : 0),
        0
      ),
    },
  };
}

export function setComparisonMode(plan: ContextPlan, comparisonMode: boolean): ContextPlan {
  return {
    ...plan,
    comparisonMode,
    // Leaving comparison mode is an explicit return to a one-patient context.
    selectedPatients: comparisonMode ? plan.selectedPatients : plan.selectedPatients.slice(0, 1),
  };
}

export function selectContextPatient(plan: ContextPlan, patient: Patient): ContextPlan {
  const alreadySelected = plan.selectedPatients.some((item) => item.id === patient.id);
  if (alreadySelected) return plan;
  return {
    ...plan,
    selectedPatients: plan.comparisonMode ? [...plan.selectedPatients, patient] : [patient],
  };
}

export function removeContextPatient(plan: ContextPlan, patientId: string): ContextPlan {
  return {
    ...plan,
    selectedPatients: plan.selectedPatients.filter((patient) => patient.id !== patientId),
  };
}

export function toggleContextSource(plan: ContextPlan, kind: ContextSourceKind): ContextPlan {
  return withPlanning(
    plan,
    plan.sources.map((item) =>
      item.kind === kind && !item.pinned ? { ...item, included: !item.included } : item
    )
  );
}

export function togglePinnedContextSource(plan: ContextPlan, kind: ContextSourceKind): ContextPlan {
  return withPlanning(
    plan,
    plan.sources.map((item) =>
      item.kind === kind
        ? { ...item, pinned: !item.pinned, included: !item.pinned || item.included }
        : item
    )
  );
}

async function countSource(patientId: string, kind: ContextSourceKind): Promise<number> {
  switch (kind) {
    case 'overview':
      await getPatient(patientId);
      return 1;
    case 'diagnoses':
      return (await listDiagnosesForPatient(patientId)).length;
    case 'medications':
      return (await listMedicationsForPatient(patientId)).length;
    case 'sessions':
      return (await listSessionsForPatient(patientId)).length;
    case 'files':
      return (await listFiles(patientId)).length;
  }
}

/** Read-only source planning. Consumers retain the records and choose their own budget policy. */
export async function refreshContextPlan(plan: ContextPlan): Promise<ContextPlan> {
  const patient = plan.selectedPatients[0];
  if (!patient)
    return withPlanning(
      { ...plan, planning: { ...plan.planning, isRefreshing: false } },
      plan.sources
    );

  const pending = plan.sources.filter((item) => item.included);
  const counts = await Promise.all(
    pending.map(async (item) => [item.kind, await countSource(patient.id, item.kind)] as const)
  );
  const countByKind = new Map(counts);
  const sources = plan.sources.map((item) => {
    const retrieved = item.included ? (countByKind.get(item.kind) ?? 0) : null;
    return { ...item, retrieved, summarized: retrieved };
  });
  const next = withPlanning(plan, sources);
  return {
    ...next,
    planning: { ...next.planning, isRefreshing: false, refreshedAt: new Date().toISOString() },
  };
}

/**
 * The current agent command has no durable context-plan argument. Keep the
 * selection explicit by serialising it into the request, rather than implying
 * that a UI selection has been persisted on the session.
 */
export function serializeContextPreamble(plan: ContextPlan): string | null {
  if (plan.selectedPatients.length === 0) return null;
  const patients = plan.selectedPatients
    .map((patient) => `${patient.first_name} ${patient.last_name} (${patient.id})`)
    .join(', ');
  const sources = plan.sources
    .filter((source) => source.included)
    .map((source) => `${source.kind}${source.pinned ? ' [must include]' : ''}`)
    .join(', ');

  return [
    '[Visible context plan for this request — not a durable session setting]',
    `Patients: ${patients}`,
    `Requested sources: ${sources || 'none'}`,
    `Planning metadata: included=${plan.planning.included.length}; retrieved=${plan.planning.retrieved}; summarized=${plan.planning.summarized}.`,
    'Use this plan only for this request. State clearly when a requested source is unavailable.',
  ].join('\n');
}
