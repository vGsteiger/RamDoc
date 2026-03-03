import { describe, it, expect, vi, beforeEach } from 'vitest';
import { invoke } from '@tauri-apps/api/core';
import {
  checkAuth,
  listPatients,
  createPatient,
  updatePatient,
  deletePatient,
  generateReport,
  globalSearch,
  uploadFile,
  downloadFile,
  listFiles,
  deleteFile,
  listReports,
  listSessionsForPatient,
  listDiagnosesForPatient,
  listMedicationsForPatient,
  resetApp,
  parseError,
  getEngineStatus,
  getRecommendedModel,
  downloadModel,
  loadModel,
  type ModelChoice,
} from '$lib/api';

// invoke is mocked globally via src/tests/setup.ts
const mockInvoke = vi.mocked(invoke);

beforeEach(() => {
  mockInvoke.mockReset();
});

// ---------------------------------------------------------------------------
// Auth
// ---------------------------------------------------------------------------

describe('checkAuth', () => {
  it('calls check_auth and returns the auth status string', async () => {
    mockInvoke.mockResolvedValueOnce('locked');
    const result = await checkAuth();
    expect(mockInvoke).toHaveBeenCalledWith('check_auth');
    expect(result).toBe('locked');
  });
});

describe('getEngineStatus', () => {
  it('calls get_engine_status and returns engine info', async () => {
    const status = { is_loaded: false, model_name: null, model_path: null, total_ram_bytes: 0 };
    mockInvoke.mockResolvedValueOnce(status);
    const result = await getEngineStatus();
    expect(mockInvoke).toHaveBeenCalledWith('get_engine_status');
    expect(result).toEqual(status);
  });
});

describe('getRecommendedModel', () => {
  it('calls get_recommended_model and returns a ModelChoice', async () => {
    const model: ModelChoice = {
      name: 'Phi-4 Mini Q4_K_M',
      filename: 'Phi-4-mini-instruct-Q4_K_M.gguf',
      size_bytes: 3 * 1024 ** 3,
      reason: 'Unter 16 GB RAM: Phi-4 Mini für minimale Ressourcen',
    };
    mockInvoke.mockResolvedValueOnce(model);
    const result = await getRecommendedModel();
    expect(mockInvoke).toHaveBeenCalledWith('get_recommended_model');
    expect(result).toEqual(model);
  });
});

describe('downloadModel', () => {
  it('calls download_model with the model payload', async () => {
    const model: ModelChoice = {
      name: 'Phi-4 Mini Q4_K_M',
      filename: 'Phi-4-mini-instruct-Q4_K_M.gguf',
      size_bytes: 3 * 1024 ** 3,
      reason: 'Unter 16 GB RAM: Phi-4 Mini für minimale Ressourcen',
    };
    mockInvoke.mockResolvedValueOnce(undefined);
    await downloadModel(model);
    expect(mockInvoke).toHaveBeenCalledWith('download_model', { model });
  });

  it('propagates invoke errors', async () => {
    const model: ModelChoice = {
      name: 'Phi-4 Mini Q4_K_M',
      filename: 'Phi-4-mini-instruct-Q4_K_M.gguf',
      size_bytes: 3 * 1024 ** 3,
      reason: '',
    };
    mockInvoke.mockRejectedValueOnce({ code: 'VALIDATION_ERROR', message: 'Unknown model filename' });
    await expect(downloadModel(model)).rejects.toMatchObject({ code: 'VALIDATION_ERROR' });
  });
});

describe('loadModel', () => {
  it('calls load_model with the filename as modelFilename', async () => {
    mockInvoke.mockResolvedValueOnce(undefined);
    await loadModel('Phi-4-mini-instruct-Q4_K_M.gguf');
    expect(mockInvoke).toHaveBeenCalledWith('load_model', {
      modelFilename: 'Phi-4-mini-instruct-Q4_K_M.gguf',
    });
  });

  it('propagates invoke errors', async () => {
    mockInvoke.mockRejectedValueOnce({ code: 'LLM_ERROR', message: 'Model file not found' });
    await expect(loadModel('missing.gguf')).rejects.toMatchObject({ code: 'LLM_ERROR' });
  });
});

// ---------------------------------------------------------------------------
// Patients
// ---------------------------------------------------------------------------

describe('listPatients', () => {
  it('calls list_patients with limit and offset', async () => {
    const patients = [{ id: 'p1', first_name: 'John', last_name: 'Doe' }];
    mockInvoke.mockResolvedValueOnce(patients);
    const result = await listPatients(10, 0);
    expect(mockInvoke).toHaveBeenCalledWith('list_patients', { limit: 10, offset: 0 });
    expect(result).toEqual(patients);
  });
});

describe('createPatient', () => {
  it('calls create_patient with the full input payload', async () => {
    const input = {
      ahv_number: '7561234567897',
      first_name: 'Jane',
      last_name: 'Doe',
      date_of_birth: '1990-01-15',
    };
    const created = { id: 'p2', ...input, created_at: '', updated_at: '' };
    mockInvoke.mockResolvedValueOnce(created);
    const result = await createPatient(input);
    expect(mockInvoke).toHaveBeenCalledWith('create_patient', { input });
    expect(result).toEqual(created);
  });
});

describe('updatePatient', () => {
  it('calls update_patient with id and input', async () => {
    const updated = { id: 'p1', first_name: 'Janet', last_name: 'Doe', date_of_birth: '1990-01-15', ahv_number: '7561234567897', created_at: '', updated_at: '' };
    mockInvoke.mockResolvedValueOnce(updated);
    const result = await updatePatient('p1', { first_name: 'Janet' });
    expect(mockInvoke).toHaveBeenCalledWith('update_patient', { id: 'p1', input: { first_name: 'Janet' } });
    expect(result).toEqual(updated);
  });
});

describe('deletePatient', () => {
  it('calls delete_patient with the patient id', async () => {
    mockInvoke.mockResolvedValueOnce(undefined);
    await deletePatient('p1');
    expect(mockInvoke).toHaveBeenCalledWith('delete_patient', { id: 'p1' });
  });
});

// ---------------------------------------------------------------------------
// Files
// ---------------------------------------------------------------------------

describe('uploadFile', () => {
  it('calls upload_file with the correct parameters', async () => {
    const fileRecord = {
      id: 'f1',
      patient_id: 'p1',
      filename: 'test.pdf',
      vault_path: '/vault/f1',
      mime_type: 'application/pdf',
      size_bytes: 1024,
      created_at: '',
    };
    mockInvoke.mockResolvedValueOnce(fileRecord);
    const result = await uploadFile('p1', 'test.pdf', [1, 2, 3], 'application/pdf');
    expect(mockInvoke).toHaveBeenCalledWith('upload_file', {
      patientId: 'p1',
      filename: 'test.pdf',
      data: [1, 2, 3],
      mimeType: 'application/pdf',
    });
    expect(result).toEqual(fileRecord);
  });

  it('propagates invoke errors (e.g. file too large)', async () => {
    mockInvoke.mockRejectedValueOnce(new Error('File too large'));
    await expect(uploadFile('p1', 'big.pdf', [], 'application/pdf')).rejects.toThrow('File too large');
  });
});

describe('listFiles', () => {
  it('calls list_files with camelCase patientId', async () => {
    mockInvoke.mockResolvedValueOnce([]);
    await listFiles('p1');
    expect(mockInvoke).toHaveBeenCalledWith('list_files', { patientId: 'p1' });
  });
});

describe('downloadFile', () => {
  it('calls download_file with camelCase fileId', async () => {
    mockInvoke.mockResolvedValueOnce([1, 2, 3]);
    const result = await downloadFile('f1');
    expect(mockInvoke).toHaveBeenCalledWith('download_file', { fileId: 'f1' });
    expect(result).toEqual([1, 2, 3]);
  });
});

describe('deleteFile', () => {
  it('calls delete_file with camelCase fileId', async () => {
    mockInvoke.mockResolvedValueOnce(undefined);
    await deleteFile('f1');
    expect(mockInvoke).toHaveBeenCalledWith('delete_file', { fileId: 'f1' });
  });
});

// ---------------------------------------------------------------------------
// Search
// ---------------------------------------------------------------------------

describe('globalSearch', () => {
  it('calls global_search with query and limit', async () => {
    const results = [
      {
        result_type: 'patient',
        entity_id: 'p1',
        patient_id: 'p1',
        patient_name: 'Jane Doe',
        title: 'Jane Doe',
        snippet: '...',
        date: null,
        rank: 1,
      },
    ];
    mockInvoke.mockResolvedValueOnce(results);
    const result = await globalSearch('Jane', 5);
    expect(mockInvoke).toHaveBeenCalledWith('global_search', { query: 'Jane', limit: 5 });
    expect(result).toEqual(results);
  });
});

// ---------------------------------------------------------------------------
// Reports
// ---------------------------------------------------------------------------

describe('generateReport', () => {
  it('calls generate_report with the correct params', async () => {
    mockInvoke.mockResolvedValueOnce('Generated report text');
    const result = await generateReport('patient context', 'discharge_letter', 'session notes');
    expect(mockInvoke).toHaveBeenCalledWith('generate_report', {
      patientContext: 'patient context',
      reportType: 'discharge_letter',
      sessionNotes: 'session notes',
      systemPrompt: undefined,
    });
    expect(result).toBe('Generated report text');
  });

  it('passes an optional system_prompt when provided', async () => {
    mockInvoke.mockResolvedValueOnce('');
    await generateReport('ctx', 'type', 'notes', 'custom system prompt');
    expect(mockInvoke).toHaveBeenCalledWith('generate_report', expect.objectContaining({
      systemPrompt: 'custom system prompt',
    }));
  });
});

describe('listReports', () => {
  it('calls list_reports with camelCase patientId', async () => {
    mockInvoke.mockResolvedValueOnce([]);
    await listReports('p1', 10, 0);
    expect(mockInvoke).toHaveBeenCalledWith('list_reports', { patientId: 'p1', limit: 10, offset: 0 });
  });
});

// ---------------------------------------------------------------------------
// Sessions / Diagnoses / Medications — camelCase patientId guard
// ---------------------------------------------------------------------------

describe('listSessionsForPatient', () => {
  it('calls list_sessions_for_patient with camelCase patientId', async () => {
    mockInvoke.mockResolvedValueOnce([]);
    await listSessionsForPatient('p1', 10, 0);
    expect(mockInvoke).toHaveBeenCalledWith('list_sessions_for_patient', {
      patientId: 'p1',
      limit: 10,
      offset: 0,
    });
  });
});

describe('listDiagnosesForPatient', () => {
  it('calls list_diagnoses_for_patient with camelCase patientId', async () => {
    mockInvoke.mockResolvedValueOnce([]);
    await listDiagnosesForPatient('p1', 10, 0);
    expect(mockInvoke).toHaveBeenCalledWith('list_diagnoses_for_patient', {
      patientId: 'p1',
      limit: 10,
      offset: 0,
    });
  });
});

describe('listMedicationsForPatient', () => {
  it('calls list_medications_for_patient with camelCase patientId', async () => {
    mockInvoke.mockResolvedValueOnce([]);
    await listMedicationsForPatient('p1', 10, 0);
    expect(mockInvoke).toHaveBeenCalledWith('list_medications_for_patient', {
      patientId: 'p1',
      limit: 10,
      offset: 0,
    });
  });
});

// ---------------------------------------------------------------------------
// Auth — reset
// ---------------------------------------------------------------------------

describe('resetApp', () => {
  it('calls reset_app with no arguments', async () => {
    mockInvoke.mockResolvedValueOnce(undefined);
    await resetApp();
    expect(mockInvoke).toHaveBeenCalledWith('reset_app');
  });
});

// ---------------------------------------------------------------------------
// parseError
// ---------------------------------------------------------------------------

describe('parseError', () => {
  it('returns code and message from a structured Tauri error object', () => {
    const err = { code: 'KEYCHAIN_ERROR', message: 'Keychain error: item not found' };
    expect(parseError(err)).toEqual({ code: 'KEYCHAIN_ERROR', message: 'Keychain error: item not found' });
  });

  it('wraps a plain string in UNKNOWN_ERROR', () => {
    const result = parseError('something went wrong');
    expect(result).toEqual({ code: 'UNKNOWN_ERROR', message: 'something went wrong' });
  });

  it('wraps an Error instance in UNKNOWN_ERROR', () => {
    const result = parseError(new Error('boom'));
    expect(result.code).toBe('UNKNOWN_ERROR');
    expect(result.message).toContain('boom');
  });

  it('wraps null in UNKNOWN_ERROR', () => {
    const result = parseError(null);
    expect(result.code).toBe('UNKNOWN_ERROR');
  });

  it('wraps an object missing code in UNKNOWN_ERROR', () => {
    const result = parseError({ message: 'no code here' });
    expect(result.code).toBe('UNKNOWN_ERROR');
  });
});
