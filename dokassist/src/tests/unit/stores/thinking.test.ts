import { describe, it, expect, beforeEach, vi } from 'vitest';
import { get } from 'svelte/store';

describe('thinkingEffort store', () => {
  beforeEach(() => {
    localStorage.clear();
    vi.resetModules();
  });

  it('defaults to medium when nothing is stored', async () => {
    const { thinkingEffort } = await import('$lib/stores/thinking');
    expect(get(thinkingEffort)).toBe('medium');
  });

  it('persists extra_high to localStorage', async () => {
    const { thinkingEffort } = await import('$lib/stores/thinking');
    thinkingEffort.set('extra_high');
    expect(localStorage.getItem('thinking-effort')).toBe('extra_high');
    expect(get(thinkingEffort)).toBe('extra_high');
  });

  it('persists values written through update', async () => {
    const { thinkingEffort } = await import('$lib/stores/thinking');
    thinkingEffort.update(() => 'high');
    expect(localStorage.getItem('thinking-effort')).toBe('high');
    expect(get(thinkingEffort)).toBe('high');
  });
});
