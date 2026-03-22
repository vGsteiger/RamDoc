import { describe, it, expect } from 'vitest';
import { render, screen } from '@testing-library/svelte';
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

  it('renders Thinking section when assistant content starts with <think>', () => {
    const content = '<think>Let me reason...</think>Here is the answer.';
    render(ChatMessage, { props: { message: makeMsg({ role: 'assistant', content }) } });
    expect(screen.getByText('Thinking')).toBeInTheDocument();
    expect(screen.getByText('Let me reason...')).toBeInTheDocument();
  });

  it('strips <think> block from main assistant content', () => {
    const content = '<think>Reasoning here</think>Final answer.';
    render(ChatMessage, { props: { message: makeMsg({ role: 'assistant', content }) } });
    // Main bubble shows the text after </think>
    expect(screen.getByText('Final answer.')).toBeInTheDocument();
    // The think content is in the Thinking section (not in the main bubble)
    expect(screen.getByText('Thinking')).toBeInTheDocument();
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

  it('renders Ergebnis button for tool_result role', () => {
    render(ChatMessage, {
      props: { message: makeMsg({ role: 'tool_result', content: '{"name":"Anna"}' }) },
    });
    expect(screen.getByRole('button', { name: /Ergebnis/ })).toBeInTheDocument();
  });

  it('renders pulse indicator when streaming with empty content', () => {
    render(ChatMessage, {
      props: {
        message: makeMsg({ role: 'assistant', content: '' }),
        isStreaming: true,
      },
    });
    expect(screen.getByText('●')).toBeInTheDocument();
  });
});
