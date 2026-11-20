/**
 * Shared, reactive state of one open patient workspace.
 *
 * `patients/[id]/+layout.svelte` creates it and puts it in context; the
 * header reads it, and tabs call `refresh*()` after a write so the header's
 * diagnosis and medication chips and the Chats count stay current.
 */
import { getContext, setContext } from 'svelte';
import {
  getPatient,
  listChatSessions,
  listDiagnosesForPatient,
  listMedicationsForPatient,
  type ChatSession,
  type Diagnosis,
  type Medication,
  type Patient,
} from '$lib/api';
import { chatsForPatient } from './workspace';

const KEY = Symbol('patient-workspace');

export class PatientWorkspace {
  readonly id: string;
  patient = $state<Patient | null>(null);
  diagnoses = $state<Diagnosis[]>([]);
  medications = $state<Medication[]>([]);
  chats = $state<ChatSession[]>([]);
  loading = $state(true);
  failed = $state(false);
  /** True once the chat list has been fetched (or failed), for the landing redirect. */
  chatsLoaded = $state(false);

  constructor(id: string) {
    this.id = id;
  }

  async load() {
    this.loading = true;
    this.failed = false;
    try {
      this.patient = await getPatient(this.id);
    } catch (error) {
      console.error('Failed to load patient:', error);
      this.failed = true;
    } finally {
      this.loading = false;
    }
    if (this.failed) return;
    await Promise.all([this.refreshRecords(), this.refreshChats()]);
  }

  setPatient(patient: Patient) {
    this.patient = patient;
  }

  /** Reloads the header's diagnosis and medication summary. */
  async refreshRecords() {
    const [diagnoses, medications] = await Promise.allSettled([
      listDiagnosesForPatient(this.id),
      listMedicationsForPatient(this.id),
    ]);
    if (diagnoses.status === 'fulfilled') this.diagnoses = diagnoses.value;
    else console.error('Failed to load diagnoses:', diagnoses.reason);
    if (medications.status === 'fulfilled') this.medications = medications.value;
    else console.error('Failed to load medications:', medications.reason);
  }

  async refreshChats() {
    try {
      const [scoped, global] = await Promise.all([
        listChatSessions('patient', this.id),
        listChatSessions('global').catch(() => [] as ChatSession[]),
      ]);
      this.chats = chatsForPatient(scoped, global, this.id);
    } catch (error) {
      console.error('Failed to load patient chats:', error);
    } finally {
      this.chatsLoaded = true;
    }
  }
}

type WorkspaceGetter = () => PatientWorkspace | null;

/** The layout passes a getter, because the workspace is replaced when the patient id changes. */
export function setPatientWorkspace(getter: WorkspaceGetter): void {
  setContext(KEY, getter);
}

/** The surrounding workspace, or null outside `patients/[id]` (e.g. in component tests). */
export function getPatientWorkspace(): PatientWorkspace | null {
  const getter = getContext<WorkspaceGetter | undefined>(KEY);
  return getter?.() ?? null;
}

/** A getter for use in event handlers and $derived, which always sees the current workspace. */
export function usePatientWorkspace(): WorkspaceGetter {
  return getContext<WorkspaceGetter | undefined>(KEY) ?? (() => null);
}
