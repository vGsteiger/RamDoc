import { writable } from 'svelte/store';

export interface Toast {
  id: string;
  message: string;
  type: 'success' | 'error';
}

const { subscribe, update } = writable<Toast[]>([]);

export const toasts = { subscribe };

export function addToast(message: string, type: 'success' | 'error' = 'success') {
  const id = crypto.randomUUID();
  update((t) => [...t, { id, message, type }]);
  setTimeout(() => removeToast(id), 3000);
}

export function removeToast(id: string) {
  update((t) => t.filter((toast) => toast.id !== id));
}
