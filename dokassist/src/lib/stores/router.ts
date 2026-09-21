import { get, writable } from 'svelte/store';
import {
  getRouterDiagnostics,
  parseError,
  setRouterModelOverride,
  unloadRouterModel,
  type RouterDiagnostics,
} from '$lib/api';

export interface RouterState {
  diagnostics: RouterDiagnostics | null;
  isMutating: boolean;
  error: string | null;
}

const initial: RouterState = { diagnostics: null, isMutating: false, error: null };
const store = writable<RouterState>(initial);

export const router = { subscribe: store.subscribe };

/** Test helper — not used by views. */
export function resetRouterState() {
  store.set(initial);
}

export async function refreshRouterDiagnostics(): Promise<RouterDiagnostics | null> {
  try {
    const diagnostics = await getRouterDiagnostics();
    store.update((state) => ({ ...state, diagnostics, error: null }));
    return diagnostics;
  } catch (error) {
    store.update((state) => ({ ...state, error: parseError(error).message }));
    return get(store).diagnostics;
  }
}

export async function setRouterOverride(modelId: string | null): Promise<void> {
  store.update((state) => ({ ...state, isMutating: true, error: null }));
  try {
    await setRouterModelOverride(modelId);
    await refreshRouterDiagnostics();
  } catch (error) {
    store.update((state) => ({ ...state, error: parseError(error).message }));
    throw error;
  } finally {
    store.update((state) => ({ ...state, isMutating: false }));
  }
}

export async function unloadRouter(): Promise<void> {
  store.update((state) => ({ ...state, isMutating: true, error: null }));
  try {
    await unloadRouterModel();
    await refreshRouterDiagnostics();
  } catch (error) {
    store.update((state) => ({ ...state, error: parseError(error).message }));
    throw error;
  } finally {
    store.update((state) => ({ ...state, isMutating: false }));
  }
}
