import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/svelte';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { resetEngineState } from '$lib/stores/engine';
import { maskAhv } from '$lib/components/chat/patient-context';

const pageState = vi.hoisted(() => ({
  url: new URL('http://localhost/chat'),
  params: {},
  route: { id: '/chat' },
}));

vi.mock('$app/state', () => ({ page: pageState }));
vi.mock('$app/navigation', () => ({ goto: vi.fn() }));
vi.mock('@tauri-apps/api/event', () => ({ listen: vi.fn() }));

import ChatPage from '../../routes/chat/+page.svelte';

const mockInvoke = vi.mocked(invoke);

const PATIENT = {
  id: 'p1',
  first_name: 'Anna',
  last_name: 'Müller',
  date_of_birth: '1984-03-12',
  gender: null,
  ahv_number: '756.1234.5678.97',
  address: null,
  phone: null,
  email: null,
  insurance: null,
  gp_name: null,
  gp_address: null,
  notes: null,
  created_at: '',
  updated_at: '',
};

const SESSIONS = [
  {
    id: 'c1',
    scope: 'patient',
    patient_id: 'p1',
    title: 'Course of treatment',
    created_at: '2026-11-17 08:00:00',
    updated_at: '2026-11-17 08:00:00',
  },
  {
    id: 'c2',
    scope: 'global',
    patient_id: null,
    title: 'Interactions',
    created_at: '2026-11-14 08:00:00',
    updated_at: '2026-11-14 08:00:00',
  },
];

function mockBackend() {
  mockInvoke.mockImplementation((cmd, args) => {
    const a = (args ?? {}) as Record<string, unknown>;
    switch (cmd) {
      case 'list_chat_sessions':
        return Promise.resolve(SESSIONS.filter((s) => s.scope === a.scope));
      case 'get_chat_messages':
        return Promise.resolve([]);
      case 'get_patient':
        return Promise.resolve(PATIENT);
      case 'get_dashboard_data':
        return Promise.resolve({
          todays_sessions: [],
          recent_patients: [],
          sessions_with_incomplete_notes: [],
        });
      case 'list_diagnoses_for_patient':
        return Promise.resolve([
          {
            id: 'd1',
            patient_id: 'p1',
            icd10_code: 'F33.1',
            description: 'Recurrent depressive disorder',
            status: 'active',
            diagnosed_date: '2023-04-02',
            resolved_date: null,
            notes: null,
            created_at: '',
            updated_at: '',
          },
        ]);
      case 'list_medications_for_patient':
      case 'list_scores_for_patient':
      case 'list_sessions_for_patient':
        return Promise.resolve([]);
      default:
        return Promise.resolve(null);
    }
  });
}

function setWidth(wide: boolean) {
  Object.defineProperty(window, 'matchMedia', {
    configurable: true,
    writable: true,
    value: vi.fn().mockImplementation((query: string) => ({
      matches: wide,
      media: query,
      addEventListener: vi.fn(),
      removeEventListener: vi.fn(),
    })),
  });
}

beforeEach(() => {
  mockInvoke.mockReset();
  resetEngineState();
  vi.mocked(listen).mockResolvedValue(vi.fn());
  window.HTMLElement.prototype.scrollIntoView = vi.fn();
  mockBackend();
});

afterEach(() => {
  // @ts-expect-error -- jsdom has no matchMedia; remove the stub between tests.
  delete window.matchMedia;
});

describe('chat page', () => {
  it('shows the Today panel and no patient panel in a fresh chat', async () => {
    setWidth(true);
    pageState.url = new URL('http://localhost/chat');
    render(ChatPage);
    expect(await screen.findByTestId('today-panel')).toBeInTheDocument();
    expect(screen.queryByTestId('patient-context-panel')).not.toBeInTheDocument();
    expect(
      screen.queryByRole('button', { name: /patient context/i })
    ).not.toBeInTheDocument();
  });

  it('docks the patient context panel open by default at 1280 px and wider', async () => {
    setWidth(true);
    pageState.url = new URL('http://localhost/chat?sessionId=c1');
    render(ChatPage);
    const panel = await screen.findByTestId('patient-context-panel');
    expect(panel).toBeInTheDocument();
    await waitFor(() => expect(screen.getByText('F33.1')).toBeInTheDocument());
    expect(screen.getByTestId('patient-ahv')).toHaveTextContent('756.••••.••••.97');
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
    expect(screen.getByTestId('attached-patient-chip')).toHaveTextContent('Anna Müller');

    await fireEvent.click(screen.getByRole('button', { name: 'Hide patient context' }));
    expect(screen.queryByTestId('patient-context-panel')).not.toBeInTheDocument();
  });

  it('keeps the panel closed below 1280 px and opens it as a sheet', async () => {
    setWidth(false);
    pageState.url = new URL('http://localhost/chat?sessionId=c1');
    render(ChatPage);
    const toggle = await screen.findByRole('button', { name: 'Show patient context' });
    expect(screen.queryByTestId('patient-context-panel')).not.toBeInTheDocument();

    await fireEvent.click(toggle);
    const sheet = screen.getByRole('dialog', { name: 'Patient context' });
    expect(sheet).toContainElement(screen.getByTestId('patient-context-panel'));

    await fireEvent.click(screen.getByRole('button', { name: 'Close patient context' }));
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
  });

  it('attaches the patient from ?patientId= to a fresh chat', async () => {
    setWidth(true);
    pageState.url = new URL('http://localhost/chat?patientId=p1');
    render(ChatPage);
    expect(await screen.findByTestId('attached-patient-chip')).toHaveTextContent('Anna Müller');
    expect(screen.getByRole('radio', { name: 'Records only · cited' })).toBeInTheDocument();
    expect(screen.queryByTestId('today-panel')).not.toBeInTheDocument();
    expect(mockInvoke).not.toHaveBeenCalledWith('create_chat_session', expect.anything());
  });
});

describe('maskAhv', () => {
  it('keeps the country prefix and check digits only', () => {
    expect(maskAhv('756.1234.5678.97')).toBe('756.••••.••••.97');
    expect(maskAhv('7561234567897')).toBe('756.••••.••••.97');
    expect(maskAhv('12345')).toBe('•••45');
  });
});
