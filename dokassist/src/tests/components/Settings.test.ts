import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/svelte';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import Settings from '../../routes/settings/+page.svelte';

// Mock Tauri event listener — the component uses this for download progress.
vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn(),
}));

const mockInvoke = vi.mocked(invoke);
const mockListen = vi.mocked(listen);

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

const NOT_LOADED = {
  is_loaded: false,
  model_name: null,
  model_path: null,
  total_ram_bytes: 8 * 1024 ** 3,
};

const LOADED = {
  is_loaded: true,
  model_name: 'Phi-4 Mini Q4_K_M',
  model_path: '/Users/user/DokAssist/models/Phi-4-mini-instruct-Q4_K_M.gguf',
  total_ram_bytes: 8 * 1024 ** 3,
};

const RECOMMENDED = {
  name: 'Phi-4 Mini Q4_K_M',
  filename: 'Phi-4-mini-instruct-Q4_K_M.gguf',
  size_bytes: 3 * 1024 ** 3,
  reason: 'Unter 16 GB RAM: Phi-4 Mini für minimale Ressourcen',
};

// Helper: mock the three onMount invoke calls for a "not loaded" machine.
function setupNotLoaded() {
  mockInvoke
    .mockResolvedValueOnce(NOT_LOADED)   // get_engine_status
    .mockResolvedValueOnce(RECOMMENDED)  // get_recommended_model
    .mockResolvedValueOnce('0.1.0');     // get_app_version
}

beforeEach(() => {
  mockInvoke.mockReset();
  mockListen.mockReset();
  // Default: listen returns a no-op unlisten function.
  mockListen.mockResolvedValue(() => {});
});

// ---------------------------------------------------------------------------
// Initial render (before onMount resolves)
// ---------------------------------------------------------------------------

describe('Settings — before onMount resolves', () => {
  it('shows "No model downloaded" immediately', () => {
    // Never resolves — keeps the component in its pre-onMount state.
    mockInvoke.mockReturnValue(new Promise(() => {}));
    render(Settings);
    expect(screen.getByText('No model downloaded')).toBeInTheDocument();
  });
});

// ---------------------------------------------------------------------------
// Model not loaded
// ---------------------------------------------------------------------------

describe('Settings — model not loaded', () => {
  it('shows "No model downloaded" and available RAM after mount', async () => {
    setupNotLoaded();
    render(Settings);
    await waitFor(() => {
      expect(screen.getByText('No model downloaded')).toBeInTheDocument();
      expect(screen.getByText(/8\.0 GB system RAM available/)).toBeInTheDocument();
    });
  });

  it('shows the recommended model name and reason', async () => {
    setupNotLoaded();
    render(Settings);
    await waitFor(() => {
      expect(screen.getByText('Phi-4 Mini Q4_K_M')).toBeInTheDocument();
      expect(screen.getByText('Unter 16 GB RAM: Phi-4 Mini für minimale Ressourcen')).toBeInTheDocument();
    });
  });

  it('shows the model download size', async () => {
    setupNotLoaded();
    render(Settings);
    await waitFor(() => {
      expect(screen.getByText('3.0 GB')).toBeInTheDocument();
    });
  });

  it('shows "Download & Load" and "Load existing" buttons', async () => {
    setupNotLoaded();
    render(Settings);
    await waitFor(() => {
      expect(screen.getByRole('button', { name: /download & load/i })).toBeInTheDocument();
      expect(screen.getByRole('button', { name: /load existing/i })).toBeInTheDocument();
    });
  });
});

// ---------------------------------------------------------------------------
// Model already loaded
// ---------------------------------------------------------------------------

describe('Settings — model already loaded', () => {
  it('shows the loaded model name', async () => {
    mockInvoke.mockResolvedValueOnce(LOADED).mockResolvedValueOnce(RECOMMENDED);
    render(Settings);
    await waitFor(() => {
      expect(screen.getByText('Phi-4 Mini Q4_K_M')).toBeInTheDocument();
    });
  });

  it('shows "Loaded" in the status line', async () => {
    mockInvoke.mockResolvedValueOnce(LOADED).mockResolvedValueOnce(RECOMMENDED);
    render(Settings);
    await waitFor(() => {
      expect(screen.getByText(/Loaded/)).toBeInTheDocument();
    });
  });

  it('hides the download/load buttons', async () => {
    mockInvoke.mockResolvedValueOnce(LOADED).mockResolvedValueOnce(RECOMMENDED);
    render(Settings);
    await waitFor(() => screen.getByText(/Loaded/));
    expect(screen.queryByRole('button', { name: /download & load/i })).not.toBeInTheDocument();
    expect(screen.queryByRole('button', { name: /load existing/i })).not.toBeInTheDocument();
  });

  it('shows the "Model ready" success message', async () => {
    mockInvoke.mockResolvedValueOnce(LOADED).mockResolvedValueOnce(RECOMMENDED);
    render(Settings);
    await waitFor(() => {
      expect(screen.getByText(/Model ready/i)).toBeInTheDocument();
    });
  });
});

// ---------------------------------------------------------------------------
// "Load existing" interaction
// ---------------------------------------------------------------------------

describe('Settings — "Load existing" button', () => {
  it('calls load_model with the recommended filename', async () => {
    mockInvoke
      .mockResolvedValueOnce(NOT_LOADED)  // onMount: getEngineStatus
      .mockResolvedValueOnce(RECOMMENDED) // onMount: getRecommendedModel
      .mockResolvedValueOnce(undefined)   // load_model
      .mockResolvedValueOnce(LOADED);     // getEngineStatus after load

    render(Settings);
    await waitFor(() => screen.getByRole('button', { name: /load existing/i }));
    await fireEvent.click(screen.getByRole('button', { name: /load existing/i }));

    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith('load_model', {
        modelFilename: 'Phi-4-mini-instruct-Q4_K_M.gguf',
      });
    });
  });

  it('shows "Model ready" after a successful load', async () => {
    mockInvoke
      .mockResolvedValueOnce(NOT_LOADED)
      .mockResolvedValueOnce(RECOMMENDED)
      .mockResolvedValueOnce('0.1.0')     // getAppVersion
      .mockResolvedValueOnce(undefined)
      .mockResolvedValueOnce(LOADED);

    render(Settings);
    await waitFor(() => screen.getByRole('button', { name: /load existing/i }));
    await fireEvent.click(screen.getByRole('button', { name: /load existing/i }));

    await waitFor(() => {
      expect(screen.getByText(/Model ready/i)).toBeInTheDocument();
    });
  });

  it('shows an error message when load_model fails', async () => {
    mockInvoke
      .mockResolvedValueOnce(NOT_LOADED)
      .mockResolvedValueOnce(RECOMMENDED)
      .mockResolvedValueOnce('0.1.0')     // getAppVersion
      .mockRejectedValueOnce(new Error('File not found'));

    render(Settings);
    await waitFor(() => screen.getByRole('button', { name: /load existing/i }));
    await fireEvent.click(screen.getByRole('button', { name: /load existing/i }));

    await waitFor(() => {
      expect(screen.getByText(/File not found/)).toBeInTheDocument();
    });
  });

  it('shows a Tauri-style error message correctly', async () => {
    mockInvoke
      .mockResolvedValueOnce(NOT_LOADED)
      .mockResolvedValueOnce(RECOMMENDED)
      .mockResolvedValueOnce('0.1.0')     // getAppVersion
      .mockRejectedValueOnce({ code: 'LLM_ERROR', message: 'Model binary missing' });

    render(Settings);
    await waitFor(() => screen.getByRole('button', { name: /load existing/i }));
    await fireEvent.click(screen.getByRole('button', { name: /load existing/i }));

    await waitFor(() => {
      expect(screen.getByText('Model binary missing')).toBeInTheDocument();
    });
  });

  it('disables both buttons while loading', async () => {
    let resolveLoad!: () => void;
    mockInvoke
      .mockResolvedValueOnce(NOT_LOADED)
      .mockResolvedValueOnce(RECOMMENDED)
      .mockResolvedValueOnce('0.1.0')     // getAppVersion
      .mockReturnValueOnce(new Promise((res) => { resolveLoad = () => res(undefined); }));

    render(Settings);
    await waitFor(() => screen.getByRole('button', { name: /load existing/i }));
    await fireEvent.click(screen.getByRole('button', { name: /load existing/i }));

    await waitFor(() => {
      expect(screen.getByRole('button', { name: /loading/i })).toBeDisabled();
    });

    // Clean up the hanging promise.
    resolveLoad();
  });
});

// ---------------------------------------------------------------------------
// "Download & Load" interaction
// ---------------------------------------------------------------------------

describe('Settings — "Download & Load" button', () => {
  it('calls download_model with the recommended model object', async () => {
    mockInvoke
      .mockResolvedValueOnce(NOT_LOADED)
      .mockResolvedValueOnce(RECOMMENDED)
      .mockResolvedValueOnce(undefined)   // download_model
      .mockResolvedValueOnce(undefined)   // load_model
      .mockResolvedValueOnce(LOADED);     // getEngineStatus after load

    render(Settings);
    await waitFor(() => screen.getByRole('button', { name: /download & load/i }));
    await fireEvent.click(screen.getByRole('button', { name: /download & load/i }));

    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith('download_model', { model: RECOMMENDED });
    });
  });

  it('calls load_model automatically after download completes', async () => {
    mockInvoke
      .mockResolvedValueOnce(NOT_LOADED)
      .mockResolvedValueOnce(RECOMMENDED)
      .mockResolvedValueOnce(undefined)
      .mockResolvedValueOnce(undefined)
      .mockResolvedValueOnce(LOADED);

    render(Settings);
    await waitFor(() => screen.getByRole('button', { name: /download & load/i }));
    await fireEvent.click(screen.getByRole('button', { name: /download & load/i }));

    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith('load_model', {
        modelFilename: 'Phi-4-mini-instruct-Q4_K_M.gguf',
      });
    });
  });

  it('shows "Model ready" after a successful download and load', async () => {
    mockInvoke
      .mockResolvedValueOnce(NOT_LOADED)
      .mockResolvedValueOnce(RECOMMENDED)
      .mockResolvedValueOnce('0.1.0')     // getAppVersion
      .mockResolvedValueOnce(undefined)
      .mockResolvedValueOnce(undefined)
      .mockResolvedValueOnce(LOADED);

    render(Settings);
    await waitFor(() => screen.getByRole('button', { name: /download & load/i }));
    await fireEvent.click(screen.getByRole('button', { name: /download & load/i }));

    await waitFor(() => {
      expect(screen.getByText(/Model ready/i)).toBeInTheDocument();
    });
  });

  it('disables the button and shows "Downloading…" while in progress', async () => {
    let resolveDownload!: () => void;
    mockInvoke
      .mockResolvedValueOnce(NOT_LOADED)
      .mockResolvedValueOnce(RECOMMENDED)
      .mockResolvedValueOnce('0.1.0')     // getAppVersion
      .mockReturnValueOnce(new Promise((res) => { resolveDownload = () => res(undefined); }));

    render(Settings);
    await waitFor(() => screen.getByRole('button', { name: /download & load/i }));
    await fireEvent.click(screen.getByRole('button', { name: /download & load/i }));

    await waitFor(() => {
      expect(screen.getByRole('button', { name: /downloading/i })).toBeDisabled();
    });

    resolveDownload();
  });

  it('subscribes to model-download-progress and model-download-done events', async () => {
    mockInvoke
      .mockResolvedValueOnce(NOT_LOADED)
      .mockResolvedValueOnce(RECOMMENDED)
      .mockResolvedValueOnce(undefined)
      .mockResolvedValueOnce(undefined)
      .mockResolvedValueOnce(LOADED);

    render(Settings);
    await waitFor(() => screen.getByRole('button', { name: /download & load/i }));
    await fireEvent.click(screen.getByRole('button', { name: /download & load/i }));

    await waitFor(() => {
      expect(mockListen).toHaveBeenCalledWith('model-download-progress', expect.any(Function));
      expect(mockListen).toHaveBeenCalledWith('model-download-done', expect.any(Function));
    });
  });

  it('shows an error message when download fails', async () => {
    mockInvoke
      .mockResolvedValueOnce(NOT_LOADED)
      .mockResolvedValueOnce(RECOMMENDED)
      .mockResolvedValueOnce('0.1.0')     // getAppVersion
      .mockRejectedValueOnce(new Error('Network error'));

    render(Settings);
    await waitFor(() => screen.getByRole('button', { name: /download & load/i }));
    await fireEvent.click(screen.getByRole('button', { name: /download & load/i }));

    await waitFor(() => {
      expect(screen.getByText(/Network error/)).toBeInTheDocument();
    });
  });
});
