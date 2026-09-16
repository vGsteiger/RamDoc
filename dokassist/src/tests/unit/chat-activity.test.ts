import { describe, it, expect } from 'vitest';
import {
  chatActivityLabel,
  chatActivityStage,
  chatToolLabel,
  formatElapsed,
} from '$lib/chat-activity';

const t = (key: string) =>
  (
    ({
      'chat.activity.thinking': 'Thinking',
      'chat.activity.stillThinking': 'Still thinking',
      'chat.activity.reasoning': 'Reasoning',
      'chat.activity.writing': 'Writing',
      'chat.activity.looking_up': 'Looking things up',
      'chat.activity.lookedUp': 'Looked up {tool}',
      'chat.toolNames.list_medications': 'medications',
    }) as Record<string, string>
  )[key] ?? key;

describe('chatActivityStage', () => {
  it('is thinking when there is no content yet', () => {
    expect(chatActivityStage('')).toBe('thinking');
  });

  it('is reasoning while a think block is still open', () => {
    expect(chatActivityStage('<think>drafting')).toBe('reasoning');
  });

  it('is reasoning after think closes with no answer yet', () => {
    expect(chatActivityStage('<think>drafting</think>')).toBe('reasoning');
    expect(chatActivityStage('<think>drafting</think>  ')).toBe('reasoning');
  });

  it('is writing once answer tokens exist', () => {
    expect(chatActivityStage('Hello')).toBe('writing');
    expect(chatActivityStage('<think>drafting</think>Hello')).toBe('writing');
  });

  it('is looking_up when a tool name is active, even with empty content', () => {
    expect(chatActivityStage('', 'list_medications')).toBe('looking_up');
  });
});

describe('chatActivityLabel', () => {
  it('uses Thinking, then Still thinking after 8 seconds', () => {
    expect(chatActivityLabel('thinking', 1, t)).toBe('Thinking');
    expect(chatActivityLabel('thinking', 8, t)).toBe('Still thinking');
  });

  it('names the tool that just ran', () => {
    expect(chatActivityLabel('looking_up', 1, t, 'list_medications')).toBe('Looked up medications');
  });

  it('falls back to a generic looking-up label without a tool name', () => {
    expect(chatActivityLabel('looking_up', 1, t)).toBe('Looking things up');
  });
});

describe('chatToolLabel', () => {
  it('uses the translation when present and otherwise humanises the name', () => {
    expect(chatToolLabel('list_medications', t)).toBe('medications');
    expect(chatToolLabel('mystery_tool', t)).toBe('mystery tool');
  });
});

describe('formatElapsed', () => {
  it('formats minutes and zero-padded seconds', () => {
    expect(formatElapsed(0)).toBe('0:00');
    expect(formatElapsed(2)).toBe('0:02');
    expect(formatElapsed(75)).toBe('1:15');
  });
});
