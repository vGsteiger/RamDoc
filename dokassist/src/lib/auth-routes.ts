/** Where factory-reset Cancel should go. Unlock is invalid on first run. */
export function cancelResetPath(status: string): string {
  if (status === 'first_run' || status === 'initializing') return '/setup';
  return '/unlock';
}
