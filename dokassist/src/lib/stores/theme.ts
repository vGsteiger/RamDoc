import { writable, derived, readable } from 'svelte/store';
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

// Create a store that tracks the system's dark mode preference
const systemDarkMode = readable(false, (set) => {
  if (!browser || typeof window.matchMedia !== 'function') {
    return;
  }

  const mediaQuery = window.matchMedia('(prefers-color-scheme: dark)');

  // Set initial value
  set(mediaQuery.matches);

  // Listen for changes
  const listener = (e: MediaQueryListEvent) => set(e.matches);
  mediaQuery.addEventListener('change', listener);

  return () => mediaQuery.removeEventListener('change', listener);
});

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

// Determine the actual theme to apply based on preference and system settings.
// Note: When `preference` is 'system' and `window.matchMedia` is not available
// (e.g. during SSR, in non-browser, or certain test environments), this
// intentionally falls back to 'dark'. This provides a deterministic value in
// environments where the real system preference cannot be detected; the
// client-side browser environment will re-resolve the theme once available.
export const resolvedTheme = derived(
  [themePreference, systemDarkMode],
  ([$preference, $systemDark]) => {
    if ($preference === 'system') {
      return $systemDark ? 'dark' : 'light';
    }
    return $preference;
  }
);
