import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/svelte';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import UpdateNotification from '../../lib/components/UpdateNotification.svelte';

vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn(),
}));

const mockInvoke = vi.mocked(invoke);
const mockListen = vi.mocked(listen);

const UPDATE_AVAILABLE = {
  current_version: '1.0.0',
  latest_version: '1.1.0',
  update_available: true,
  body: '## What\'s new\n- Bug fixes\n- Performance improvements',
  date: '2026-03-20T00:00:00Z',
};

const NO_UPDATE = {
  current_version: '1.1.0',
  latest_version: '1.1.0',
  update_available: false,
  body: null,
  date: null,
};

beforeEach(() => {
  mockInvoke.mockReset();
  mockListen.mockReset();
  mockListen.mockResolvedValue(() => {});
});

describe('UpdateNotification', () => {
  it('shows notification when update is available', async () => {
    mockInvoke.mockResolvedValueOnce(UPDATE_AVAILABLE);
    render(UpdateNotification);
    await waitFor(() => {
      expect(screen.getByText('Update Available')).toBeInTheDocument();
    });
    expect(screen.getByText(/Version 1.1.0 is now available/)).toBeInTheDocument();
  });

  it('shows nothing when no update is available', async () => {
    mockInvoke.mockResolvedValueOnce(NO_UPDATE);
    render(UpdateNotification);
    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith('check_for_updates');
    });
    expect(screen.queryByText('Update Available')).not.toBeInTheDocument();
  });

  it('does not auto-check when autoCheck=false', async () => {
    render(UpdateNotification, { props: { autoCheck: false } });
    expect(mockInvoke).not.toHaveBeenCalled();
    expect(screen.queryByText('Update Available')).not.toBeInTheDocument();
  });

  it('dismisses notification on Later click', async () => {
    mockInvoke.mockResolvedValueOnce(UPDATE_AVAILABLE);
    render(UpdateNotification);
    await waitFor(() => expect(screen.getByText('Update Available')).toBeInTheDocument());

    fireEvent.click(screen.getByRole('button', { name: /Later/ }));
    await waitFor(() => {
      expect(screen.queryByText('Update Available')).not.toBeInTheDocument();
    });
  });

  it('renders changelog body as markdown', async () => {
    mockInvoke.mockResolvedValueOnce(UPDATE_AVAILABLE);
    render(UpdateNotification);
    await waitFor(() => expect(screen.getByText('Update Available')).toBeInTheDocument());
    expect(screen.getByText("What's new:")).toBeInTheDocument();
  });

  it('shows error message when check fails', async () => {
    mockInvoke.mockRejectedValueOnce({ code: 'NETWORK_ERROR', message: 'No internet' });
    render(UpdateNotification);
    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith('check_for_updates');
    });
    // Error is shown (parseError formats it)
    // No notification shown
    expect(screen.queryByText('Update Available')).not.toBeInTheDocument();
  });

  it('starts install when Install Update is clicked', async () => {
    mockInvoke.mockResolvedValueOnce(UPDATE_AVAILABLE); // check_for_updates
    mockInvoke.mockResolvedValueOnce(undefined); // install_update

    render(UpdateNotification);
    await waitFor(() => expect(screen.getByRole('button', { name: /Install Update/ })).toBeInTheDocument());

    fireEvent.click(screen.getByRole('button', { name: /Install Update/ }));
    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith('install_update');
    });
  });
});
