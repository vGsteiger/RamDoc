import { writable } from 'svelte/store';
import { browser } from '$app/environment';
import type { ThinkingEffort } from '$lib/api';

const STORAGE_KEY = 'thinking-effort';
const LEVELS: ThinkingEffort[] = ['low', 'medium', 'high', 'extra_high'];

export function getStoredThinkingEffort(): ThinkingEffort | null {
  if (!browser) return null;
  const stored = localStorage.getItem(STORAGE_KEY);
  return stored && LEVELS.includes(stored as ThinkingEffort) ? (stored as ThinkingEffort) : null;
}

function getInitialEffort(): ThinkingEffort {
  return getStoredThinkingEffort() ?? 'medium';
}

function createThinkingEffortStore() {
  const { subscribe, set, update } = writable<ThinkingEffort>(getInitialEffort());

  return {
    subscribe,
    // Applies a task-specific default without turning it into a user preference.
    setTransient: set,
    set: (value: ThinkingEffort) => {
      if (browser) {
        localStorage.setItem(STORAGE_KEY, value);
      }
      set(value);
    },
    update: (updater: (value: ThinkingEffort) => ThinkingEffort) => {
      update((current) => {
        const next = updater(current);
        if (browser) {
          localStorage.setItem(STORAGE_KEY, next);
        }
        return next;
      });
    },
  };
}

export const thinkingEffort = createThinkingEffortStore();
