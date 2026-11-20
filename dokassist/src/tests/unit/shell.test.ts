import { describe, it, expect } from 'vitest';
import {
  chatSessionHref,
  isActivePath,
  matchShellShortcut,
  NEW_CHAT_URL,
  patientIdFromPath,
} from '$lib/shell';
import { mergeChatSessions } from '$lib/stores/chat-sessions';

describe('isActivePath (sidebar active state)', () => {
  it('matches the section root', () => {
    expect(isActivePath('/patients', '/patients')).toBe(true);
  });

  it('matches nested routes by prefix', () => {
    expect(isActivePath('/patients/p1', '/patients')).toBe(true);
    expect(isActivePath('/patients/p1/sessions/s1', '/patients')).toBe(true);
    expect(isActivePath('/settings/models', '/settings')).toBe(true);
  });

  it('ignores a trailing slash', () => {
    expect(isActivePath('/calendar/', '/calendar')).toBe(true);
  });

  it('does not match a sibling that only shares the prefix string', () => {
    expect(isActivePath('/patientsfoo', '/patients')).toBe(false);
    expect(isActivePath('/chatter', '/chat')).toBe(false);
  });

  it('does not match other sections', () => {
    expect(isActivePath('/calendar', '/patients')).toBe(false);
    expect(isActivePath('/', '/chat')).toBe(false);
  });

  it('treats aliases as the same section (Reference ← /literature)', () => {
    expect(isActivePath('/literature', '/reference', ['/literature'])).toBe(true);
    expect(isActivePath('/literature/l1', '/reference', ['/literature'])).toBe(true);
    expect(isActivePath('/reference/medications', '/reference', ['/literature'])).toBe(true);
  });
});

const key = (
  k: string,
  mods: Partial<Record<'metaKey' | 'ctrlKey' | 'shiftKey' | 'altKey', boolean>> = {}
) => ({
  key: k,
  metaKey: false,
  ctrlKey: false,
  shiftKey: false,
  altKey: false,
  ...mods,
});

describe('matchShellShortcut (D10)', () => {
  it('Cmd+K opens the command palette', () => {
    expect(matchShellShortcut(key('k', { metaKey: true }))).toBe('commandPalette');
  });

  it('Cmd+N starts a new chat', () => {
    expect(matchShellShortcut(key('n', { metaKey: true }))).toBe('newChat');
  });

  it('Cmd+Shift+N creates a new patient (key arrives upper-case with Shift)', () => {
    expect(matchShellShortcut(key('N', { metaKey: true, shiftKey: true }))).toBe('newPatient');
  });

  it('keeps Cmd+Shift+S for a new session', () => {
    expect(matchShellShortcut(key('S', { metaKey: true, shiftKey: true }))).toBe('newSession');
  });

  it('Cmd+\\ toggles the sidebar', () => {
    expect(matchShellShortcut(key('\\', { metaKey: true }))).toBe('toggleSidebar');
  });

  it('accepts Ctrl as the modifier for development in a browser', () => {
    expect(matchShellShortcut(key('k', { ctrlKey: true }))).toBe('commandPalette');
  });

  it('ignores unmodified keys, Alt chords and unrelated combos', () => {
    expect(matchShellShortcut(key('n'))).toBeNull();
    expect(matchShellShortcut(key('k', { shiftKey: true }))).toBeNull();
    expect(matchShellShortcut(key('n', { metaKey: true, altKey: true }))).toBeNull();
    expect(matchShellShortcut(key('s', { metaKey: true }))).toBeNull();
    expect(matchShellShortcut(key('Escape', { metaKey: true }))).toBeNull();
  });
});

describe('chat URLs', () => {
  it('new chat is /chat?new=1', () => {
    expect(NEW_CHAT_URL).toBe('/chat?new=1');
  });

  it('a global chat links by sessionId', () => {
    expect(chatSessionHref({ id: 'c1', patient_id: null })).toBe('/chat?sessionId=c1');
  });

  it('a patient chat also carries its patientId', () => {
    expect(chatSessionHref({ id: 'c2', patient_id: 'p1' })).toBe('/chat?sessionId=c2&patientId=p1');
  });
});

describe('patientIdFromPath', () => {
  it('extracts the patient id from patient pages', () => {
    expect(patientIdFromPath('/patients/p1')).toBe('p1');
    expect(patientIdFromPath('/patients/p1/sessions')).toBe('p1');
  });

  it('returns null for the list and the new-patient form', () => {
    expect(patientIdFromPath('/patients')).toBeNull();
    expect(patientIdFromPath('/patients/new')).toBeNull();
  });
});

describe('mergeChatSessions', () => {
  const s = (id: string, updated_at: string, patient_id: string | null = null) => ({
    id,
    scope: patient_id ? 'patient' : 'global',
    patient_id,
    title: id,
    created_at: updated_at,
    updated_at,
  });

  it('merges global and patient chats newest first, without duplicates', () => {
    const merged = mergeChatSessions(
      [s('a', '2026-11-10T10:00:00Z'), s('b', '2026-11-12T10:00:00Z')],
      [s('c', '2026-11-11T10:00:00Z', 'p1'), s('a', '2026-11-10T10:00:00Z')]
    );
    expect(merged.map((x) => x.id)).toEqual(['b', 'c', 'a']);
  });
});
