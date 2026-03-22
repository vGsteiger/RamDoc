import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/svelte';
import { invoke } from '@tauri-apps/api/core';
import ChatSessionList from '../../lib/components/ChatSessionList.svelte';
import type { ChatSession } from '$lib/api';

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
  scope: 'global',
  patient_id: null,
  title: 'Second Chat',
  created_at: '2026-01-02T00:00:00Z',
  updated_at: '2026-01-02T00:00:00Z',
};

beforeEach(() => {
  mockInvoke.mockReset();
});

describe('ChatSessionList', () => {
  it('renders all session titles', () => {
    render(ChatSessionList, {
      props: {
        sessions: [SESSION_1, SESSION_2],
        activeSessionId: null,
        onsessionselect: vi.fn(),
        onsessionnew: vi.fn(),
      },
    });
    expect(screen.getByText('First Chat')).toBeInTheDocument();
    expect(screen.getByText('Second Chat')).toBeInTheDocument();
  });

  it('shows empty state placeholder when no sessions', () => {
    render(ChatSessionList, {
      props: {
        sessions: [],
        activeSessionId: null,
        onsessionselect: vi.fn(),
        onsessionnew: vi.fn(),
      },
    });
    expect(screen.getByText('Keine Chats vorhanden')).toBeInTheDocument();
  });

  it('"Neuer Chat" button calls onsessionnew', async () => {
    const onsessionnew = vi.fn();
    render(ChatSessionList, {
      props: {
        sessions: [],
        activeSessionId: null,
        onsessionselect: vi.fn(),
        onsessionnew,
      },
    });
    await fireEvent.click(screen.getByText('Neuer Chat'));
    expect(onsessionnew).toHaveBeenCalledOnce();
  });

  it('clicking a session calls onsessionselect with its id', async () => {
    const onsessionselect = vi.fn();
    render(ChatSessionList, {
      props: {
        sessions: [SESSION_1],
        activeSessionId: null,
        onsessionselect,
        onsessionnew: vi.fn(),
      },
    });
    await fireEvent.click(screen.getByText('First Chat'));
    expect(onsessionselect).toHaveBeenCalledWith('s1');
  });

  it('Escape during rename cancels without invoking rename command', async () => {
    render(ChatSessionList, {
      props: {
        sessions: [SESSION_1],
        activeSessionId: null,
        onsessionselect: vi.fn(),
        onsessionnew: vi.fn(),
      },
    });
    // Trigger rename by clicking the rename button (Pencil icon has title "Umbenennen")
    const renameBtn = screen.getByTitle('Umbenennen');
    await fireEvent.click(renameBtn);
    const input = screen.getByRole<HTMLInputElement>('textbox');
    expect(input).toBeInTheDocument();
    await fireEvent.keyDown(input, { key: 'Escape' });
    expect(mockInvoke).not.toHaveBeenCalled();
    expect(screen.queryByRole('textbox')).not.toBeInTheDocument();
  });

  it('confirming rename calls rename_chat_session', async () => {
    mockInvoke.mockResolvedValueOnce({ ...SESSION_1, title: 'Renamed Chat' });
    render(ChatSessionList, {
      props: {
        sessions: [SESSION_1],
        activeSessionId: null,
        onsessionselect: vi.fn(),
        onsessionnew: vi.fn(),
      },
    });
    const renameBtn = screen.getByTitle('Umbenennen');
    await fireEvent.click(renameBtn);
    const input = screen.getByRole<HTMLInputElement>('textbox');
    await fireEvent.input(input, { target: { value: 'Renamed Chat' } });
    await fireEvent.keyDown(input, { key: 'Enter' });
    await waitFor(() =>
      expect(mockInvoke).toHaveBeenCalledWith('rename_chat_session', {
        sessionId: 's1',
        title: 'Renamed Chat',
      })
    );
  });

  it('delete button calls delete_chat_session', async () => {
    mockInvoke.mockResolvedValueOnce(undefined);
    render(ChatSessionList, {
      props: {
        sessions: [SESSION_1],
        activeSessionId: null,
        onsessionselect: vi.fn(),
        onsessionnew: vi.fn(),
      },
    });
    const deleteBtn = screen.getByTitle('Löschen');
    await fireEvent.click(deleteBtn);
    await waitFor(() =>
      expect(mockInvoke).toHaveBeenCalledWith('delete_chat_session', { sessionId: 's1' })
    );
  });
});
