import { describe, it, expect, beforeEach } from 'vitest';
import { get } from 'svelte/store';
import { thinkingEffort } from '$lib/stores/thinking';

describe('thinkingEffort store', () => {
  beforeEach(() => {
    localStorage.clear();
    thinkingEffort.set('medium');
  });

  it('defaults to medium', () => {
    expect(get(thinkingEffort)).toBe('medium');
  });

  it('persists extra_high to localStorage', () => {
    thinkingEffort.set('extra_high');
    expect(localStorage.getItem('thinking-effort')).toBe('extra_high');
    expect(get(thinkingEffort)).toBe('extra_high');
  });
});
