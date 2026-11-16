import { describe, it, expect } from 'vitest';
import { get } from 'svelte/store';
import { classifyUnlockError } from '$lib/unlock-errors';
import { t } from '$lib/translations';
import { language } from '$lib/stores/language';

describe('classifyUnlockError', () => {
  it('stays silent when the Touch ID sheet is dismissed', () => {
    expect(classifyUnlockError('BIOMETRIC_CANCELLED')).toEqual({ kind: 'ignore' });
  });

  it('routes missing Keychain keys to recovery', () => {
    expect(classifyUnlockError('RECOVERY_REQUIRED')).toEqual({ kind: 'recovery_required' });
  });

  it('asks for confirmation on an audit checkpoint mismatch', () => {
    expect(classifyUnlockError('AUDIT_CHECKPOINT_MISMATCH')).toEqual({
      kind: 'checkpoint_mismatch',
    });
  });

  it.each(['KEYCHAIN_ACCESS_DENIED', 'KEYCHAIN_ERROR', 'DB_KEY_MISMATCH', 'DATABASE_ERROR'])(
    'offers the recovery phrase for %s, which retrying Touch ID cannot fix',
    (code) => {
      const outcome = classifyUnlockError(code);
      expect(outcome.kind).toBe('error');
      expect(outcome.kind === 'error' && outcome.suggestRecovery).toBe(true);
    }
  );

  it('does not offer recovery for a broken audit chain, which recovery cannot accept', () => {
    const outcome = classifyUnlockError('AUDIT_INTEGRITY_ERROR');
    expect(outcome).toMatchObject({ kind: 'error', suggestRecovery: false, showDetails: true });
  });

  it('falls back to the backend message for unknown codes', () => {
    expect(classifyUnlockError('SOMETHING_NEW')).toMatchObject({ kind: 'error', messageKey: null });
  });

  it.each(['en', 'de'] as const)('has a %s translation for every message key', (lang) => {
    language.set(lang);
    const codes = [
      'KEYCHAIN_ACCESS_DENIED',
      'KEYCHAIN_ERROR',
      'DB_KEY_MISMATCH',
      'DATABASE_ERROR',
      'AUDIT_INTEGRITY_ERROR',
      'FILESYSTEM_ERROR',
      'AUTH_REQUIRED',
    ];
    for (const code of codes) {
      const outcome = classifyUnlockError(code);
      if (outcome.kind !== 'error' || !outcome.messageKey) throw new Error(code);
      expect(get(t)(outcome.messageKey)).not.toBe(outcome.messageKey);
    }
    language.set('en');
  });
});
