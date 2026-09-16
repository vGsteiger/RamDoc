import { get, writable } from 'svelte/store';
import {
  getEngineStatus,
  loadModel,
  parseError,
  type InferenceProfile,
  type LlmEngineStatus,
} from '$lib/api';

export interface EngineState {
  status: LlmEngineStatus | null;
  isLoading: boolean;
  loadingFilename: string | null;
  loadingStartedAt: number | null;
  error: string | null;
}

const initial: EngineState = {
  status: null,
  isLoading: false,
  loadingFilename: null,
  loadingStartedAt: null,
  error: null,
};

const store = writable<EngineState>(initial);

export const engine = {
  subscribe: store.subscribe,
};

/** Test helper — not used by views. */
export function resetEngineState() {
  store.set({ ...initial });
}

/** Test helper — not used by views. */
export function setEngineState(state: EngineState) {
  store.set(state);
}

export function loadingModelLabel(filename: string | null): string {
  if (!filename) return '';
  return filename.replace(/\.gguf$/i, '');
}

export async function refreshEngineStatus(): Promise<LlmEngineStatus | null> {
  try {
    const status = await getEngineStatus();
    store.update((current) => ({ ...current, status }));
    return status;
  } catch {
    return get(store).status;
  }
}

export async function loadEngineModel(
  filename: string,
  inferenceProfile?: InferenceProfile
): Promise<void> {
  if (get(store).isLoading) return;

  store.update((current) => ({
    ...current,
    isLoading: true,
    loadingFilename: filename,
    loadingStartedAt: Date.now(),
    error: null,
  }));

  try {
    await loadModel(filename, inferenceProfile);
    const status = await getEngineStatus();
    store.set({
      status,
      isLoading: false,
      loadingFilename: null,
      loadingStartedAt: null,
      error: null,
    });
  } catch (err) {
    store.update((current) => ({
      ...current,
      isLoading: false,
      loadingFilename: null,
      loadingStartedAt: null,
      error: parseError(err).message,
    }));
    throw err;
  }
}

/** If a model is on disk but not in RAM, start loading it. */
export async function ensureEngineLoaded(): Promise<void> {
  const status = (await refreshEngineStatus()) ?? get(store).status;
  if (!status) return;
  if (status.is_loaded || !status.is_downloaded || !status.downloaded_filename) return;
  await loadEngineModel(status.downloaded_filename);
}
