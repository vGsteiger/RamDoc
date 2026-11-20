import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/svelte';
import { invoke } from '@tauri-apps/api/core';
import { goto } from '$app/navigation';
import CommandPalette from '$lib/components/CommandPalette.svelte';
import type { Patient } from '$lib/api';

const mockInvoke = vi.mocked(invoke);

const PATIENT = {
  id: 'p1',
  first_name: 'Anna',
  last_name: 'Müller',
  date_of_birth: '1984-03-12',
  ahv_number: '756.1234.5678.97',
} as unknown as Patient;

vi.mock('$app/navigation', () => ({
  goto: vi.fn(),
}));

beforeEach(() => {
  vi.clearAllMocks();
  mockInvoke.mockReset();
  mockInvoke.mockResolvedValue([]);
});

function renderPalette(props: Record<string, unknown> = {}) {
  return render(CommandPalette, {
    props: { isOpen: true, onClose: vi.fn(), patients: [], ...props },
  });
}

describe('CommandPalette — dialog semantics', () => {
  it('exposes a focusable modal dialog with an accessible name', () => {
    renderPalette();
    const dialog = screen.getByRole('dialog', { name: /command palette/i });
    expect(dialog).toHaveAttribute('aria-modal', 'true');
    expect(dialog).toHaveAttribute('tabindex', '-1');
  });

  it('renders nothing while closed', () => {
    renderPalette({ isOpen: false });
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
  });

  it('moves focus to the search input when opened', async () => {
    vi.useFakeTimers();
    try {
      renderPalette();
      const input = screen.getByPlaceholderText(/search patients, chats/i);
      await vi.advanceTimersByTimeAsync(100);
      expect(document.activeElement).toBe(input);
    } finally {
      vi.useRealTimers();
    }
  });
});

describe('CommandPalette — keyboard handling', () => {
  it('closes on Escape from the search input', async () => {
    const onClose = vi.fn();
    renderPalette({ onClose });
    await fireEvent.keyDown(screen.getByPlaceholderText(/search patients, chats/i), {
      key: 'Escape',
    });
    expect(onClose).toHaveBeenCalledOnce();
  });

  it('closes on Escape when focus is on the dialog itself', async () => {
    const onClose = vi.fn();
    renderPalette({ onClose });
    await fireEvent.keyDown(screen.getByRole('dialog'), { key: 'Escape' });
    expect(onClose).toHaveBeenCalledOnce();
  });

  it('advances the selection by one item per ArrowDown', async () => {
    renderPalette();
    const input = screen.getByRole('combobox');
    const newChat = screen.getByRole('option', { name: /new chat/i });
    const newPatient = screen.getByRole('option', { name: /new patient/i });

    expect(newChat).toHaveAttribute('aria-selected', 'true');
    expect(input).toHaveAttribute('aria-activedescendant', newChat.id);

    await fireEvent.keyDown(input, { key: 'ArrowDown' });
    await waitFor(() => expect(newPatient).toHaveAttribute('aria-selected', 'true'));
    expect(newChat).toHaveAttribute('aria-selected', 'false');
    expect(input).toHaveAttribute('aria-activedescendant', newPatient.id);

    await fireEvent.keyDown(input, { key: 'ArrowUp' });
    await waitFor(() => expect(newChat).toHaveAttribute('aria-selected', 'true'));
  });

  it('runs the selected item on Enter', async () => {
    const onClose = vi.fn();
    renderPalette({ onClose });
    const input = screen.getByPlaceholderText(/search patients, chats/i);
    await fireEvent.keyDown(input, { key: 'Enter' });
    await waitFor(() => expect(onClose).toHaveBeenCalled());
    expect(goto).toHaveBeenCalledWith('/chat?new=1');
  });
});

describe('CommandPalette — contents', () => {
  it('has no Dashboard entry and navigates to Reference (D3, D9)', () => {
    renderPalette();
    expect(screen.queryByRole('option', { name: /dashboard/i })).not.toBeInTheDocument();
    expect(screen.getByRole('option', { name: /go to reference/i })).toBeInTheDocument();
  });

  it('lists matching patients for a query', async () => {
    renderPalette({ patients: [PATIENT] });
    const input = screen.getByRole('combobox');
    await fireEvent.input(input, { target: { value: 'müll' } });
    expect(await screen.findByRole('option', { name: /Anna Müller/ })).toBeInTheDocument();
  });

  it('shows record search hits with highlighted matches, never raw tags', async () => {
    mockInvoke.mockImplementation(async (cmd: string) =>
      cmd === 'global_search'
        ? [
            {
              result_type: 'session',
              entity_id: 's4',
              patient_id: 'p1',
              patient_name: 'Anna Müller',
              title: 'Psychotherapie',
              snippet: 'Exploration der <mark>Belastung</mark>situation <b>x</b>',
              date: '2026-11-10',
              rank: 1,
            },
          ]
        : []
    );
    renderPalette({ patients: [] });
    await fireEvent.input(screen.getByRole('combobox'), { target: { value: 'Belastung' } });
    const option = await screen.findByRole('option', { name: /Psychotherapie/ }, { timeout: 2000 });
    const mark = option.querySelector('mark');
    expect(mark).toHaveTextContent('Belastung');
    // Other tags print as text rather than being dropped or interpreted.
    expect(option).toHaveTextContent('situation <b>x</b>');
    expect(option.querySelector('b')).toBeNull();
    expect(option).toHaveTextContent('10.11.2026');

    await fireEvent.click(option);
    expect(goto).toHaveBeenCalledWith('/patients/p1/sessions/s4');
  });
});

describe('CommandPalette — backdrop', () => {
  it('closes when the backdrop is clicked', async () => {
    const onClose = vi.fn();
    const { container } = renderPalette({ onClose });
    const backdrop = container.querySelector('[role="presentation"]')!;
    await fireEvent.click(backdrop);
    expect(onClose).toHaveBeenCalledOnce();
  });

  it('stays open when a click lands inside the dialog', async () => {
    const onClose = vi.fn();
    renderPalette({ onClose });
    await fireEvent.click(screen.getByRole('dialog'));
    expect(onClose).not.toHaveBeenCalled();
  });
});
