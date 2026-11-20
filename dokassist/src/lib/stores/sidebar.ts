import { writable } from 'svelte/store';
import { browser } from '$app/env';

/**
 * Whether the user collapsed the sidebar to its icon rail (D2: shown by
 * default, hideable). Persisted across launches. Below 1024px the sidebar is
 * always a rail regardless; this only governs wider windows.
 */
const STORAGE_KEY = 'sidebar-collapsed';

function initial(): boolean {
  if (!browser) return false;
  try {
    return localStorage.getItem(STORAGE_KEY) === 'true';
  } catch {
    return false;
  }
}

const store = writable<boolean>(initial());

store.subscribe((value) => {
  if (!browser) return;
  try {
    localStorage.setItem(STORAGE_KEY, String(value));
  } catch {
    // Storage unavailable: the preference just doesn't persist.
  }
});

export const sidebarCollapsed = store;
