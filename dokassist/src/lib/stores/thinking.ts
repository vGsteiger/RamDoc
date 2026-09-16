import { writable } from 'svelte/store';
import { browser } from '$app/environment';
import type { ThinkingEffort } from '$lib/api';

const STORAGE_KEY = 'thinking-effort';
const LEVELS: ThinkingEffort[] = ['low', 'medium', 'high', 'extra_high'];

function getInitialEffort(): ThinkingEffort {
  if (!browser) return 'medium';
  const stored = localStorage.getItem(STORAGE_KEY);
  if (stored && LEVELS.includes(stored as ThinkingEffort)) {
    return stored as ThinkingEffort;
  }
  return 'medium';
}

function createThinkingEffortStore() {
  const { subscribe, set, update } = writable<ThinkingEffort>(getInitialEffort());

  return {
    subscribe,
    set: (value: ThinkingEffort) => {
      if (browser) {
        localStorage.setItem(STORAGE_KEY, value);
      }
      set(value);
    },
    update,
  };
}

export const thinkingEffort = createThinkingEffortStore();
