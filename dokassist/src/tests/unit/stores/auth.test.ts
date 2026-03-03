import { describe, it, expect, beforeEach } from 'vitest';
import { get } from 'svelte/store';
import { authStatus, isLoading } from '$lib/stores/auth';

beforeEach(() => {
  // Reset stores to their initial values before every test.
  authStatus.set(null);
  isLoading.set(true);
});

describe('authStatus store', () => {
  it('initialises to null', () => {
    expect(get(authStatus)).toBe(null);
  });

  it('can be set to locked', () => {
    authStatus.set('locked');
    expect(get(authStatus)).toBe('locked');
  });

  it('can be set to unlocked', () => {
    authStatus.set('unlocked');
    expect(get(authStatus)).toBe('unlocked');
  });

  it('can be set to first_run', () => {
    authStatus.set('first_run');
    expect(get(authStatus)).toBe('first_run');
  });

  it('can be set to recovery_required', () => {
    authStatus.set('recovery_required');
    expect(get(authStatus)).toBe('recovery_required');
  });

  it('update propagates to subscribers', () => {
    const values: Array<typeof null | string> = [];
    const unsubscribe = authStatus.subscribe((v) => values.push(v));

    authStatus.set('locked');
    authStatus.set('unlocked');
    unsubscribe();

    expect(values).toEqual([null, 'locked', 'unlocked']);
  });
});

describe('isLoading store', () => {
  it('initialises to true', () => {
    expect(get(isLoading)).toBe(true);
  });

  it('can be set to false', () => {
    isLoading.set(false);
    expect(get(isLoading)).toBe(false);
  });

  it('can be toggled back to true', () => {
    isLoading.set(false);
    isLoading.set(true);
    expect(get(isLoading)).toBe(true);
  });
});
