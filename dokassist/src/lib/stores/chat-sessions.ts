import { get, writable } from 'svelte/store';
import { listChatSessions, listPatients, type ChatSession } from '$lib/api';

/**
 * Recent chats for the app sidebar and the command palette: global and
 * patient-scoped sessions merged, newest first, plus the display name of each
 * attached patient.
 *
 * The chat page should call `refreshChatSessions()` after it creates, renames
 * or deletes a session, or after the first message gives a chat its title, so
 * the sidebar updates without waiting for the next navigation.
 */
export interface ChatSessionsState {
  sessions: ChatSession[];
  patientNames: Record<string, string>;
  loaded: boolean;
}

const initial: ChatSessionsState = { sessions: [], patientNames: {}, loaded: false };
const store = writable<ChatSessionsState>(initial);

export const chatSessions = { subscribe: store.subscribe };

/** Test helper — not used by views. */
export function resetChatSessions() {
  store.set({ ...initial });
}

export function mergeChatSessions(...lists: ChatSession[][]): ChatSession[] {
  const byId = new Map<string, ChatSession>();
  for (const session of lists.flat()) byId.set(session.id, session);
  return [...byId.values()].sort((a, b) => b.updated_at.localeCompare(a.updated_at));
}

let inflight: Promise<void> | null = null;

export function refreshChatSessions(): Promise<void> {
  // Coalesce bursts (navigation + an explicit refresh from the chat page).
  if (inflight) return inflight;
  inflight = (async () => {
    try {
      const [global, patient] = await Promise.all([
        listChatSessions('global'),
        listChatSessions('patient'),
      ]);
      const sessions = mergeChatSessions(global, patient);
      let patientNames = get(store).patientNames;
      const missing = sessions.some((s) => s.patient_id && !(s.patient_id in patientNames));
      if (missing) {
        try {
          const patients = await listPatients();
          patientNames = Object.fromEntries(
            patients.map((p) => [p.id, `${p.first_name} ${p.last_name}`.trim()])
          );
        } catch (error) {
          console.error('Failed to load patient names for chats:', error);
        }
      }
      store.set({ sessions, patientNames, loaded: true });
    } catch (error) {
      console.error('Failed to load chat sessions:', error);
    } finally {
      inflight = null;
    }
  })();
  return inflight;
}

/** Optimistic local update after a rename made elsewhere. */
export function upsertChatSession(session: ChatSession) {
  store.update((state) => ({
    ...state,
    sessions: mergeChatSessions(
      state.sessions.filter((s) => s.id !== session.id),
      [session]
    ),
  }));
}

export function removeChatSession(sessionId: string) {
  store.update((state) => ({
    ...state,
    sessions: state.sessions.filter((s) => s.id !== sessionId),
  }));
}
