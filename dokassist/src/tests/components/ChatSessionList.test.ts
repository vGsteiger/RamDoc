import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/svelte';
import { invoke } from '@tauri-apps/api/core';
import { goto } from '$app/navigation';
import ChatSessionList from '../../lib/components/ChatSessionList.svelte';
import type { ChatSession } from '$lib/api';

vi.mock('$app/navigation', () => ({ goto: vi.fn() }));

const mockInvoke = vi.mocked(invoke);

const SESSION_1: ChatSession = {
  id: 's1',
  scope: 'global',
  patient_id: null,
  title: 'First Chat',
  created_at: '2026-01-01T00:00:00Z',
  updated_at: '2026-01-01T00:00:00Z',
};

const SESSION_2: ChatSession = {
  id: 's2',
  scope: 'patient',
  patient_id: 'p1',
  title: 'Second Chat',
  created_at: '2026-01-02T00:00:00Z',
  updated_at: '2026-01-02T00:00:00Z',
};

function many(n: number): ChatSession[] {
  return Array.from({ length: n }, (_, i) => ({
    ...SESSION_1,
    id: `m${i}`,
    title: `Chat ${i}`,
  }));
}

beforeEach(() => {
  vi.clearAllMocks();
  mockInvoke.mockReset();
});

function renderList(props: Record<string, unknown> = {}) {
  return render(ChatSessionList, {
    props: { sessions: [SESSION_1, SESSION_2], activeSessionId: null, ...props },
  });
}

describe('ChatSessionList (sidebar recent chats)', () => {
  it('renders each chat as a link to its session', () => {
    renderList();
    expect(screen.getByRole('link', { name: /First Chat/ })).toHaveAttribute(
      'href',
      '/chat?sessionId=s1'
    );
    expect(screen.getByRole('link', { name: /Second Chat/ })).toHaveAttribute(
      'href',
      '/chat?sessionId=s2&patientId=p1'
    );
  });

  it("shows the attached patient's name for patient chats", () => {
    renderList({ patientNames: { p1: 'Anna Müller' } });
    expect(screen.getByRole('link', { name: /Second Chat/ })).toHaveTextContent('Anna Müller');
  });

  it('marks the active chat with aria-current', () => {
    renderList({ activeSessionId: 's2' });
    expect(screen.getByRole('link', { name: /Second Chat/ })).toHaveAttribute(
      'aria-current',
      'page'
    );
    expect(screen.getByRole('link', { name: /First Chat/ })).not.toHaveAttribute('aria-current');
  });

  it('shows an empty-state line when there are no chats', () => {
    renderList({ sessions: [] });
    expect(screen.getByText('No chats yet')).toBeInTheDocument();
  });

  it('navigates and notifies onsessionselect when a chat is clicked', async () => {
    const onsessionselect = vi.fn();
    renderList({ onsessionselect });
    await fireEvent.click(screen.getByRole('link', { name: /First Chat/ }));
    expect(onsessionselect).toHaveBeenCalledWith('s1');
    expect(goto).toHaveBeenCalledWith('/chat?sessionId=s1');
  });

  it('shows the first rows and expands with "Show all"', async () => {
    renderList({ sessions: many(12), limit: 8 });
    expect(screen.getAllByRole('link')).toHaveLength(8);
    await fireEvent.click(screen.getByRole('button', { name: 'Show all (12)' }));
    expect(screen.getAllByRole('link')).toHaveLength(12);
    await fireEvent.click(screen.getByRole('button', { name: 'Show fewer' }));
    expect(screen.getAllByRole('link')).toHaveLength(8);
  });

  it('keeps the active chat visible even when it is beyond the limit', () => {
    renderList({ sessions: many(12), limit: 8, activeSessionId: 'm10' });
    expect(screen.getByRole('link', { name: /Chat 10/ })).toBeInTheDocument();
  });

  it('every row has a keyboard-reachable ⋯ menu (no hover-only actions)', () => {
    renderList();
    expect(screen.getByRole('button', { name: 'Actions for “First Chat”' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Actions for “Second Chat”' })).toBeInTheDocument();
  });

  it('Escape during rename cancels without invoking rename command', async () => {
    renderList({ sessions: [SESSION_1] });
    await fireEvent.click(screen.getByRole('button', { name: 'Actions for “First Chat”' }));
    await fireEvent.click(screen.getByRole('menuitem', { name: 'Rename' }));
    const input = screen.getByRole<HTMLInputElement>('textbox', { name: 'Chat title' });
    await fireEvent.keyDown(input, { key: 'Escape' });
    expect(mockInvoke).not.toHaveBeenCalled();
    expect(screen.queryByRole('textbox')).not.toBeInTheDocument();
  });

  it('confirming rename calls rename_chat_session', async () => {
    mockInvoke.mockResolvedValueOnce({ ...SESSION_1, title: 'Renamed Chat' });
    const onlistchange = vi.fn();
    renderList({ sessions: [SESSION_1], onlistchange });
    await fireEvent.click(screen.getByRole('button', { name: 'Actions for “First Chat”' }));
    await fireEvent.click(screen.getByRole('menuitem', { name: 'Rename' }));
    const input = screen.getByRole<HTMLInputElement>('textbox', { name: 'Chat title' });
    await fireEvent.input(input, { target: { value: 'Renamed Chat' } });
    await fireEvent.keyDown(input, { key: 'Enter' });
    await waitFor(() =>
      expect(mockInvoke).toHaveBeenCalledWith('rename_chat_session', {
        sessionId: 's1',
        title: 'Renamed Chat',
      })
    );
    await waitFor(() => expect(onlistchange).toHaveBeenCalled());
  });

  it('delete asks for confirmation before calling delete_chat_session', async () => {
    mockInvoke.mockResolvedValueOnce(undefined);
    const onsessiondeleted = vi.fn();
    renderList({ sessions: [SESSION_1], onsessiondeleted });
    await fireEvent.click(screen.getByRole('button', { name: 'Actions for “First Chat”' }));
    await fireEvent.click(screen.getByRole('menuitem', { name: 'Delete' }));

    const dialog = await screen.findByRole('dialog');
    expect(dialog).toHaveTextContent('Delete the chat “First Chat” and all its messages?');
    expect(mockInvoke).not.toHaveBeenCalled();

    await fireEvent.click(dialog.querySelector('[data-confirm]')!);
    await waitFor(() =>
      expect(mockInvoke).toHaveBeenCalledWith('delete_chat_session', { sessionId: 's1' })
    );
    await waitFor(() => expect(onsessiondeleted).toHaveBeenCalledWith('s1'));
  });

  it('cancelling the delete confirmation keeps the chat', async () => {
    renderList({ sessions: [SESSION_1] });
    await fireEvent.click(screen.getByRole('button', { name: 'Actions for “First Chat”' }));
    await fireEvent.click(screen.getByRole('menuitem', { name: 'Delete' }));
    await fireEvent.click(await screen.findByRole('button', { name: 'Cancel' }));
    expect(mockInvoke).not.toHaveBeenCalled();
    expect(screen.getByRole('link', { name: /First Chat/ })).toBeInTheDocument();
  });
});
