import { describe, it, expect } from 'vitest';
import { stripThinkTags } from '$lib/llm/strip-think';

describe('stripThinkTags', () => {
  it('removes a completed think block', () => {
    expect(stripThinkTags('<think>internal</think>\nDear colleague,')).toBe('Dear colleague,');
  });

  it('drops an unclosed think block', () => {
    expect(stripThinkTags('<think>still reasoning')).toBe('');
  });

  it('leaves ordinary clinical text unchanged', () => {
    expect(stripThinkTags('Medikation: Sertralin 50 mg')).toBe('Medikation: Sertralin 50 mg');
  });
});
