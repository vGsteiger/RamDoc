import { describe, it, expect, vi, beforeEach } from 'vitest';
import { invoke } from '@tauri-apps/api/core';
import { insertDictation } from '$lib/dictation';
import {
  audioMimeType,
  cancelDictation,
  getDictationStatus,
  isAudioMime,
  startDictation,
  stopDictation,
  downloadDictationModel,
  setActiveDictationModel,
} from '$lib/api';

vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }));
const mockInvoke = vi.mocked(invoke);

function textarea(value: string, start: number, end = start): HTMLTextAreaElement {
  const el = document.createElement('textarea');
  el.value = value;
  el.setSelectionRange(start, end);
  return el;
}

describe('insertDictation', () => {
  it('appends to an empty value without padding', () => {
    expect(insertDictation(null, '', 'Hallo Welt.')).toBe('Hallo Welt.');
  });

  it('appends with a separating space when there is no caret element', () => {
    expect(insertDictation(undefined, 'Befund:', 'unauffällig.')).toBe('Befund: unauffällig.');
  });

  it('does not double spaces after whitespace', () => {
    expect(insertDictation(null, 'Befund:\n', 'unauffällig.')).toBe('Befund:\nunauffällig.');
  });

  it('inserts at the caret and pads both sides', () => {
    const el = textarea('AB', 1);
    expect(insertDictation(el, 'AB', 'x')).toBe('A x B');
  });

  it('replaces the selection', () => {
    const el = textarea('eins zwei drei', 5, 9);
    expect(insertDictation(el, 'eins zwei drei', 'vier')).toBe('eins vier drei');
  });

  it('does not add a space before punctuation', () => {
    const el = textarea('Stimmung.', 8);
    expect(insertDictation(el, 'Stimmung.', 'gedrückt')).toBe('Stimmung gedrückt.');
  });
});

describe('audio MIME detection', () => {
  it('keeps a reported audio type', () => {
    expect(audioMimeType('a.bin', 'audio/mpeg')).toBe('audio/mpeg');
  });

  it('falls back to the extension when WebKit reports no type', () => {
    expect(audioMimeType('Diktat.M4A', '')).toBe('audio/mp4');
    expect(audioMimeType('memo.caf', 'application/octet-stream')).toBe('audio/x-caf');
  });

  it('rejects non-audio files', () => {
    expect(audioMimeType('bericht.pdf', 'application/pdf')).toBeNull();
    expect(isAudioMime('application/pdf')).toBe(false);
    expect(isAudioMime('audio/wav')).toBe(true);
  });
});

describe('dictation commands', () => {
  beforeEach(() => mockInvoke.mockReset());

  it('maps to the backend command names and argument casing', async () => {
    mockInvoke.mockResolvedValue(undefined);
    await getDictationStatus();
    await startDictation();
    await stopDictation(7);
    await cancelDictation(7);
    await cancelDictation();
    await downloadDictationModel('parakeet-tdt-0.6b-v3');
    await setActiveDictationModel('whisper-large-v3-turbo');
    expect(mockInvoke.mock.calls).toEqual([
      ['get_dictation_status'],
      ['start_dictation'],
      ['stop_dictation', { id: 7 }],
      ['cancel_dictation', { id: 7 }],
      ['cancel_dictation', { id: null }],
      ['download_dictation_model', { modelId: 'parakeet-tdt-0.6b-v3' }],
      ['set_active_dictation_model', { modelId: 'whisper-large-v3-turbo' }],
    ]);
  });
});
