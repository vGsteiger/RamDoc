import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/svelte';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { open } from '@tauri-apps/plugin-dialog';
import PromotedModelImportDialog from '../../lib/components/PromotedModelImportDialog.svelte';

vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn(),
}));

vi.mock('@tauri-apps/plugin-dialog', () => ({
  open: vi.fn(),
}));

const mockInvoke = vi.mocked(invoke);
const mockListen = vi.mocked(listen);
const mockOpen = vi.mocked(open);

const PREVIEW_MISSING = {
  display_name: 'RamDoc clinical mix',
  study_id: 'unit-study-v1',
  filename: 'clinical-mix.gguf',
  size_bytes: 4200000000,
  quantization: 'RamDoc-Mix-v1',
  artifact_found: false,
  artifact_size_matches: false,
  artifact_bytes: null,
  artifact_path: null,
  dominates: ['q4-standard'],
  baseline_artifacts: ['q4-standard'],
  worst_category_regression: 0.01,
};

const PREVIEW_FOUND = {
  ...PREVIEW_MISSING,
  artifact_found: true,
  artifact_size_matches: true,
  artifact_bytes: 4200000000,
  artifact_path: '/tmp/clinical-mix.gguf',
};

beforeEach(() => {
  mockInvoke.mockReset();
  mockListen.mockResolvedValue(vi.fn());
  mockOpen.mockReset();
});

describe('PromotedModelImportDialog', () => {
  it('explains the two-file import before anything is selected', () => {
    render(PromotedModelImportDialog, { props: { open: true } });
    expect(screen.getByRole('dialog')).toBeInTheDocument();
    expect(screen.getByText(/not clinical validation/i)).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /choose promotion record/i })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /verify and import/i })).toBeDisabled();
  });

  it('asks for the GGUF when the sibling file is missing', async () => {
    mockOpen.mockResolvedValueOnce('/tmp/clinical-mix.promotion.json');
    mockInvoke.mockResolvedValueOnce(PREVIEW_MISSING);
    render(PromotedModelImportDialog, { props: { open: true } });

    await fireEvent.click(screen.getByRole('button', { name: /choose promotion record/i }));

    await waitFor(() =>
      expect(screen.getByText(/clinical-mix.gguf was not found/i)).toBeInTheDocument()
    );
    expect(screen.getByRole('button', { name: /choose model file/i })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /verify and import/i })).toBeDisabled();
  });

  it('imports with the located GGUF path and reports success', async () => {
    mockOpen.mockResolvedValueOnce('/tmp/clinical-mix.promotion.json');
    mockInvoke.mockResolvedValueOnce(PREVIEW_FOUND).mockResolvedValueOnce({
      id: 'clinical-mix',
      name: 'RamDoc clinical mix',
      filename: 'clinical-mix.gguf',
      sha256: 'abc',
      size_bytes: 4200000000,
      downloaded_at: '2026-08-17T12:00:00Z',
      last_used: null,
      is_default: false,
    });
    const onImported = vi.fn();
    render(PromotedModelImportDialog, { props: { open: true, onImported } });

    await fireEvent.click(screen.getByRole('button', { name: /choose promotion record/i }));
    await waitFor(() => expect(screen.getByText(/found matching model file/i)).toBeInTheDocument());

    await fireEvent.click(screen.getByRole('button', { name: /verify and import/i }));

    await waitFor(() =>
      expect(screen.getByText(/imported ramdoc clinical mix/i)).toBeInTheDocument()
    );
    expect(mockInvoke).toHaveBeenCalledWith('import_promoted_model', {
      promotionPath: '/tmp/clinical-mix.promotion.json',
      artifactPath: '/tmp/clinical-mix.gguf',
    });
    expect(onImported).toHaveBeenCalled();
  });
});
