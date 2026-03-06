import { writable } from 'svelte/store';
import { browser } from '$app/environment';

export type ThemeMode = 'light' | 'dark' | 'system';

const STORAGE_KEY = 'theme-preference';

// Get initial theme preference from localStorage or default to 'system'
function getInitialTheme(): ThemeMode {
  if (!browser) return 'system';

  const stored = localStorage.getItem(STORAGE_KEY);
  if (stored === 'light' || stored === 'dark' || stored === 'system') {
    return stored;
  }
  return 'system';
}

// Determine the actual theme to apply based on preference and system settings
export function resolveTheme(preference: ThemeMode): 'light' | 'dark' {
  if (preference === 'system') {
    if (!browser || typeof window.matchMedia !== 'function') return 'dark';
    return window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light';
  }
  return preference;
}

function createThemeStore() {
  const { subscribe, set, update } = writable<ThemeMode>(getInitialTheme());

  return {
    subscribe,
    set: (value: ThemeMode) => {
      if (browser) {
        localStorage.setItem(STORAGE_KEY, value);
      }
      set(value);
    },
    update,
  };
}

export const themePreference = createThemeStore();

// Watch for system theme changes when in 'system' mode
if (browser && typeof window.matchMedia === 'function') {
  const mediaQuery = window.matchMedia('(prefers-color-scheme: dark)');
  mediaQuery.addEventListener('change', () => {
    // This will trigger reactivity in components subscribed to theme changes
    themePreference.update(v => v);
  });
}
