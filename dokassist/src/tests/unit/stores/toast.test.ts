import { describe, it, expect, vi, beforeEach } from 'vitest';
import { get } from 'svelte/store';
import { toasts, addToast, removeToast } from '$lib/stores/toast';

beforeEach(() => {
  // Clear toasts between tests by removing all
  get(toasts).forEach((t) => removeToast(t.id));
});

describe('toast store', () => {
  it('starts empty', () => {
    expect(get(toasts)).toHaveLength(0);
  });

  it('addToast adds a success toast', () => {
    vi.useFakeTimers();
    addToast('Saved successfully');
    const list = get(toasts);
    expect(list).toHaveLength(1);
    expect(list[0].message).toBe('Saved successfully');
    expect(list[0].type).toBe('success');
    vi.useRealTimers();
  });

  it('addToast adds an error toast', () => {
    vi.useFakeTimers();
    addToast('Something went wrong', 'error');
    const list = get(toasts);
    expect(list[0].type).toBe('error');
    vi.useRealTimers();
  });

  it('removeToast removes a toast by id', () => {
    vi.useFakeTimers();
    addToast('Test');
    const id = get(toasts)[0].id;
    removeToast(id);
    expect(get(toasts)).toHaveLength(0);
    vi.useRealTimers();
  });

  it('auto-removes toast after 3 seconds', () => {
    vi.useFakeTimers();
    addToast('Auto-dismiss');
    expect(get(toasts)).toHaveLength(1);
    vi.advanceTimersByTime(3000);
    expect(get(toasts)).toHaveLength(0);
    vi.useRealTimers();
  });
});
