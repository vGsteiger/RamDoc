import { describe, it, expect } from 'vitest';
import {
  chatActivityLabel,
  chatActivityStage,
  chatToolActivityLabel,
  chatToolLabel,
  formatElapsed,
  isWriteTool,
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
      'chat.activity.lookingUp': 'Looking up {tool}',
      'chat.activity.prepared': 'Prepared {tool}',
      'chat.activity.preparing': 'Preparing {tool}',
      'chat.toolNames.list_medications': 'medications',
      'chat.toolNames.write_report': 'the report',
      'chat.toolNames.draft_email': 'an email draft',
      'chat.toolNames.create_diagnosis': 'diagnoses',
    }) as Record<string, string>
  )[key] ?? key;

describe('chatActivityStage', () => {
  it('is thinking when there is no content yet', () => {
    expect(chatActivityStage('')).toBe('thinking');
  });

  it('treats whitespace-only content as still thinking', () => {
    expect(chatActivityStage('\n')).toBe('thinking');
    expect(chatActivityStage('  \n\t')).toBe('thinking');
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

  it('uses a prepared label for write and proposal tools', () => {
    expect(chatActivityLabel('looking_up', 1, t, 'write_report')).toBe('Prepared the report');
    expect(chatActivityLabel('looking_up', 1, t, 'draft_email')).toBe('Prepared an email draft');
    expect(chatActivityLabel('looking_up', 1, t, 'create_diagnosis')).toBe('Prepared diagnoses');
  });

  it('falls back to a generic looking-up label without a tool name', () => {
    expect(chatActivityLabel('looking_up', 1, t)).toBe('Looking things up');
  });
});

describe('chatToolActivityLabel', () => {
  it('uses looking-up copy while a read tool is in progress', () => {
    expect(chatToolActivityLabel('list_medications', t, true)).toBe('Looking up medications');
  });

  it('uses preparing copy while a write tool is in progress', () => {
    expect(chatToolActivityLabel('write_report', t, true)).toBe('Preparing the report');
  });
});

describe('isWriteTool', () => {
  it('treats create_*, draft_email, and write_report as writes', () => {
    expect(isWriteTool('create_medication')).toBe(true);
    expect(isWriteTool('draft_email')).toBe(true);
    expect(isWriteTool('write_report')).toBe(true);
    expect(isWriteTool('list_medications')).toBe(false);
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
