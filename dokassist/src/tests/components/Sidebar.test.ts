import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/svelte';
import Sidebar from '$lib/components/Sidebar.svelte';
import { authStatus } from '$lib/stores/auth';
import { resetEngineState, setEngineState } from '$lib/stores/engine';
import type { LlmEngineStatus } from '$lib/api';
import { invoke } from '@tauri-apps/api/core';
import { resetChatSessions } from '$lib/stores/chat-sessions';

const mockInvoke = vi.mocked(invoke);

const nav = vi.hoisted(() => ({
  page: { url: new URL('http://localhost/patients'), params: {}, route: { id: null } },
}));

vi.mock('$app/state', () => nav);
vi.mock('$app/navigation', () => ({ goto: vi.fn() }));

function at(path: string) {
  nav.page.url = new URL(`http://localhost${path}`);
}

beforeEach(() => {
  vi.clearAllMocks();
  authStatus.set(null);
  resetEngineState();
  resetChatSessions();
  mockInvoke.mockReset();
});

function current() {
  return screen
    .getAllByRole('link')
    .filter((link) => link.getAttribute('aria-current') === 'page')
    .map((link) => link.getAttribute('href'));
}

describe('Sidebar — structure (D2)', () => {
  it('orders the primary items Chat, Patients, Calendar, Reference', () => {
    at('/patients');
    render(Sidebar);
    const navEl = screen.getByRole('navigation', { name: 'Primary navigation' });
    const hrefs = [...navEl.querySelectorAll(':scope > ul > li > a')].map((a) =>
      a.getAttribute('href')
    );
    expect(hrefs).toEqual(['/chat', '/patients', '/calendar', '/reference']);
  });

  it('offers New chat, Search, Settings and Lock', () => {
    at('/patients');
    const onOpenPalette = vi.fn();
    render(Sidebar, { props: { onOpenPalette } });
    expect(screen.getByRole('link', { name: /New chat/ })).toHaveAttribute('href', '/chat?new=1');
    expect(screen.getByRole('link', { name: 'Settings' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Lock' })).toBeInTheDocument();
    screen.getByRole('button', { name: /Search/ }).click();
    expect(onOpenPalette).toHaveBeenCalledOnce();
  });
});

describe('Sidebar — active state is prefix-matched', () => {
  it.each([
    ['/patients', '/patients'],
    ['/patients/p1', '/patients'],
    ['/patients/p1/sessions/s1', '/patients'],
    ['/calendar', '/calendar'],
    ['/reference', '/reference'],
    ['/literature', '/reference'],
    ['/settings', '/settings'],
    ['/chat', '/chat'],
  ])('%s highlights %s', (path, expected) => {
    at(path);
    render(Sidebar);
    expect(current()).toEqual([expected]);
  });

  it('draws the brand bar only on the active item', () => {
    at('/patients/p1');
    const { container } = render(Sidebar);
    const bars = container.querySelectorAll('.bg-brand');
    expect(bars).toHaveLength(1);
    expect(bars[0].closest('a')).toHaveAttribute('href', '/patients');
  });

  it('a selected chat row is current instead of the Chat item', async () => {
    mockInvoke.mockImplementation(async (cmd: string, args?: unknown) => {
      if (cmd === 'list_chat_sessions') {
        return (args as { scope: string }).scope === 'global'
          ? [
              {
                id: 'c1',
                scope: 'global',
                patient_id: null,
                title: 'Verlauf',
                created_at: '2026-11-17T08:00:00Z',
                updated_at: '2026-11-17T08:00:00Z',
              },
            ]
          : [];
      }
      return undefined;
    });
    at('/chat?sessionId=c1');
    authStatus.set('unlocked');
    const { container } = render(Sidebar);
    const row = await screen.findByRole('link', { name: /Verlauf/ });
    expect(row).toHaveAttribute('aria-current', 'page');
    expect(screen.getByRole('link', { name: 'Chat' })).not.toHaveAttribute('aria-current');
    expect(container.querySelectorAll('.bg-brand')).toHaveLength(1);
  });
});

describe('Sidebar — rail and toggle', () => {
  it('renders the full sidebar when wide and not collapsed', () => {
    at('/patients');
    const { container } = render(Sidebar, { props: { wide: true, collapsed: false } });
    expect(container.querySelector('aside')).toHaveAttribute('data-sidebar-mode', 'full');
    expect(screen.getByText('RamDoc')).toBeInTheDocument();
  });

  it('collapses to the icon rail when the user collapsed it', () => {
    at('/patients');
    const { container } = render(Sidebar, { props: { wide: true, collapsed: true } });
    expect(container.querySelector('aside')).toHaveAttribute('data-sidebar-mode', 'rail');
    // Rail items keep their accessible names.
    expect(screen.getByRole('link', { name: 'Patients' })).toHaveAttribute('aria-current', 'page');
  });

  it('is always a rail below 1024px, and the toggle opens the overlay', async () => {
    at('/patients');
    const onToggle = vi.fn();
    const { container } = render(Sidebar, { props: { wide: false, collapsed: false, onToggle } });
    expect(container.querySelector('aside')).toHaveAttribute('data-sidebar-mode', 'rail');
    await fireEvent.click(screen.getByRole('button', { name: 'Open navigation' }));
    expect(onToggle).toHaveBeenCalledOnce();
  });

  it('overlay mode floats the full sidebar and closes on Escape', async () => {
    at('/patients');
    const onOverlayClose = vi.fn();
    const { container } = render(Sidebar, {
      props: { wide: false, overlayOpen: true, onOverlayClose },
    });
    expect(container.querySelector('aside')).toHaveAttribute('data-sidebar-mode', 'overlay');
    await fireEvent.keyDown(window, { key: 'Escape' });
    expect(onOverlayClose).toHaveBeenCalled();
  });
});

describe('Sidebar — model status', () => {
  const status = (over: Partial<LlmEngineStatus>) =>
    ({
      is_loaded: false,
      is_downloaded: false,
      model_name: null,
      desired_model: null,
      lifecycle: { phase: 'idle', requested_filename: null, active_filename: null, error: null },
      ...over,
    }) as unknown as LlmEngineStatus;

  it('labels a loaded model with its name and links to settings', () => {
    at('/patients');
    setEngineState({
      status: status({
        is_loaded: true,
        is_downloaded: true,
        desired_model: { id: 'q', name: 'Qwen3 14B', filename: 'q.gguf', exists_on_disk: true },
      }),
      isLoading: false,
      loadingFilename: null,
      loadingStartedAt: null,
      error: null,
    });
    const { container } = render(Sidebar);
    const chip = container.querySelector('[data-model-status]')!;
    expect(chip).toHaveAttribute('data-model-status', 'ok');
    expect(chip).toHaveAttribute('href', '/settings');
    expect(chip).toHaveTextContent('Model ready');
    expect(chip).toHaveTextContent('Qwen3 14B');
  });

  it('shows the loading state with text, not colour alone', () => {
    at('/patients');
    setEngineState({
      status: status({ is_downloaded: true }),
      isLoading: true,
      loadingFilename: 'qwen.gguf',
      loadingStartedAt: Date.now(),
      error: null,
    });
    const { container } = render(Sidebar);
    const chip = container.querySelector('[data-model-status]')!;
    expect(chip).toHaveAttribute('data-model-status', 'busy');
    expect(chip).toHaveTextContent('Loading model');
    expect(chip).toHaveTextContent('qwen');
  });

  it('says when no model is installed', () => {
    at('/patients');
    setEngineState({
      status: status({}),
      isLoading: false,
      loadingFilename: null,
      loadingStartedAt: null,
      error: null,
    });
    const { container } = render(Sidebar);
    expect(container.querySelector('[data-model-status]')).toHaveTextContent('No model installed');
  });
});
