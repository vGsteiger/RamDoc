import { writable } from 'svelte/store';

export type AuthStatus = 'first_run' | 'locked' | 'unlocked' | 'recovery_required';

export const authStatus = writable<AuthStatus | null>(null);
export const isLoading = writable<boolean>(true);
