import { describe, it, expect, vi, beforeEach } from 'vitest';
import { invoke } from '@tauri-apps/api/core';
import {
  checkAuth,
  initializeApp,
  unlockApp,
  recoverApp,
  lockApp,
  listPatients,
  createPatient,
  getPatient,
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
  formatError,
  getUserFriendlyMessage,
  getEngineStatus,
  getEmbedStatus,
  initializeEmbedEngine,
  getRecommendedModel,
  downloadModel,
  loadModel,
  createSession,
  getSession,
  listAllSessions,
  updateSession,
  deleteSession,
  createDiagnosis,
  getDiagnosis,
  updateDiagnosis,
  deleteDiagnosis,
  createMedication,
  getMedication,
  updateMedication,
  deleteMedication,
  createReport,
  getReport,
  updateReport,
  deleteReport,
  checkForUpdates,
  installUpdate,
  getAppVersion,
  exportAllPatientData,
  runAgentTurn,
  createChatSession,
  getOrCreatePatientChatSession,
  listChatSessions,
  deleteChatSession,
  getChatMessages,
  renameChatSession,
  uploadLiterature,
  getLiteratureById,
  listAllLiterature,
  updateLiteratureMetadata,
  deleteLiteratureDocument,
  downloadLiterature,
  processLiterature,
  searchLiterature,
  getLiteratureDocumentChunks,
  exportReportToPdf,
  exportReportToDocx,
  getDashboardData,
  type ModelChoice,
  type AppError,
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
    const err = { code: 'KEYCHAIN_ERROR', message: 'Keychain error: item not found', ref: 'SOME_REF' };
    expect(parseError(err)).toEqual({ code: 'KEYCHAIN_ERROR', message: 'Keychain error: item not found', ref: 'SOME_REF' });
  });

  it('wraps a plain string in UNKNOWN_ERROR', () => {
    const result = parseError('something went wrong');
    expect(result).toEqual({ code: 'UNKNOWN_ERROR', message: 'something went wrong', ref: 'UNKNOWN_REF' });
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

// ---------------------------------------------------------------------------
// formatError
// ---------------------------------------------------------------------------

describe('formatError', () => {
  it('formats message and ref into a human-readable string', () => {
    const err: AppError = { code: 'DB_ERROR', message: 'Connection failed', ref: 'ERR-001' };
    const result = formatError(err);
    expect(result).toContain('Connection failed');
    expect(result).toContain('ERR-001');
  });

  it('includes the "Error Reference:" label', () => {
    const err: AppError = { code: 'LLM_ERROR', message: 'Model unavailable', ref: 'LLM-42' };
    expect(formatError(err)).toContain('Error Reference:');
  });

  it('includes the support hint', () => {
    const err: AppError = { code: 'AUTH_REQUIRED', message: 'Please unlock', ref: 'AUTH-1' };
    expect(formatError(err)).toContain('support');
  });
});

// ---------------------------------------------------------------------------
// getUserFriendlyMessage
// ---------------------------------------------------------------------------

describe('getUserFriendlyMessage', () => {
  it('returns specific message for REPORT_NOT_FOUND', () => {
    const err: AppError = { code: 'REPORT_NOT_FOUND', message: 'raw', ref: 'X' };
    expect(getUserFriendlyMessage(err)).toMatch(/report could not be found/i);
  });

  it('returns specific message for PATIENT_NOT_FOUND', () => {
    const err: AppError = { code: 'PATIENT_NOT_FOUND', message: 'raw', ref: 'X' };
    expect(getUserFriendlyMessage(err)).toMatch(/patient could not be found/i);
  });

  it('returns specific message for SESSION_NOT_FOUND', () => {
    const err: AppError = { code: 'SESSION_NOT_FOUND', message: 'raw', ref: 'X' };
    expect(getUserFriendlyMessage(err)).toMatch(/session could not be found/i);
  });

  it('returns specific message for FILE_NOT_FOUND', () => {
    const err: AppError = { code: 'FILE_NOT_FOUND', message: 'raw', ref: 'X' };
    expect(getUserFriendlyMessage(err)).toMatch(/file could not be found/i);
  });

  it('returns specific message for REPORT_VALIDATION_ERROR', () => {
    const err: AppError = { code: 'REPORT_VALIDATION_ERROR', message: 'raw', ref: 'X' };
    expect(getUserFriendlyMessage(err)).toMatch(/report data is invalid/i);
  });

  it('returns specific message for PATIENT_VALIDATION_ERROR', () => {
    const err: AppError = { code: 'PATIENT_VALIDATION_ERROR', message: 'raw', ref: 'X' };
    expect(getUserFriendlyMessage(err)).toMatch(/patient data is invalid/i);
  });

  it('returns specific message for DB_UNIQUE_CONSTRAINT', () => {
    const err: AppError = { code: 'DB_UNIQUE_CONSTRAINT', message: 'raw', ref: 'X' };
    expect(getUserFriendlyMessage(err)).toMatch(/already exists/i);
  });

  it('returns specific message for DB_FOREIGN_KEY', () => {
    const err: AppError = { code: 'DB_FOREIGN_KEY', message: 'raw', ref: 'X' };
    expect(getUserFriendlyMessage(err)).toMatch(/cannot complete this operation/i);
  });

  it('returns specific message for AUTH_REQUIRED', () => {
    const err: AppError = { code: 'AUTH_REQUIRED', message: 'raw', ref: 'X' };
    expect(getUserFriendlyMessage(err)).toMatch(/unlock the application/i);
  });

  it('returns specific message for LLM_ERROR', () => {
    const err: AppError = { code: 'LLM_ERROR', message: 'raw', ref: 'X' };
    expect(getUserFriendlyMessage(err)).toMatch(/language model/i);
  });

  it('falls back to err.message for unknown codes', () => {
    const err: AppError = { code: 'SOME_UNKNOWN_CODE', message: 'Custom raw message', ref: 'X' };
    expect(getUserFriendlyMessage(err)).toBe('Custom raw message');
  });
});

// ---------------------------------------------------------------------------
// Auth lifecycle
// ---------------------------------------------------------------------------

describe('initializeApp', () => {
  it('calls initialize_app and returns migration list', async () => {
    mockInvoke.mockResolvedValueOnce(['001_init', '002_patients']);
    const result = await initializeApp();
    expect(mockInvoke).toHaveBeenCalledWith('initialize_app');
    expect(result).toEqual(['001_init', '002_patients']);
  });
});

describe('unlockApp', () => {
  it('calls unlock_app and returns true on success', async () => {
    mockInvoke.mockResolvedValueOnce(true);
    const result = await unlockApp();
    expect(mockInvoke).toHaveBeenCalledWith('unlock_app');
    expect(result).toBe(true);
  });

  it('calls unlock_app and returns false when credentials are wrong', async () => {
    mockInvoke.mockResolvedValueOnce(false);
    const result = await unlockApp();
    expect(result).toBe(false);
  });
});

describe('recoverApp', () => {
  it('calls recover_app with the mnemonic word array', async () => {
    const words = ['alpha', 'bravo', 'charlie'];
    mockInvoke.mockResolvedValueOnce(true);
    const result = await recoverApp(words);
    expect(mockInvoke).toHaveBeenCalledWith('recover_app', { words });
    expect(result).toBe(true);
  });
});

describe('lockApp', () => {
  it('calls lock_app with no arguments', async () => {
    mockInvoke.mockResolvedValueOnce(undefined);
    await lockApp();
    expect(mockInvoke).toHaveBeenCalledWith('lock_app');
  });
});

// ---------------------------------------------------------------------------
// Patient — getPatient
// ---------------------------------------------------------------------------

describe('getPatient', () => {
  it('calls get_patient with the patient id', async () => {
    const patient = { id: 'p1', first_name: 'Jane', last_name: 'Doe' };
    mockInvoke.mockResolvedValueOnce(patient);
    const result = await getPatient('p1');
    expect(mockInvoke).toHaveBeenCalledWith('get_patient', { id: 'p1' });
    expect(result).toEqual(patient);
  });
});

// ---------------------------------------------------------------------------
// Embed engine
// ---------------------------------------------------------------------------

describe('getEmbedStatus', () => {
  it('calls get_embed_status and returns the status object', async () => {
    const status = { is_loaded: true, is_downloaded: true };
    mockInvoke.mockResolvedValueOnce(status);
    const result = await getEmbedStatus();
    expect(mockInvoke).toHaveBeenCalledWith('get_embed_status');
    expect(result).toEqual(status);
  });
});

describe('initializeEmbedEngine', () => {
  it('calls initialize_embed_engine with no arguments', async () => {
    mockInvoke.mockResolvedValueOnce(undefined);
    await initializeEmbedEngine();
    expect(mockInvoke).toHaveBeenCalledWith('initialize_embed_engine');
  });
});

// ---------------------------------------------------------------------------
// Sessions
// ---------------------------------------------------------------------------

describe('createSession', () => {
  it('calls create_session with the input payload', async () => {
    const input = { patient_id: 'p1', session_date: '2025-06-01', session_type: 'individual' };
    const created = { id: 's1', ...input, duration_minutes: null, notes: null, amdp_data: null, created_at: '', updated_at: '' };
    mockInvoke.mockResolvedValueOnce(created);
    const result = await createSession(input);
    expect(mockInvoke).toHaveBeenCalledWith('create_session', { input });
    expect(result).toEqual(created);
  });
});

describe('getSession', () => {
  it('calls get_session with the session id', async () => {
    const session = { id: 's1', patient_id: 'p1', session_date: '2025-06-01', session_type: 'individual', duration_minutes: null, notes: null, amdp_data: null, created_at: '', updated_at: '' };
    mockInvoke.mockResolvedValueOnce(session);
    const result = await getSession('s1');
    expect(mockInvoke).toHaveBeenCalledWith('get_session', { id: 's1' });
    expect(result).toEqual(session);
  });
});

describe('listAllSessions', () => {
  it('calls list_all_sessions with limit and offset', async () => {
    mockInvoke.mockResolvedValueOnce([]);
    await listAllSessions(20, 0);
    expect(mockInvoke).toHaveBeenCalledWith('list_all_sessions', { limit: 20, offset: 0 });
  });
});

describe('updateSession', () => {
  it('calls update_session with id and input', async () => {
    const updated = { id: 's1', patient_id: 'p1', session_date: '2025-06-02', session_type: 'group', duration_minutes: null, notes: null, amdp_data: null, created_at: '', updated_at: '' };
    mockInvoke.mockResolvedValueOnce(updated);
    const result = await updateSession('s1', { session_date: '2025-06-02' });
    expect(mockInvoke).toHaveBeenCalledWith('update_session', { id: 's1', input: { session_date: '2025-06-02' } });
    expect(result).toEqual(updated);
  });
});

describe('deleteSession', () => {
  it('calls delete_session with the session id', async () => {
    mockInvoke.mockResolvedValueOnce(undefined);
    await deleteSession('s1');
    expect(mockInvoke).toHaveBeenCalledWith('delete_session', { id: 's1' });
  });
});

// ---------------------------------------------------------------------------
// Diagnoses
// ---------------------------------------------------------------------------

describe('createDiagnosis', () => {
  it('calls create_diagnosis with the input payload', async () => {
    const input = { patient_id: 'p1', icd10_code: 'F32.1', description: 'Major depressive episode', status: 'active', diagnosed_date: '2025-01-01' };
    const created = { id: 'd1', ...input, resolved_date: null, notes: null, created_at: '', updated_at: '' };
    mockInvoke.mockResolvedValueOnce(created);
    const result = await createDiagnosis(input);
    expect(mockInvoke).toHaveBeenCalledWith('create_diagnosis', { input });
    expect(result).toEqual(created);
  });
});

describe('getDiagnosis', () => {
  it('calls get_diagnosis with the diagnosis id', async () => {
    const diagnosis = { id: 'd1', patient_id: 'p1', icd10_code: 'F32.1', description: 'Major depressive episode', status: 'active', diagnosed_date: '2025-01-01', resolved_date: null, notes: null, created_at: '', updated_at: '' };
    mockInvoke.mockResolvedValueOnce(diagnosis);
    const result = await getDiagnosis('d1');
    expect(mockInvoke).toHaveBeenCalledWith('get_diagnosis', { id: 'd1' });
    expect(result).toEqual(diagnosis);
  });
});

describe('updateDiagnosis', () => {
  it('calls update_diagnosis with id and input', async () => {
    const updated = { id: 'd1', patient_id: 'p1', icd10_code: 'F33.0', description: 'Recurrent depressive', status: 'active', diagnosed_date: '2025-01-01', resolved_date: null, notes: null, created_at: '', updated_at: '' };
    mockInvoke.mockResolvedValueOnce(updated);
    const result = await updateDiagnosis('d1', { icd10_code: 'F33.0' });
    expect(mockInvoke).toHaveBeenCalledWith('update_diagnosis', { id: 'd1', input: { icd10_code: 'F33.0' } });
    expect(result).toEqual(updated);
  });
});

describe('deleteDiagnosis', () => {
  it('calls delete_diagnosis with the diagnosis id', async () => {
    mockInvoke.mockResolvedValueOnce(undefined);
    await deleteDiagnosis('d1');
    expect(mockInvoke).toHaveBeenCalledWith('delete_diagnosis', { id: 'd1' });
  });
});

// ---------------------------------------------------------------------------
// Medications
// ---------------------------------------------------------------------------

describe('createMedication', () => {
  it('calls create_medication with the input payload', async () => {
    const input = { patient_id: 'p1', substance: 'Sertraline', dosage: '50mg', frequency: 'once daily', start_date: '2025-02-01' };
    const created = { id: 'm1', ...input, end_date: null, notes: null, created_at: '', updated_at: '' };
    mockInvoke.mockResolvedValueOnce(created);
    const result = await createMedication(input);
    expect(mockInvoke).toHaveBeenCalledWith('create_medication', { input });
    expect(result).toEqual(created);
  });
});

describe('getMedication', () => {
  it('calls get_medication with the medication id', async () => {
    const medication = { id: 'm1', patient_id: 'p1', substance: 'Sertraline', dosage: '50mg', frequency: 'once daily', start_date: '2025-02-01', end_date: null, notes: null, created_at: '', updated_at: '' };
    mockInvoke.mockResolvedValueOnce(medication);
    const result = await getMedication('m1');
    expect(mockInvoke).toHaveBeenCalledWith('get_medication', { id: 'm1' });
    expect(result).toEqual(medication);
  });
});

describe('updateMedication', () => {
  it('calls update_medication with id and input', async () => {
    const updated = { id: 'm1', patient_id: 'p1', substance: 'Sertraline', dosage: '100mg', frequency: 'once daily', start_date: '2025-02-01', end_date: null, notes: null, created_at: '', updated_at: '' };
    mockInvoke.mockResolvedValueOnce(updated);
    const result = await updateMedication('m1', { dosage: '100mg' });
    expect(mockInvoke).toHaveBeenCalledWith('update_medication', { id: 'm1', input: { dosage: '100mg' } });
    expect(result).toEqual(updated);
  });
});

describe('deleteMedication', () => {
  it('calls delete_medication with the medication id', async () => {
    mockInvoke.mockResolvedValueOnce(undefined);
    await deleteMedication('m1');
    expect(mockInvoke).toHaveBeenCalledWith('delete_medication', { id: 'm1' });
  });
});

// ---------------------------------------------------------------------------
// Reports CRUD (non-generate)
// ---------------------------------------------------------------------------

describe('createReport', () => {
  it('calls create_report with the input payload', async () => {
    const input = { patient_id: 'p1', report_type: 'discharge_letter', content: 'Report text', model_name: null, prompt_hash: null, session_ids: null };
    const created = { id: 'r1', ...input, generated_at: '', created_at: '' };
    mockInvoke.mockResolvedValueOnce(created);
    const result = await createReport(input);
    expect(mockInvoke).toHaveBeenCalledWith('create_report', { input });
    expect(result).toEqual(created);
  });
});

describe('getReport', () => {
  it('calls get_report with the report id', async () => {
    const report = { id: 'r1', patient_id: 'p1', report_type: 'discharge_letter', content: 'Report text', generated_at: '', model_name: null, prompt_hash: null, session_ids: null, created_at: '' };
    mockInvoke.mockResolvedValueOnce(report);
    const result = await getReport('r1');
    expect(mockInvoke).toHaveBeenCalledWith('get_report', { id: 'r1' });
    expect(result).toEqual(report);
  });
});

describe('updateReport', () => {
  it('calls update_report with id and input', async () => {
    const updated = { id: 'r1', patient_id: 'p1', report_type: 'progress_note', content: 'Updated text', generated_at: '', model_name: null, prompt_hash: null, session_ids: null, created_at: '' };
    mockInvoke.mockResolvedValueOnce(updated);
    const result = await updateReport('r1', { content: 'Updated text' });
    expect(mockInvoke).toHaveBeenCalledWith('update_report', { id: 'r1', input: { content: 'Updated text' } });
    expect(result).toEqual(updated);
  });
});

describe('deleteReport', () => {
  it('calls delete_report with the report id', async () => {
    mockInvoke.mockResolvedValueOnce(undefined);
    await deleteReport('r1');
    expect(mockInvoke).toHaveBeenCalledWith('delete_report', { id: 'r1' });
  });
});

// ---------------------------------------------------------------------------
// App helpers
// ---------------------------------------------------------------------------

describe('checkForUpdates', () => {
  it('calls check_for_updates and returns update info', async () => {
    const info = { current_version: '0.1.0', latest_version: '0.2.0', update_available: true, body: 'Changelog', date: '2025-01-01' };
    mockInvoke.mockResolvedValueOnce(info);
    const result = await checkForUpdates();
    expect(mockInvoke).toHaveBeenCalledWith('check_for_updates');
    expect(result).toEqual(info);
  });
});

describe('installUpdate', () => {
  it('calls install_update with no arguments', async () => {
    mockInvoke.mockResolvedValueOnce(undefined);
    await installUpdate();
    expect(mockInvoke).toHaveBeenCalledWith('install_update');
  });
});

describe('getAppVersion', () => {
  it('calls get_app_version and returns the version string', async () => {
    mockInvoke.mockResolvedValueOnce('0.1.1');
    const result = await getAppVersion();
    expect(mockInvoke).toHaveBeenCalledWith('get_app_version');
    expect(result).toBe('0.1.1');
  });
});

describe('exportAllPatientData', () => {
  it('calls export_all_patient_data and returns byte array', async () => {
    const bytes = [1, 2, 3, 4, 5];
    mockInvoke.mockResolvedValueOnce(bytes);
    const result = await exportAllPatientData();
    expect(mockInvoke).toHaveBeenCalledWith('export_all_patient_data');
    expect(result).toEqual(bytes);
  });
});

describe('exportReportToPdf', () => {
  it('calls export_report_to_pdf with reportId and returns byte array', async () => {
    const bytes = [37, 80, 68, 70]; // %PDF magic bytes
    mockInvoke.mockResolvedValueOnce(bytes);
    const result = await exportReportToPdf('report-abc');
    expect(mockInvoke).toHaveBeenCalledWith('export_report_to_pdf', { reportId: 'report-abc' });
    expect(result).toEqual(bytes);
  });
});

describe('exportReportToDocx', () => {
  it('calls export_report_to_docx with reportId and returns byte array', async () => {
    const bytes = [80, 75, 3, 4]; // PK ZIP magic bytes (DOCX is a ZIP)
    mockInvoke.mockResolvedValueOnce(bytes);
    const result = await exportReportToDocx('report-xyz');
    expect(mockInvoke).toHaveBeenCalledWith('export_report_to_docx', { reportId: 'report-xyz' });
    expect(result).toEqual(bytes);
  });
});

// ---------------------------------------------------------------------------
// Chat / Agent API
// ---------------------------------------------------------------------------

describe('runAgentTurn', () => {
  it('calls run_agent_turn with sessionId and userMessage', async () => {
    const turnResult = { session_id: 'cs1', final_answer: 'Here is the answer', tool_calls_made: [] };
    mockInvoke.mockResolvedValueOnce(turnResult);
    const result = await runAgentTurn('cs1', 'What medications does the patient take?');
    expect(mockInvoke).toHaveBeenCalledWith('run_agent_turn', { sessionId: 'cs1', userMessage: 'What medications does the patient take?' });
    expect(result).toEqual(turnResult);
  });
});

describe('createChatSession', () => {
  it('calls create_chat_session with scope and optional patientId', async () => {
    const session = { id: 'cs1', scope: 'patient', patient_id: 'p1', title: 'Chat', created_at: '', updated_at: '' };
    mockInvoke.mockResolvedValueOnce(session);
    const result = await createChatSession('patient', 'p1');
    expect(mockInvoke).toHaveBeenCalledWith('create_chat_session', { scope: 'patient', patientId: 'p1', title: undefined });
    expect(result).toEqual(session);
  });

  it('calls create_chat_session with a custom title', async () => {
    const session = { id: 'cs2', scope: 'global', patient_id: null, title: 'My Chat', created_at: '', updated_at: '' };
    mockInvoke.mockResolvedValueOnce(session);
    await createChatSession('global', undefined, 'My Chat');
    expect(mockInvoke).toHaveBeenCalledWith('create_chat_session', { scope: 'global', patientId: undefined, title: 'My Chat' });
  });
});

describe('getOrCreatePatientChatSession', () => {
  it('calls get_or_create_patient_chat_session with patientId', async () => {
    const session = { id: 'cs1', scope: 'patient', patient_id: 'p1', title: 'Chat', created_at: '', updated_at: '' };
    mockInvoke.mockResolvedValueOnce(session);
    const result = await getOrCreatePatientChatSession('p1');
    expect(mockInvoke).toHaveBeenCalledWith('get_or_create_patient_chat_session', { patientId: 'p1' });
    expect(result).toEqual(session);
  });
});

describe('listChatSessions', () => {
  it('calls list_chat_sessions with scope', async () => {
    mockInvoke.mockResolvedValueOnce([]);
    await listChatSessions('global');
    expect(mockInvoke).toHaveBeenCalledWith('list_chat_sessions', { scope: 'global', patientId: undefined });
  });
});

describe('deleteChatSession', () => {
  it('calls delete_chat_session with sessionId', async () => {
    mockInvoke.mockResolvedValueOnce(undefined);
    await deleteChatSession('cs1');
    expect(mockInvoke).toHaveBeenCalledWith('delete_chat_session', { sessionId: 'cs1' });
  });
});

describe('getChatMessages', () => {
  it('calls get_chat_messages with sessionId and returns messages', async () => {
    const messages = [
      { id: 'msg1', session_id: 'cs1', role: 'user', content: 'Hello', tool_name: null, tool_args_json: null, tool_result_for: null, created_at: '' },
    ];
    mockInvoke.mockResolvedValueOnce(messages);
    const result = await getChatMessages('cs1');
    expect(mockInvoke).toHaveBeenCalledWith('get_chat_messages', { sessionId: 'cs1' });
    expect(result).toEqual(messages);
  });
});

describe('renameChatSession', () => {
  it('calls rename_chat_session with sessionId and title', async () => {
    const session = { id: 'cs1', scope: 'global', patient_id: null, title: 'New Title', created_at: '', updated_at: '' };
    mockInvoke.mockResolvedValueOnce(session);
    const result = await renameChatSession('cs1', 'New Title');
    expect(mockInvoke).toHaveBeenCalledWith('rename_chat_session', { sessionId: 'cs1', title: 'New Title' });
    expect(result).toEqual(session);
  });
});

// ---------------------------------------------------------------------------
// Literature API
// ---------------------------------------------------------------------------

describe('uploadLiterature', () => {
  it('calls upload_literature with the correct parameters', async () => {
    const literature = { id: 'lit1', filename: 'paper.pdf', vault_path: '/vault/lit1', mime_type: 'application/pdf', size_bytes: 2048, description: 'A paper', created_at: '', updated_at: '', chunk_count: 0 };
    mockInvoke.mockResolvedValueOnce(literature);
    const data = new Uint8Array([1, 2, 3]);
    const result = await uploadLiterature('paper.pdf', data, 'application/pdf', 'A paper');
    expect(mockInvoke).toHaveBeenCalledWith('upload_literature', {
      filename: 'paper.pdf',
      data: [1, 2, 3],
      mimeType: 'application/pdf',
      description: 'A paper',
    });
    expect(result).toEqual(literature);
  });

  it('defaults description to null when not provided', async () => {
    mockInvoke.mockResolvedValueOnce({});
    await uploadLiterature('doc.txt', new Uint8Array(), 'text/plain');
    expect(mockInvoke).toHaveBeenCalledWith('upload_literature', expect.objectContaining({ description: null }));
  });
});

describe('getLiteratureById', () => {
  it('calls get_literature_by_id with the document id', async () => {
    const literature = { id: 'lit1', filename: 'paper.pdf', vault_path: '', mime_type: 'application/pdf', size_bytes: 0, description: null, created_at: '', updated_at: '', chunk_count: 5 };
    mockInvoke.mockResolvedValueOnce(literature);
    const result = await getLiteratureById('lit1');
    expect(mockInvoke).toHaveBeenCalledWith('get_literature_by_id', { id: 'lit1' });
    expect(result).toEqual(literature);
  });
});

describe('listAllLiterature', () => {
  it('calls list_all_literature with default limit and offset', async () => {
    mockInvoke.mockResolvedValueOnce([]);
    await listAllLiterature();
    expect(mockInvoke).toHaveBeenCalledWith('list_all_literature', { limit: 100, offset: 0 });
  });

  it('calls list_all_literature with custom limit and offset', async () => {
    mockInvoke.mockResolvedValueOnce([]);
    await listAllLiterature(10, 20);
    expect(mockInvoke).toHaveBeenCalledWith('list_all_literature', { limit: 10, offset: 20 });
  });
});

describe('updateLiteratureMetadata', () => {
  it('calls update_literature_metadata with id and description', async () => {
    const updated = { id: 'lit1', filename: 'paper.pdf', vault_path: '', mime_type: 'application/pdf', size_bytes: 0, description: 'Updated desc', created_at: '', updated_at: '', chunk_count: 5 };
    mockInvoke.mockResolvedValueOnce(updated);
    const result = await updateLiteratureMetadata('lit1', 'Updated desc');
    expect(mockInvoke).toHaveBeenCalledWith('update_literature_metadata', { id: 'lit1', description: 'Updated desc' });
    expect(result).toEqual(updated);
  });

  it('passes null description to clear it', async () => {
    mockInvoke.mockResolvedValueOnce({});
    await updateLiteratureMetadata('lit1', null);
    expect(mockInvoke).toHaveBeenCalledWith('update_literature_metadata', { id: 'lit1', description: null });
  });
});

describe('deleteLiteratureDocument', () => {
  it('calls delete_literature_document with the id', async () => {
    mockInvoke.mockResolvedValueOnce(undefined);
    await deleteLiteratureDocument('lit1');
    expect(mockInvoke).toHaveBeenCalledWith('delete_literature_document', { id: 'lit1' });
  });
});

describe('downloadLiterature', () => {
  it('calls download_literature and converts the result to Uint8Array', async () => {
    mockInvoke.mockResolvedValueOnce([10, 20, 30]);
    const result = await downloadLiterature('lit1');
    expect(mockInvoke).toHaveBeenCalledWith('download_literature', { id: 'lit1' });
    expect(result).toBeInstanceOf(Uint8Array);
    expect(Array.from(result)).toEqual([10, 20, 30]);
  });
});

describe('processLiterature', () => {
  it('calls process_literature with the document id', async () => {
    mockInvoke.mockResolvedValueOnce(undefined);
    await processLiterature('lit1');
    expect(mockInvoke).toHaveBeenCalledWith('process_literature', { id: 'lit1' });
  });
});

describe('searchLiterature', () => {
  it('calls search_literature with query and default limit', async () => {
    const chunks = [{ chunk_id: 'c1', literature_id: 'lit1', filename: 'paper.pdf', chunk_index: 0, content: 'relevant text', similarity: 0.9 }];
    mockInvoke.mockResolvedValueOnce(chunks);
    const result = await searchLiterature('relevant query');
    expect(mockInvoke).toHaveBeenCalledWith('search_literature', { query: 'relevant query', limit: 5 });
    expect(result).toEqual(chunks);
  });

  it('calls search_literature with a custom limit', async () => {
    mockInvoke.mockResolvedValueOnce([]);
    await searchLiterature('query', 10);
    expect(mockInvoke).toHaveBeenCalledWith('search_literature', { query: 'query', limit: 10 });
  });
});

describe('getLiteratureDocumentChunks', () => {
  it('calls get_literature_document_chunks with the document id', async () => {
    const chunks = [{ id: 'c1', file_id: null, literature_id: 'lit1', chunk_index: 0, content: 'text', word_count: 50, created_at: '' }];
    mockInvoke.mockResolvedValueOnce(chunks);
    const result = await getLiteratureDocumentChunks('lit1');
    expect(mockInvoke).toHaveBeenCalledWith('get_literature_document_chunks', { id: 'lit1' });
    expect(result).toEqual(chunks);
  });
});

describe('getDashboardData', () => {
  it('calls get_dashboard_data and returns dashboard data', async () => {
    const dashboardData = {
      todays_sessions: [],
      recent_patients: [],
      sessions_with_incomplete_notes: [],
    };
    mockInvoke.mockResolvedValueOnce(dashboardData);
    const result = await getDashboardData();
    expect(mockInvoke).toHaveBeenCalledWith('get_dashboard_data');
    expect(result).toEqual(dashboardData);
  });
});
