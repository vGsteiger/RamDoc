import { describe, it, expect, beforeEach, vi } from 'vitest';
import { get } from 'svelte/store';

describe('thinkingEffort store', () => {
  beforeEach(() => {
    localStorage.clear();
    vi.resetModules();
  });

  it('defaults to medium when nothing is stored', async () => {
    const { thinkingEffort, getStoredThinkingEffort } = await import('$lib/stores/thinking');
    expect(get(thinkingEffort)).toBe('medium');
    expect(getStoredThinkingEffort()).toBeNull();
  });

  it('persists extra_high to localStorage', async () => {
    const { thinkingEffort, getStoredThinkingEffort } = await import('$lib/stores/thinking');
    thinkingEffort.set('extra_high');
    expect(localStorage.getItem('thinking-effort')).toBe('extra_high');
    expect(get(thinkingEffort)).toBe('extra_high');
    expect(getStoredThinkingEffort()).toBe('extra_high');
  });

  it('persists values written through update', async () => {
    const { thinkingEffort } = await import('$lib/stores/thinking');
    thinkingEffort.update(() => 'high');
    expect(localStorage.getItem('thinking-effort')).toBe('high');
    expect(get(thinkingEffort)).toBe('high');
  });

  it('supports a task default without persisting it as a user preference', async () => {
    const { thinkingEffort, getStoredThinkingEffort } = await import('$lib/stores/thinking');
    thinkingEffort.setTransient('low');

    expect(get(thinkingEffort)).toBe('low');
    expect(getStoredThinkingEffort()).toBeNull();
  });
});
