import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/svelte';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { goto } from '$app/navigation';
import EmailCompose from '../../routes/patients/[id]/email/new/+page.svelte';

vi.mock('$app/stores', () => {
  const { readable } = require('svelte/store');
  return {
    page: readable({
      params: { id: 'patient-1' },
      url: new URL('http://localhost/patients/patient-1/email/new'),
      route: { id: null },
    }),
  };
});

vi.mock('$app/navigation', () => ({
  goto: vi.fn(),
}));

vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn(),
}));

const mockInvoke = vi.mocked(invoke);
const mockListen = vi.mocked(listen);
const mockGoto = vi.mocked(goto);

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

const PATIENT = {
  id: 'patient-1',
  first_name: 'Anna',
  last_name: 'Müller',
  date_of_birth: '1975-06-20',
  ahv_number: '7561234567897',
  gender: 'female',
  address: 'Bahnhofstrasse 1, 8001 Zürich',
  phone: '+41 44 123 45 67',
  email: 'anna.mueller@example.com',
  insurance: 'Helsana',
  gp_name: 'Dr. Keller',
  gp_address: 'Arztgasse 5, 8001 Zürich',
  notes: '',
  created_at: '2025-01-01T00:00:00Z',
  updated_at: '2025-01-01T00:00:00Z',
};

const ENGINE_LOADED = {
  is_loaded: true,
  model_name: 'Phi-4 Mini Q4_K_M',
  model_path: '/models/phi4.gguf',
  total_ram_bytes: 16 * 1024 ** 3,
  is_downloaded: true,
  downloaded_filename: 'phi4.gguf',
};

const ENGINE_NOT_LOADED = {
  is_loaded: false,
  model_name: null,
  model_path: null,
  total_ram_bytes: 8 * 1024 ** 3,
  is_downloaded: false,
  downloaded_filename: null,
};

const CHAT_SESSION = {
  id: 'session-1',
  scope: 'patient',
  patient_id: 'patient-1',
  title: 'Email Draft',
  created_at: '2026-01-01T00:00:00Z',
  updated_at: '2026-01-01T00:00:00Z',
};

const EMAIL = {
  id: 'email-1',
  patient_id: 'patient-1',
  recipient_email: 'anna.mueller@example.com',
  subject: 'Follow-up',
  body: 'Hello Anna,\n\nBest regards.',
  status: 'draft',
  created_at: '2026-01-01T00:00:00Z',
  updated_at: '2026-01-01T00:00:00Z',
};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/** Captures event handlers registered via listen() so tests can fire them. */
function captureListeners() {
  const handlers: Record<string, (e: { payload: unknown }) => void> = {};
  mockListen.mockImplementation((event, handler) => {
    handlers[event] = handler as (e: { payload: unknown }) => void;
    return Promise.resolve(() => {});
  });
  return handlers;
}

/** Mount the component with model loaded (default happy path). */
function setupLoaded() {
  mockInvoke
    .mockResolvedValueOnce(PATIENT)        // get_patient
    .mockResolvedValueOnce(ENGINE_LOADED); // get_engine_status
}

/** Mount the component with model NOT loaded. */
function setupNotLoaded() {
  mockInvoke
    .mockResolvedValueOnce(PATIENT)           // get_patient
    .mockResolvedValueOnce(ENGINE_NOT_LOADED);// get_engine_status
}

beforeEach(() => {
  mockInvoke.mockReset();
  mockListen.mockReset();
  mockGoto.mockReset();
  // Default: listen returns a no-op unlisten function.
  mockListen.mockResolvedValue(() => {});
});

// ---------------------------------------------------------------------------
// Initial render
// ---------------------------------------------------------------------------

describe('EmailCompose — initial render', () => {
  it('shows the "Compose Email" heading immediately', () => {
    mockInvoke.mockReturnValue(new Promise(() => {}));
    render(EmailCompose);
    expect(screen.getByRole('heading', { name: /compose email/i })).toBeInTheDocument();
  });

  it('shows the patient name after mount', async () => {
    setupLoaded();
    render(EmailCompose);
    await waitFor(() => {
      expect(screen.getByText(/anna müller/i)).toBeInTheDocument();
    });
  });

  it('pre-fills the recipient email from the patient record', async () => {
    setupLoaded();
    render(EmailCompose);
    await waitFor(() => {
      expect(screen.getByDisplayValue('anna.mueller@example.com')).toBeInTheDocument();
    });
  });

  it('renders To, Subject and Message fields', async () => {
    setupLoaded();
    render(EmailCompose);
    await waitFor(() => {
      expect(screen.getByLabelText(/to:/i)).toBeInTheDocument();
      expect(screen.getByLabelText(/subject:/i)).toBeInTheDocument();
      expect(screen.getByLabelText(/message:/i)).toBeInTheDocument();
    });
  });

  it('renders Save Draft and Open in Mail Client buttons', () => {
    mockInvoke.mockReturnValue(new Promise(() => {}));
    render(EmailCompose);
    expect(screen.getByRole('button', { name: /save draft/i })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /open in mail client/i })).toBeInTheDocument();
  });
});

// ---------------------------------------------------------------------------
// AI Assist panel — collapsed by default
// ---------------------------------------------------------------------------

describe('EmailCompose — AI Assist panel', () => {
  it('does not show the prompt textarea before the panel is opened', async () => {
    setupLoaded();
    render(EmailCompose);
    await waitFor(() => expect(screen.getByText(/anna müller/i)).toBeInTheDocument());
    expect(screen.queryByLabelText(/what should this email say/i)).not.toBeInTheDocument();
  });

  it('expands the panel when the AI Assist button is clicked', async () => {
    setupLoaded();
    render(EmailCompose);
    await waitFor(() => expect(screen.getByText(/anna müller/i)).toBeInTheDocument());

    await fireEvent.click(screen.getByRole('button', { name: /ai assist/i }));
    expect(screen.getByLabelText(/what should this email say/i)).toBeInTheDocument();
  });

  it('shows the Generate Draft button when the model is loaded', async () => {
    setupLoaded();
    render(EmailCompose);
    await waitFor(() => expect(screen.getByText(/anna müller/i)).toBeInTheDocument());

    await fireEvent.click(screen.getByRole('button', { name: /ai assist/i }));
    expect(screen.getByRole('button', { name: /generate draft/i })).toBeInTheDocument();
  });

  it('shows "No AI model is loaded" when model is not loaded', async () => {
    setupNotLoaded();
    render(EmailCompose);
    await waitFor(() => expect(screen.getByText(/anna müller/i)).toBeInTheDocument());

    await fireEvent.click(screen.getByRole('button', { name: /ai assist/i }));
    expect(screen.getByText(/no ai model is loaded/i)).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: /generate draft/i })).not.toBeInTheDocument();
  });
});

// ---------------------------------------------------------------------------
// AI generation
// ---------------------------------------------------------------------------

describe('EmailCompose — AI generation', () => {
  it('calls create_chat_session and run_agent_turn when Generate Draft is clicked', async () => {
    setupLoaded();
    mockInvoke
      .mockResolvedValueOnce(CHAT_SESSION)    // create_chat_session
      .mockResolvedValueOnce({ session_id: 'session-1', final_answer: '', tool_calls_made: [] }); // run_agent_turn

    render(EmailCompose);
    await waitFor(() => expect(screen.getByText(/anna müller/i)).toBeInTheDocument());

    await fireEvent.click(screen.getByRole('button', { name: /ai assist/i }));
    await fireEvent.click(screen.getByRole('button', { name: /generate draft/i }));

    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith('create_chat_session', expect.objectContaining({ patientId: 'patient-1' }));
      expect(mockInvoke).toHaveBeenCalledWith('run_agent_turn', expect.objectContaining({ sessionId: 'session-1' }));
    });
  });

  it('shows "Generating…" label while streaming and disables Save/Send', async () => {
    const handlers = captureListeners();
    setupLoaded();
    // create_chat_session resolves; run_agent_turn never resolves (simulates streaming)
    mockInvoke
      .mockResolvedValueOnce(CHAT_SESSION)
      .mockReturnValueOnce(new Promise(() => {}));

    render(EmailCompose);
    await waitFor(() => expect(screen.getByText(/anna müller/i)).toBeInTheDocument());

    await fireEvent.click(screen.getByRole('button', { name: /ai assist/i }));
    await fireEvent.click(screen.getByRole('button', { name: /generate draft/i }));

    await waitFor(() => {
      expect(screen.getByRole('button', { name: /generating/i })).toBeInTheDocument();
      expect(screen.getByRole('button', { name: /save draft/i })).toBeDisabled();
      expect(screen.getByRole('button', { name: /open in mail client/i })).toBeDisabled();
    });
  });

  it('appends agent-chunk tokens into the message body', async () => {
    const handlers = captureListeners();
    setupLoaded();
    mockInvoke
      .mockResolvedValueOnce(CHAT_SESSION)
      .mockReturnValueOnce(new Promise(() => {}));

    render(EmailCompose);
    await waitFor(() => expect(screen.getByText(/anna müller/i)).toBeInTheDocument());
    // Listeners are registered during onMount; wait for them to be set up.
    await waitFor(() => expect(handlers['agent-chunk']).toBeDefined());

    await fireEvent.click(screen.getByRole('button', { name: /ai assist/i }));
    await fireEvent.click(screen.getByRole('button', { name: /generate draft/i }));

    // Simulate streaming tokens
    handlers['agent-chunk']({ payload: 'Dear Anna,' });
    handlers['agent-chunk']({ payload: '\n\nYour next appointment' });

    await waitFor(() => {
      expect(screen.getByDisplayValue(/Dear Anna,/)).toBeInTheDocument();
    });
  });

  it('clears the generating state when agent-done fires', async () => {
    const handlers = captureListeners();
    setupLoaded();
    mockInvoke
      .mockResolvedValueOnce(CHAT_SESSION)
      .mockReturnValueOnce(new Promise(() => {}));

    render(EmailCompose);
    await waitFor(() => expect(handlers['agent-done']).toBeDefined());

    await fireEvent.click(screen.getByRole('button', { name: /ai assist/i }));
    await fireEvent.click(screen.getByRole('button', { name: /generate draft/i }));

    await waitFor(() =>
      expect(screen.getByRole('button', { name: /generating/i })).toBeInTheDocument()
    );

    handlers['agent-done']({ payload: undefined });

    await waitFor(() => {
      expect(screen.queryByRole('button', { name: /generating/i })).not.toBeInTheDocument();
      expect(screen.getByRole('button', { name: /generate draft/i })).toBeInTheDocument();
    });
  });

  it('shows an error and clears generating state when agent-error fires', async () => {
    const handlers = captureListeners();
    setupLoaded();
    mockInvoke
      .mockResolvedValueOnce(CHAT_SESSION)
      .mockReturnValueOnce(new Promise(() => {}));

    render(EmailCompose);
    await waitFor(() => expect(handlers['agent-error']).toBeDefined());

    await fireEvent.click(screen.getByRole('button', { name: /ai assist/i }));
    await fireEvent.click(screen.getByRole('button', { name: /generate draft/i }));

    handlers['agent-error']({ payload: { message: 'LLM failed to generate' } });

    await waitFor(() => {
      expect(screen.getByText(/LLM failed to generate/i)).toBeInTheDocument();
      expect(screen.queryByRole('button', { name: /generating/i })).not.toBeInTheDocument();
    });
  });
});

// ---------------------------------------------------------------------------
// Form validation
// ---------------------------------------------------------------------------

describe('EmailCompose — form validation', () => {
  it('shows a validation error when saving with an empty subject', async () => {
    setupLoaded();
    render(EmailCompose);
    await waitFor(() => expect(screen.getByText(/anna müller/i)).toBeInTheDocument());

    // Clear the pre-filled recipient and leave subject/body empty
    await fireEvent.input(screen.getByLabelText(/to:/i), { target: { value: '' } });
    await fireEvent.click(screen.getByRole('button', { name: /save draft/i }));

    expect(screen.getByText(/please fill in all fields/i)).toBeInTheDocument();
  });

  it('shows a validation error when sending with an empty body', async () => {
    setupLoaded();
    render(EmailCompose);
    await waitFor(() => expect(screen.getByText(/anna müller/i)).toBeInTheDocument());

    await fireEvent.input(screen.getByLabelText(/subject:/i), { target: { value: 'Test' } });
    // body is still empty
    await fireEvent.click(screen.getByRole('button', { name: /open in mail client/i }));

    expect(screen.getByText(/please fill in all fields/i)).toBeInTheDocument();
  });
});

// ---------------------------------------------------------------------------
// Save Draft
// ---------------------------------------------------------------------------

describe('EmailCompose — save draft', () => {
  it('calls create_email with the correct payload and navigates away', async () => {
    setupLoaded();
    mockInvoke.mockResolvedValueOnce(EMAIL); // create_email

    render(EmailCompose);
    await waitFor(() => expect(screen.getByDisplayValue('anna.mueller@example.com')).toBeInTheDocument());

    await fireEvent.input(screen.getByLabelText(/subject:/i), { target: { value: 'Follow-up' } });
    await fireEvent.input(screen.getByLabelText(/message:/i), { target: { value: 'Hello Anna,' } });
    await fireEvent.click(screen.getByRole('button', { name: /save draft/i }));

    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith('create_email', {
        input: {
          patient_id: 'patient-1',
          recipient_email: 'anna.mueller@example.com',
          subject: 'Follow-up',
          body: 'Hello Anna,',
        },
      });
      expect(mockGoto).toHaveBeenCalledWith('/patients/patient-1/email');
    });
  });
});

// ---------------------------------------------------------------------------
// Send (Open in Mail Client)
// ---------------------------------------------------------------------------

describe('EmailCompose — send email', () => {
  it('calls create_email then mark_email_as_sent before opening the mail client', async () => {
    setupLoaded();
    mockInvoke
      .mockResolvedValueOnce(EMAIL)  // create_email
      .mockResolvedValueOnce(EMAIL); // mark_email_as_sent

    // Stub window.location.href to avoid navigation errors in jsdom
    Object.defineProperty(window, 'location', {
      value: { href: '' },
      writable: true,
    });

    render(EmailCompose);
    await waitFor(() => expect(screen.getByDisplayValue('anna.mueller@example.com')).toBeInTheDocument());

    await fireEvent.input(screen.getByLabelText(/subject:/i), { target: { value: 'Follow-up' } });
    await fireEvent.input(screen.getByLabelText(/message:/i), { target: { value: 'Hello Anna,' } });
    await fireEvent.click(screen.getByRole('button', { name: /open in mail client/i }));

    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith('create_email', expect.any(Object));
      expect(mockInvoke).toHaveBeenCalledWith('mark_email_as_sent', { id: 'email-1' });
    });
  });
});
