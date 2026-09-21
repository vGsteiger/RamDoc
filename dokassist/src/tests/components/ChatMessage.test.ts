import { describe, it, expect, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/svelte';
import { invoke } from '@tauri-apps/api/core';
import ChatMessage from '../../lib/components/ChatMessage.svelte';
import type { ChatMessageRow } from '$lib/api';

function makeMsg(overrides: Partial<ChatMessageRow> = {}): ChatMessageRow {
  return {
    id: 'm1',
    session_id: 's1',
    role: 'user',
    content: 'Hello',
    tool_name: null,
    tool_args_json: null,
    tool_result_for: null,
    created_at: '2026-01-01T00:00:00Z',
    ...overrides,
  };
}

describe('ChatMessage', () => {
  it('renders user message content', () => {
    render(ChatMessage, { props: { message: makeMsg({ content: 'Hello there' }) } });
    expect(screen.getByText('Hello there')).toBeInTheDocument();
  });

  it('renders assistant message content', () => {
    render(ChatMessage, {
      props: { message: makeMsg({ role: 'assistant', content: 'I can help with that.' }) },
    });
    expect(screen.getByText('I can help with that.')).toBeInTheDocument();
  });

  it('renders reasoning section when assistant content starts with <think>', () => {
    const content = '<think>Let me reason...</think>Here is the answer.';
    render(ChatMessage, { props: { message: makeMsg({ role: 'assistant', content }) } });
    expect(screen.getByText('Reasoning')).toBeInTheDocument();
    expect(screen.getByText('Let me reason...')).toBeInTheDocument();
  });

  it('strips <think> block from main assistant content', () => {
    const content = '<think>Reasoning here</think>Final answer.';
    render(ChatMessage, { props: { message: makeMsg({ role: 'assistant', content }) } });
    // Main bubble shows the text after </think>
    expect(screen.getByText('Final answer.')).toBeInTheDocument();
    // The think content is in the reasoning section (not in the main bubble)
    expect(screen.getByText('Reasoning')).toBeInTheDocument();
  });

  it('renders tool name for tool_call role', () => {
    render(ChatMessage, {
      props: {
        message: makeMsg({
          role: 'tool_call',
          content: '{}',
          tool_name: 'get_patient',
          tool_args_json: '{"id": "p1"}',
        }),
      },
    });
    expect(screen.getByText(/Tool: get_patient/)).toBeInTheDocument();
  });

  it('renders the result disclosure for tool_result role', () => {
    render(ChatMessage, {
      props: { message: makeMsg({ role: 'tool_result', content: '{"name":"Anna"}' }) },
    });
    expect(screen.getByRole('button', { name: /Show result/ })).toBeInTheDocument();
  });

  it('renders an unsaved report proposal with an explicit review gate', () => {
    render(ChatMessage, {
      props: {
        message: makeMsg({
          role: 'tool_result',
          content: JSON.stringify({
            status: 'pending_clinician_confirmation',
            action: 'create_report',
            proposal: {
              patient_id: 'p1',
              report_type: 'Befundbericht',
              content: 'Reviewed report content',
              model_name: null,
              prompt_hash: null,
              session_ids: null,
            },
          }),
        }),
      },
    });

    expect(screen.getByText('Report draft — not saved')).toBeInTheDocument();
    expect(screen.getByRole('checkbox')).not.toBeChecked();
    expect(screen.getByRole('button', { name: /Save reviewed report draft/i })).toBeDisabled();
  });

  it('blocks a draft with an unverified citation until the clinician resolves it', async () => {
    vi.mocked(invoke).mockResolvedValue([]);
    render(ChatMessage, {
      props: {
        message: makeMsg({
          role: 'tool_result',
          tool_name: 'write_report',
          content: JSON.stringify({
            status: 'pending_clinician_confirmation',
            action: 'create_report',
            proposal: {
              patient_id: 'p1',
              report_type: 'Befundbericht',
              content: 'Claim [E404]',
              model_name: null,
              prompt_hash: null,
              session_ids: null,
            },
          }),
        }),
      },
    });

    await fireEvent.click(screen.getByRole('checkbox'));
    expect(screen.getByText('Resolve unsupported claims before saving')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /Save reviewed report draft/i })).toBeDisabled();
    expect(screen.getAllByRole('combobox')).toHaveLength(2);
  });

  it('shows only typed tool-result sources, not citation-shaped assistant prose', () => {
    render(ChatMessage, {
      props: {
        message: makeMsg({ role: 'assistant', content: 'Answer [E5]' }),
        provenance: {
          status: 'inferred',
          sources: [{ toolName: 'list_medications', status: 'supported' }],
          unverifiedCitations: ['[E5]'],
        },
      },
    });
    expect(screen.getByText('Sources used')).toBeInTheDocument();
    expect(screen.getByText('Medications')).toBeInTheDocument();
    expect(
      screen.getByText(/Citation-like text is not verified evidence here: \[E5\]/)
    ).toBeInTheDocument();
  });

  it('renders a thinking status when streaming with empty content', () => {
    render(ChatMessage, {
      props: {
        message: makeMsg({ role: 'assistant', content: '' }),
        isStreaming: true,
      },
    });
    expect(screen.getByRole('status')).toHaveTextContent('Thinking');
    expect(screen.queryByText('●')).not.toBeInTheDocument();
  });

  it('stays on thinking when the first streamed chunk is only whitespace', () => {
    render(ChatMessage, {
      props: {
        message: makeMsg({ role: 'assistant', content: '\n' }),
        isStreaming: true,
      },
    });
    expect(screen.getByRole('status')).toHaveTextContent('Thinking');
  });

  it('names the tool that just ran while still streaming', () => {
    render(ChatMessage, {
      props: {
        message: makeMsg({ role: 'assistant', content: '' }),
        isStreaming: true,
        activeToolName: 'list_medications',
      },
    });
    expect(screen.getByRole('status')).toHaveTextContent('Looked up medications');
  });

  it('names a completed write tool as prepared, not looked up', () => {
    render(ChatMessage, {
      props: {
        message: makeMsg({ role: 'assistant', content: '' }),
        isStreaming: true,
        activeToolName: 'write_report',
      },
    });
    expect(screen.getByRole('status')).toHaveTextContent('Prepared the report');
  });

  it('uses Reasoning while a think block is still streaming', () => {
    render(ChatMessage, {
      props: {
        message: makeMsg({ role: 'assistant', content: '<think>drafting' }),
        isStreaming: true,
      },
    });
    expect(screen.getByRole('status')).toHaveTextContent('Reasoning');
  });

  it('hides the thinking status once answer tokens stream in', () => {
    render(ChatMessage, {
      props: {
        message: makeMsg({ role: 'assistant', content: 'Here is the answer.' }),
        isStreaming: true,
      },
    });
    expect(screen.queryByRole('status')).not.toBeInTheDocument();
    expect(screen.getByText('Here is the answer.')).toBeInTheDocument();
  });
});
