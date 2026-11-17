/**
 * Where factory-reset Cancel should go. Unlock is invalid on first run, and
 * when the master keys need recovery Touch ID cannot succeed, so return to
 * the recovery screen instead of stranding the user on unlock.
 */
export function cancelResetPath(status: string): string {
  if (status === 'first_run' || status === 'initializing') return '/setup';
  if (status === 'recovery_required') return '/recover';
  return '/unlock';
}
