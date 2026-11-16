/**
 * How the unlock screen should react to an error code from `unlock_app` or
 * `unlock_app_accepting_audit_checkpoint`.
 *
 * Every failure that retrying Touch ID cannot fix must point the user to the
 * way out that does work, rather than showing an opaque message that leaves
 * them stuck on the unlock screen.
 */
export type UnlockErrorOutcome =
  /** The user dismissed the Touch ID sheet; stay on screen silently. */
  | { kind: 'ignore' }
  /** The master keys are gone from the Keychain; the backend moved to recovery. */
  | { kind: 'recovery_required' }
  /** The audit chain is valid but its Keychain checkpoint is not; ask to confirm. */
  | { kind: 'checkpoint_mismatch' }
  | {
      kind: 'error';
      /** Translation key, or `null` to show the backend message. */
      messageKey: string | null;
      /** Show the recovery phrase as the next step. */
      suggestRecovery: boolean;
      /** Show the backend message as technical detail under the summary. */
      showDetails: boolean;
    };

export function classifyUnlockError(code: string): UnlockErrorOutcome {
  const error = (messageKey: string | null, suggestRecovery = false, showDetails = false) =>
    ({ kind: 'error', messageKey, suggestRecovery, showDetails }) as const;

  switch (code) {
    case 'BIOMETRIC_CANCELLED':
      return { kind: 'ignore' };
    case 'RECOVERY_REQUIRED':
      return { kind: 'recovery_required' };
    case 'AUDIT_CHECKPOINT_MISMATCH':
      return { kind: 'checkpoint_mismatch' };
    case 'KEYCHAIN_ACCESS_DENIED':
      return error('auth.keychainAccessDenied', true);
    case 'KEYCHAIN_ERROR':
      return error('auth.keychainAccessError', true, true);
    case 'DB_KEY_MISMATCH':
      return error('auth.databaseKeyMismatch', true);
    case 'DATABASE_ERROR':
      return error('auth.databaseAccessError', true, true);
    case 'AUDIT_INTEGRITY_ERROR':
      return error('auth.auditIntegrityError', false, true);
    case 'FILESYSTEM_ERROR':
      return error('auth.filesystemAccessError', false, true);
    case 'AUTH_REQUIRED':
      return error('auth.unlockFailed');
    default:
      return error(null);
  }
}
