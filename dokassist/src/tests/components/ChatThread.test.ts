import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/svelte';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import ChatThread from '../../lib/components/ChatThread.svelte';
import type { ChatMessageRow } from '$lib/api';
import { resetEngineState } from '$lib/stores/engine';

vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn(),
}));

vi.mock('$app/navigation', () => ({
  goto: vi.fn(),
}));

const mockInvoke = vi.mocked(invoke);
const mockListen = vi.mocked(listen);

const ENGINE_LOADED = {
  is_loaded: true,
  model_name: 'Phi-4 Mini',
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

const USER_MSG: ChatMessageRow = {
  id: 'm1',
  session_id: 'sess1',
  role: 'user',
  content: 'Hello agent',
  tool_name: null,
  tool_args_json: null,
  tool_result_for: null,
  created_at: '2026-01-01T00:00:00Z',
};

const ASSISTANT_MSG: ChatMessageRow = {
  id: 'm2',
  session_id: 'sess1',
  role: 'assistant',
  content: 'I can help you with that.',
  tool_name: null,
  tool_args_json: null,
  tool_result_for: null,
  created_at: '2026-01-01T00:00:01Z',
};

beforeEach(() => {
  mockInvoke.mockReset();
  resetEngineState();
  // listen returns an unlisten function — return a resolved promise so onMount completes
  mockListen.mockResolvedValue(vi.fn());
  // jsdom does not implement scrollIntoView
  window.HTMLElement.prototype.scrollIntoView = vi.fn();
});

describe('ChatThread', () => {
  it('shows model-not-loaded banner when engine is not loaded', async () => {
    // First call: get_chat_messages, Second call: get_engine_status
    mockInvoke.mockResolvedValueOnce([]).mockResolvedValueOnce(ENGINE_NOT_LOADED);
    render(ChatThread, { props: { sessionId: 'sess1', scope: 'global' } });
    await waitFor(() => expect(screen.getByText(/No model loaded/i)).toBeInTheDocument());
  });

  it('textarea is disabled when model is not loaded', async () => {
    mockInvoke.mockResolvedValueOnce([]).mockResolvedValueOnce(ENGINE_NOT_LOADED);
    render(ChatThread, { props: { sessionId: 'sess1', scope: 'global' } });
    await waitFor(() => {
      const textarea = screen.getByRole('textbox');
      expect(textarea).toBeDisabled();
    });
  });

  it('send button is disabled when model is not loaded', async () => {
    mockInvoke.mockResolvedValueOnce([]).mockResolvedValueOnce(ENGINE_NOT_LOADED);
    render(ChatThread, { props: { sessionId: 'sess1', scope: 'global' } });
    await waitFor(() => {
      const btn = screen.getByRole('button', { name: /Send/i });
      expect(btn).toBeDisabled();
    });
  });

  it('renders empty state when no messages exist', async () => {
    mockInvoke.mockResolvedValueOnce([]).mockResolvedValueOnce(ENGINE_LOADED);
    render(ChatThread, { props: { sessionId: 'sess1', scope: 'global' } });
    await waitFor(() => {
      // No message content should be rendered — just the input area
      expect(screen.queryByText('Hello agent')).not.toBeInTheDocument();
    });
  });

  it('renders existing messages loaded on mount', async () => {
    mockInvoke
      .mockResolvedValueOnce([USER_MSG, ASSISTANT_MSG])
      .mockResolvedValueOnce(ENGINE_LOADED);
    render(ChatThread, { props: { sessionId: 'sess1', scope: 'global' } });
    await waitFor(() => {
      expect(screen.getByText('Hello agent')).toBeInTheDocument();
      expect(screen.getByText('I can help you with that.')).toBeInTheDocument();
    });
  });

  it('submitting a message calls run_agent_turn', async () => {
    mockInvoke
      .mockResolvedValueOnce([])
      .mockResolvedValueOnce(ENGINE_LOADED)
      .mockResolvedValueOnce({ session_id: 'sess1', final_answer: '', tool_calls_made: [] });
    render(ChatThread, { props: { sessionId: 'sess1', scope: 'global' } });
    await waitFor(() => expect(screen.getByRole('textbox')).not.toBeDisabled());

    const textarea = screen.getByRole<HTMLTextAreaElement>('textbox');
    // Set value directly on the element and fire input so Svelte's bind:value picks it up
    textarea.value = 'Test message';
    await fireEvent.input(textarea);
    await fireEvent.click(screen.getByRole('button', { name: /Send/i }));

    await waitFor(() =>
      expect(mockInvoke).toHaveBeenCalledWith('run_agent_turn', {
        sessionId: 'sess1',
        userMessage: 'Test message',
        thinkingEffort: 'medium',
      })
    );
  });

  it('optimistic user message appears immediately after submit', async () => {
    mockInvoke
      .mockResolvedValueOnce([])
      .mockResolvedValueOnce(ENGINE_LOADED)
      .mockResolvedValueOnce({ session_id: 'sess1', final_answer: '', tool_calls_made: [] });
    render(ChatThread, { props: { sessionId: 'sess1', scope: 'global' } });
    await waitFor(() => expect(screen.getByRole('textbox')).not.toBeDisabled());

    const textarea = screen.getByRole<HTMLTextAreaElement>('textbox');
    textarea.value = 'Optimistic message';
    await fireEvent.input(textarea);
    await fireEvent.click(screen.getByRole('button', { name: /Send/i }));

    // The optimistic message should appear in the DOM right away
    await waitFor(() => expect(screen.getByText('Optimistic message')).toBeInTheDocument());
  });

  it('shows a thinking status immediately after submit, before tokens arrive', async () => {
    mockInvoke
      .mockResolvedValueOnce([])
      .mockResolvedValueOnce(ENGINE_LOADED)
      .mockResolvedValueOnce({ session_id: 'sess1', final_answer: '', tool_calls_made: [] });
    render(ChatThread, { props: { sessionId: 'sess1', scope: 'global' } });
    await waitFor(() => expect(screen.getByRole('textbox')).not.toBeDisabled());

    const textarea = screen.getByRole<HTMLTextAreaElement>('textbox');
    textarea.value = 'What medications is she on?';
    await fireEvent.input(textarea);
    await fireEvent.click(screen.getByRole('button', { name: /Send/i }));

    await waitFor(() => expect(screen.getByRole('status')).toHaveTextContent('Thinking'));
  });

  it('shows a live loading banner while the model is being loaded into memory', async () => {
    const { setEngineState } = await import('$lib/stores/engine');
    mockInvoke.mockResolvedValueOnce([]).mockResolvedValueOnce(ENGINE_NOT_LOADED);
    setEngineState({
      status: ENGINE_NOT_LOADED,
      isLoading: true,
      loadingFilename: 'phi4.gguf',
      loadingStartedAt: Date.now(),
      error: null,
    });
    render(ChatThread, { props: { sessionId: 'sess1', scope: 'global' } });
    await waitFor(() =>
      expect(screen.getByRole('status')).toHaveTextContent(/Loading phi4 into memory/i)
    );
    expect(screen.queryByText(/No model loaded/i)).not.toBeInTheDocument();
  });

  it('shows an in-progress tool card when the agent starts a tool', async () => {
    const handlers: Record<string, (e: { payload: unknown }) => void> = {};
    mockListen.mockImplementation((event, handler) => {
      handlers[event] = handler as (e: { payload: unknown }) => void;
      return Promise.resolve(() => {});
    });
    mockInvoke
      .mockResolvedValueOnce([])
      .mockResolvedValueOnce(ENGINE_LOADED)
      .mockReturnValueOnce(new Promise(() => {}));
    render(ChatThread, { props: { sessionId: 'sess1', scope: 'global' } });
    await waitFor(() => expect(screen.getByRole('textbox')).not.toBeDisabled());

    const textarea = screen.getByRole<HTMLTextAreaElement>('textbox');
    textarea.value = 'What medications is she on?';
    await fireEvent.input(textarea);
    await fireEvent.click(screen.getByRole('button', { name: /Send/i }));

    await waitFor(() => expect(handlers['agent-tool-started']).toBeDefined());
    handlers['agent-tool-started']({
      payload: { name: 'list_medications', args_json: '{}' },
    });

    await waitFor(() =>
      expect(screen.getByRole('status')).toHaveTextContent(/Looking up medications/i)
    );
  });
});
