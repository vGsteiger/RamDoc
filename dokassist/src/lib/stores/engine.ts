import { get, writable } from 'svelte/store';
import {
  getEngineStatus,
  ensureWritingModelLoaded,
  loadModel,
  parseError,
  unloadModel,
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
let activeLoad: { filename: string; promise: Promise<void> } | null = null;

export const engine = {
  subscribe: store.subscribe,
};

/** Test helper — not used by views. */
export function resetEngineState() {
  activeLoad = null;
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
  if (activeLoad) {
    if (activeLoad.filename === filename) return activeLoad.promise;
    return Promise.reject(
      new Error(`Model '${activeLoad.filename}' is already loading; wait before loading '${filename}'.`)
    );
  }

  store.update((current) => ({
    ...current,
    isLoading: true,
    loadingFilename: filename,
    loadingStartedAt: Date.now(),
    error: null,
  }));

  const promise = (async () => {
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
    } finally {
      activeLoad = null;
    }
  })();
  activeLoad = { filename, promise };
  return promise;
}

/** Ensure the durable default writing model is resident before chat work begins. */
export async function ensureEngineLoaded(): Promise<void> {
  const status = (await refreshEngineStatus()) ?? get(store).status;
  if (status?.is_loaded) return;
  const filename = status?.desired_model?.filename;
  if (!filename) return;
  if (activeLoad) {
    if (activeLoad.filename === filename) return activeLoad.promise;
    throw new Error(`Model '${activeLoad.filename}' is already loading.`);
  }
  store.update((current) => ({
    ...current,
    isLoading: true,
    loadingFilename: filename,
    loadingStartedAt: Date.now(),
    error: null,
  }));
  const promise = (async () => {
    try {
      const loaded = await ensureWritingModelLoaded();
      store.set({
        status: loaded,
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
    } finally {
      activeLoad = null;
    }
  })();
  activeLoad = { filename, promise };
  return promise;
}

export async function unloadEngineModel(): Promise<void> {
  if (activeLoad) {
    throw new Error(`Cannot unload while '${activeLoad.filename}' is loading.`);
  }
  await unloadModel();
  await refreshEngineStatus();
}
