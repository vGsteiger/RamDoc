import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/svelte';
import { createRawSnippet } from 'svelte';
import { invoke } from '@tauri-apps/api/core';
import { goto } from '$app/navigation';
import Layout from '../../routes/+layout.svelte';
import { sidebarCollapsed } from '$lib/stores/sidebar';
import { get } from 'svelte/store';

const nav = vi.hoisted(() => ({
  page: { url: new URL('http://localhost/patients/p1'), params: {}, route: { id: null } },
}));

vi.mock('$app/state', () => nav);
vi.mock('$app/navigation', () => ({ goto: vi.fn() }));

const mockInvoke = vi.mocked(invoke);

const children = createRawSnippet(() => ({ render: () => '<p>Page body</p>' }));

function at(path: string) {
  nav.page.url = new URL(`http://localhost${path}`);
}

beforeEach(() => {
  vi.clearAllMocks();
  mockInvoke.mockReset();
  mockInvoke.mockImplementation(async (cmd: string) => {
    if (cmd === 'check_auth') return 'unlocked';
    if (cmd === 'list_chat_sessions' || cmd === 'list_patients') return [];
    return null;
  });
  sidebarCollapsed.set(false);
});

const press = (key: string, mods: Record<string, boolean> = {}) =>
  fireEvent.keyDown(window, { key, ...mods });

describe('App shell — layout', () => {
  it('renders a skip link to the main region and no top bar', () => {
    at('/patients/p1');
    render(Layout, { props: { children } });
    const skip = screen.getByRole('link', { name: 'Skip to content' });
    expect(skip).toHaveAttribute('href', '#main-content');
    expect(document.getElementById('main-content')?.tagName).toBe('MAIN');
    expect(screen.queryByRole('banner')).not.toBeInTheDocument();
    expect(screen.queryByRole('searchbox')).not.toBeInTheDocument();
  });

  it('auth routes render without the shell', () => {
    at('/unlock');
    render(Layout, { props: { children } });
    expect(screen.getByText('Page body')).toBeInTheDocument();
    expect(screen.queryByRole('navigation')).not.toBeInTheDocument();
  });

  it('still redirects a locked vault to /unlock', async () => {
    mockInvoke.mockImplementation(async (cmd: string) => (cmd === 'check_auth' ? 'locked' : null));
    at('/patients');
    render(Layout, { props: { children } });
    await waitFor(() => expect(goto).toHaveBeenCalledWith('/unlock', { replaceState: true }));
  });
});

describe('App shell — global shortcuts (D10)', () => {
  it('Cmd+N starts a new chat', async () => {
    at('/patients/p1');
    render(Layout, { props: { children } });
    await press('n', { metaKey: true });
    expect(goto).toHaveBeenCalledWith('/chat?new=1');
  });

  it('Cmd+Shift+N creates a new patient', async () => {
    at('/calendar');
    render(Layout, { props: { children } });
    await press('N', { metaKey: true, shiftKey: true });
    expect(goto).toHaveBeenCalledWith('/patients/new');
    expect(goto).not.toHaveBeenCalledWith('/chat?new=1');
  });

  it('Cmd+Shift+S starts a session on a patient page', async () => {
    at('/patients/p1');
    render(Layout, { props: { children } });
    await press('S', { metaKey: true, shiftKey: true });
    expect(goto).toHaveBeenCalledWith('/patients/p1/sessions/new');
  });

  it('Cmd+K opens the palette, Esc closes it, and Cmd+N is ignored while it is open', async () => {
    at('/patients/p1');
    render(Layout, { props: { children } });
    await press('k', { metaKey: true });
    const dialog = await screen.findByRole('dialog', { name: /command palette/i });
    await press('n', { metaKey: true });
    expect(goto).not.toHaveBeenCalled();
    await fireEvent.keyDown(dialog, { key: 'Escape' });
    await waitFor(() => expect(screen.queryByRole('dialog')).not.toBeInTheDocument());
  });

  it('Cmd+\\ collapses the sidebar and persists the choice', async () => {
    at('/patients/p1');
    render(Layout, { props: { children } });
    await press('\\', { metaKey: true });
    expect(get(sidebarCollapsed)).toBe(true);
    expect(localStorage.getItem('sidebar-collapsed')).toBe('true');
  });

  it('ignores shortcuts on auth routes', async () => {
    at('/unlock');
    render(Layout, { props: { children } });
    await press('n', { metaKey: true });
    await press('k', { metaKey: true });
    expect(goto).not.toHaveBeenCalled();
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
  });
});
