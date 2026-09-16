import { describe, it, expect, beforeEach, vi } from 'vitest';
import { get } from 'svelte/store';
import { invoke } from '@tauri-apps/api/core';
import {
  engine,
  ensureEngineLoaded,
  loadEngineModel,
  loadingModelLabel,
  refreshEngineStatus,
  resetEngineState,
} from '$lib/stores/engine';

const mockInvoke = vi.mocked(invoke);

const ENGINE_LOADED = {
  is_loaded: true,
  model_name: 'Phi-4 Mini',
  model_path: '/models/phi4.gguf',
  total_ram_bytes: 16 * 1024 ** 3,
  is_downloaded: true,
  downloaded_filename: 'phi4.gguf',
};

const ENGINE_NOT_LOADED = {
  is_loaded: false,
  model_name: null,
  model_path: null,
  total_ram_bytes: 8 * 1024 ** 3,
  is_downloaded: true,
  downloaded_filename: 'phi4.gguf',
};

beforeEach(() => {
  mockInvoke.mockReset();
  resetEngineState();
});

describe('loadingModelLabel', () => {
  it('strips the gguf suffix', () => {
    expect(loadingModelLabel('phi4.gguf')).toBe('phi4');
    expect(loadingModelLabel(null)).toBe('');
  });
});

describe('refreshEngineStatus', () => {
  it('stores the engine status', async () => {
    mockInvoke.mockResolvedValueOnce(ENGINE_LOADED);
    await refreshEngineStatus();
    expect(get(engine).status).toEqual(ENGINE_LOADED);
    expect(mockInvoke).toHaveBeenCalledWith('get_engine_status');
  });
});

describe('loadEngineModel', () => {
  it('marks the engine as loading until load_model resolves', async () => {
    let resolveLoad: (value: void) => void = () => {};
    mockInvoke.mockImplementation((cmd) => {
      if (cmd === 'load_model') {
        return new Promise<void>((resolve) => {
          resolveLoad = resolve;
        });
      }
      if (cmd === 'get_engine_status') return Promise.resolve(ENGINE_LOADED);
      return Promise.resolve(undefined);
    });

    const pending = loadEngineModel('phi4.gguf');
    await Promise.resolve();
    expect(get(engine).isLoading).toBe(true);
    expect(get(engine).loadingFilename).toBe('phi4.gguf');
    expect(get(engine).loadingStartedAt).toBeTypeOf('number');

    resolveLoad();
    await pending;
    expect(get(engine).isLoading).toBe(false);
    expect(get(engine).status?.is_loaded).toBe(true);
  });

  it('does not start a second load while one is in flight', async () => {
    mockInvoke.mockImplementation((cmd) => {
      if (cmd === 'load_model') return new Promise(() => {});
      return Promise.resolve(undefined);
    });
    void loadEngineModel('a.gguf');
    await Promise.resolve();
    await loadEngineModel('b.gguf');
    expect(mockInvoke.mock.calls.filter(([cmd]) => cmd === 'load_model')).toHaveLength(1);
  });

  it('records the error message when load_model fails', async () => {
    mockInvoke.mockRejectedValueOnce({ code: 'LLM', message: 'out of memory', ref: 'x' });
    await expect(loadEngineModel('phi4.gguf')).rejects.toBeTruthy();
    expect(get(engine).isLoading).toBe(false);
    expect(get(engine).error).toBe('out of memory');
  });
});

describe('ensureEngineLoaded', () => {
  it('loads a downloaded model that is not yet in memory', async () => {
    mockInvoke
      .mockResolvedValueOnce(ENGINE_NOT_LOADED)
      .mockResolvedValueOnce(undefined)
      .mockResolvedValueOnce(ENGINE_LOADED);
    await ensureEngineLoaded();
    expect(mockInvoke).toHaveBeenCalledWith('load_model', {
      modelFilename: 'phi4.gguf',
    });
    expect(get(engine).status?.is_loaded).toBe(true);
  });

  it('does nothing when a model is already loaded', async () => {
    mockInvoke.mockResolvedValueOnce(ENGINE_LOADED);
    await ensureEngineLoaded();
    expect(mockInvoke.mock.calls.some(([cmd]) => cmd === 'load_model')).toBe(false);
  });
});
