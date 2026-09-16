import { describe, it, expect } from 'vitest';
import { cancelResetPath } from '$lib/auth-routes';

describe('cancelResetPath', () => {
  it('returns setup while the app is still in first-run', () => {
    expect(cancelResetPath('first_run')).toBe('/setup');
    expect(cancelResetPath('initializing')).toBe('/setup');
  });

  it('returns unlock once a vault exists', () => {
    expect(cancelResetPath('locked')).toBe('/unlock');
    expect(cancelResetPath('unlocked')).toBe('/unlock');
    expect(cancelResetPath('recovery_required')).toBe('/unlock');
  });
});
