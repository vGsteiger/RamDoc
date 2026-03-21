import { derived, type Readable } from 'svelte/store';
import { language, type LanguageCode } from '$lib/stores/language';
import enTranslations from './en.json';
import deTranslations from './de.json';

type TranslationKey = string;
type Translations = typeof enTranslations;

const translations: Record<LanguageCode, Translations> = {
  en: enTranslations,
  de: deTranslations,
};

// Helper function to get nested translation value
function getNestedValue(obj: Record<string, unknown>, path: string): string {
  const keys = path.split('.');
  let current = obj;

  for (const key of keys) {
    if (current && typeof current === 'object' && key in current) {
      current = current[key];
    } else {
      return path; // Return the key if translation not found
    }
  }

  return typeof current === 'string' ? current : path;
}

// Create a derived store that provides the translation function
export const t: Readable<(key: TranslationKey) => string> = derived(
  language,
  ($language) => {
    return (key: TranslationKey) => {
      const translationSet = translations[$language];
      return getNestedValue(translationSet, key);
    };
  }
);
