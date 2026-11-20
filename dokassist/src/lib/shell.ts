/**
 * Pure helpers for the app shell (sidebar, global shortcuts, chat URLs).
 * Kept out of the components so they can be unit-tested directly.
 */

/**
 * Prefix-matched active state for a navigation item: `/patients` is active on
 * `/patients`, `/patients/p1` and `/patients/p1/sessions`, but not on
 * `/patientsfoo`. Extra `aliases` count as the same section (`/literature`
 * highlights Reference).
 */
export function isActivePath(
  currentPath: string,
  itemPath: string,
  aliases: readonly string[] = []
): boolean {
  const normalised = currentPath.length > 1 ? currentPath.replace(/\/+$/, '') : currentPath;
  return [itemPath, ...aliases].some(
    (path) => normalised === path || normalised.startsWith(`${path}/`)
  );
}

/** Fresh, empty chat (D3: shows the Today panel). Bound to Cmd+N. */
export const NEW_CHAT_URL = '/chat?new=1';

/**
 * URL of one chat session. `patientId` is included for patient-scoped chats
 * so the chat page can attach the patient without another lookup.
 */
export function chatSessionHref(session: { id: string; patient_id?: string | null }): string {
  const params = new URLSearchParams({ sessionId: session.id });
  if (session.patient_id) params.set('patientId', session.patient_id);
  return `/chat?${params.toString()}`;
}

export type ShellShortcut =
  'commandPalette' | 'newChat' | 'newPatient' | 'newSession' | 'toggleSidebar';

/**
 * Maps a keydown to a global shell shortcut (D10). Cmd on macOS; Ctrl is
 * accepted too so the bindings also work in a browser during development.
 *
 *   Cmd+K        command palette (search + commands)
 *   Cmd+N        new chat
 *   Cmd+Shift+N  new patient
 *   Cmd+Shift+S  new session (on a patient page)
 *   Cmd+\        collapse / expand the sidebar
 */
export function matchShellShortcut(
  event: Pick<KeyboardEvent, 'key' | 'metaKey' | 'ctrlKey' | 'shiftKey' | 'altKey'>
): ShellShortcut | null {
  if (!(event.metaKey || event.ctrlKey) || event.altKey) return null;
  const key = event.key.toLowerCase();
  if (key === 'k' && !event.shiftKey) return 'commandPalette';
  if (key === 'n') return event.shiftKey ? 'newPatient' : 'newChat';
  if (key === 's' && event.shiftKey) return 'newSession';
  if (key === '\\' && !event.shiftKey) return 'toggleSidebar';
  return null;
}

/** Patient id when the path is a patient page (`/patients/p1/...`), else null. */
export function patientIdFromPath(path: string): string | null {
  const match = path.match(/^\/patients\/([^/]+)/);
  return match && match[1] !== 'new' ? match[1] : null;
}
