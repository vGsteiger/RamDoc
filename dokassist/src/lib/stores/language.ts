import { writable, derived } from 'svelte/store';
import { browser } from '$app/environment';

export type LanguageCode = 'en' | 'de';

const STORAGE_KEY = 'language-preference';

// Get initial language preference from localStorage or system language
function getInitialLanguage(): LanguageCode {
  if (!browser) return 'en';

  // Check localStorage first
  const stored = localStorage.getItem(STORAGE_KEY);
  if (stored === 'en' || stored === 'de') {
    return stored;
  }

  // Fallback to system language
  if (typeof navigator !== 'undefined' && navigator.language) {
    const systemLang = navigator.language.toLowerCase();
    if (systemLang.startsWith('de')) {
      return 'de';
    }
  }

  return 'en';
}

function createLanguageStore() {
  const { subscribe, set, update } = writable<LanguageCode>(getInitialLanguage());

  return {
    subscribe,
    set: (value: LanguageCode) => {
      if (browser) {
        localStorage.setItem(STORAGE_KEY, value);
      }
      set(value);
    },
    update,
  };
}

export const language = createLanguageStore();
