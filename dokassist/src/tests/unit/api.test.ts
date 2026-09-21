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
  loadModel,
  ensureWritingModelLoaded,
  unloadModel,
  getRouterDiagnostics,
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
  exportFhirBundle,
  createVaultBackup,
  restoreVaultBackup,
  validateBackupArchive,
  exportPatientPdf,
  processFile,
  createEmail,
  getEmail,
  listEmails,
  updateEmail,
  deleteEmail,
  markEmailAsSent,
  createTreatmentPlan,
  getTreatmentPlan,
  listTreatmentPlansForPatient,
  updateTreatmentPlan,
  deleteTreatmentPlan,
  createTreatmentGoal,
  getTreatmentGoal,
  listTreatmentGoalsForPlan,
  updateTreatmentGoal,
  deleteTreatmentGoal,
  createTreatmentIntervention,
  getTreatmentIntervention,
  listTreatmentInterventionsForPlan,
  updateTreatmentIntervention,
  deleteTreatmentIntervention,
  createOutcomeScore,
  getOutcomeScore,
  listScoresForSession,
  listScoresByScale,
  listScoresForPatient,
  updateOutcomeScore,
  deleteOutcomeScore,
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
  generateSessionSummary,
  listModels,
  getModelInfo,
  downloadAndRegisterModel,
  inspectPromotedModel,
  importPromotedModel,
  deleteModel,
  setDefaultModel,
  getDefaultModel,
  createLetter,
  getLetter,
  listLetters,
  updateLetter,
  deleteLetter,
  markLetterAsFinalized,
  markLetterAsSent,
  generateLetter,
  getSettings,
  updateSettings,
  completeOnboarding,
  queryPatientHistory,
  previewPatientEvidence,
  indexPatientEvidence,
  getPatientEvidenceManifest,
  resolveEvidenceUnits,
  compareMedications,
  type Letter,
  type ModelChoice,
  type AppError,
  type MedicationComparison,
  type PracticeSettings,
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

describe('loadModel', () => {
  it('calls load_model with the filename as modelFilename', async () => {
    mockInvoke.mockResolvedValueOnce(undefined);
    await loadModel('Phi-4-mini-instruct-Q4_K_M.gguf');
    expect(mockInvoke).toHaveBeenCalledWith('load_model', {
      modelFilename: 'Phi-4-mini-instruct-Q4_K_M.gguf',
    });
  });

  it('passes an explicitly selected inference profile', async () => {
    mockInvoke.mockResolvedValueOnce(undefined);
    await loadModel('Qwen3-8B-Q4_K_M.gguf', 'q8-32k');
    expect(mockInvoke).toHaveBeenCalledWith('load_model', {
      modelFilename: 'Qwen3-8B-Q4_K_M.gguf',
      inferenceProfile: 'q8-32k',
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
    const updated = {
      id: 'p1',
      first_name: 'Janet',
      last_name: 'Doe',
      date_of_birth: '1990-01-15',
      ahv_number: '7561234567897',
      created_at: '',
      updated_at: '',
    };
    mockInvoke.mockResolvedValueOnce(updated);
    const result = await updatePatient('p1', { first_name: 'Janet' });
    expect(mockInvoke).toHaveBeenCalledWith('update_patient', {
      id: 'p1',
      input: { first_name: 'Janet' },
    });
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
    await expect(uploadFile('p1', 'big.pdf', [], 'application/pdf')).rejects.toThrow(
      'File too large'
    );
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
      thinkingEffort: null,
      sampler: null,
    });
    expect(result).toBe('Generated report text');
  });

  it('passes a sampler override when provided', async () => {
    mockInvoke.mockResolvedValueOnce('');
    const sampler = {
      temperature: 0.2,
      top_k: 20,
      top_p: 0.9,
      min_p: 0,
      repeat_penalty: 1,
      presence_penalty: 0,
      seed: 7,
    };
    await generateReport('ctx', 'type', 'notes', undefined, 'low', sampler);
    expect(mockInvoke).toHaveBeenCalledWith(
      'generate_report',
      expect.objectContaining({ sampler })
    );
  });

  it('passes an optional system_prompt when provided', async () => {
    mockInvoke.mockResolvedValueOnce('');
    await generateReport('ctx', 'type', 'notes', 'custom system prompt');
    expect(mockInvoke).toHaveBeenCalledWith(
      'generate_report',
      expect.objectContaining({
        systemPrompt: 'custom system prompt',
      })
    );
  });
});

describe('listReports', () => {
  it('calls list_reports with camelCase patientId', async () => {
    mockInvoke.mockResolvedValueOnce([]);
    await listReports('p1', 10, 0);
    expect(mockInvoke).toHaveBeenCalledWith('list_reports', {
      patientId: 'p1',
      limit: 10,
      offset: 0,
    });
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
    const err = {
      code: 'KEYCHAIN_ERROR',
      message: 'Keychain error: item not found',
      ref: 'SOME_REF',
    };
    expect(parseError(err)).toEqual({
      code: 'KEYCHAIN_ERROR',
      message: 'Keychain error: item not found',
      ref: 'SOME_REF',
    });
  });

  it('wraps a plain string in UNKNOWN_ERROR', () => {
    const result = parseError('something went wrong');
    expect(result).toEqual({
      code: 'UNKNOWN_ERROR',
      message: 'something went wrong',
      ref: 'UNKNOWN_REF',
    });
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
    const created = {
      id: 's1',
      ...input,
      duration_minutes: null,
      notes: null,
      amdp_data: null,
      created_at: '',
      updated_at: '',
    };
    mockInvoke.mockResolvedValueOnce(created);
    const result = await createSession(input);
    expect(mockInvoke).toHaveBeenCalledWith('create_session', { input });
    expect(result).toEqual(created);
  });
});

describe('getSession', () => {
  it('calls get_session with the session id', async () => {
    const session = {
      id: 's1',
      patient_id: 'p1',
      session_date: '2025-06-01',
      session_type: 'individual',
      duration_minutes: null,
      notes: null,
      amdp_data: null,
      created_at: '',
      updated_at: '',
    };
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
    const updated = {
      id: 's1',
      patient_id: 'p1',
      session_date: '2025-06-02',
      session_type: 'group',
      duration_minutes: null,
      notes: null,
      amdp_data: null,
      created_at: '',
      updated_at: '',
    };
    mockInvoke.mockResolvedValueOnce(updated);
    const result = await updateSession('s1', { session_date: '2025-06-02' });
    expect(mockInvoke).toHaveBeenCalledWith('update_session', {
      id: 's1',
      input: { session_date: '2025-06-02' },
    });
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
    const input = {
      patient_id: 'p1',
      icd10_code: 'F32.1',
      description: 'Major depressive episode',
      status: 'active',
      diagnosed_date: '2025-01-01',
    };
    const created = {
      id: 'd1',
      ...input,
      resolved_date: null,
      notes: null,
      created_at: '',
      updated_at: '',
    };
    mockInvoke.mockResolvedValueOnce(created);
    const result = await createDiagnosis(input);
    expect(mockInvoke).toHaveBeenCalledWith('create_diagnosis', { input });
    expect(result).toEqual(created);
  });
});

describe('getDiagnosis', () => {
  it('calls get_diagnosis with the diagnosis id', async () => {
    const diagnosis = {
      id: 'd1',
      patient_id: 'p1',
      icd10_code: 'F32.1',
      description: 'Major depressive episode',
      status: 'active',
      diagnosed_date: '2025-01-01',
      resolved_date: null,
      notes: null,
      created_at: '',
      updated_at: '',
    };
    mockInvoke.mockResolvedValueOnce(diagnosis);
    const result = await getDiagnosis('d1');
    expect(mockInvoke).toHaveBeenCalledWith('get_diagnosis', { id: 'd1' });
    expect(result).toEqual(diagnosis);
  });
});

describe('updateDiagnosis', () => {
  it('calls update_diagnosis with id and input', async () => {
    const updated = {
      id: 'd1',
      patient_id: 'p1',
      icd10_code: 'F33.0',
      description: 'Recurrent depressive',
      status: 'active',
      diagnosed_date: '2025-01-01',
      resolved_date: null,
      notes: null,
      created_at: '',
      updated_at: '',
    };
    mockInvoke.mockResolvedValueOnce(updated);
    const result = await updateDiagnosis('d1', { icd10_code: 'F33.0' });
    expect(mockInvoke).toHaveBeenCalledWith('update_diagnosis', {
      id: 'd1',
      input: { icd10_code: 'F33.0' },
    });
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
    const input = {
      patient_id: 'p1',
      substance: 'Sertraline',
      dosage: '50mg',
      frequency: 'once daily',
      start_date: '2025-02-01',
    };
    const created = {
      id: 'm1',
      ...input,
      end_date: null,
      notes: null,
      created_at: '',
      updated_at: '',
    };
    mockInvoke.mockResolvedValueOnce(created);
    const result = await createMedication(input);
    expect(mockInvoke).toHaveBeenCalledWith('create_medication', { input });
    expect(result).toEqual(created);
  });
});

describe('getMedication', () => {
  it('calls get_medication with the medication id', async () => {
    const medication = {
      id: 'm1',
      patient_id: 'p1',
      substance: 'Sertraline',
      dosage: '50mg',
      frequency: 'once daily',
      start_date: '2025-02-01',
      end_date: null,
      notes: null,
      created_at: '',
      updated_at: '',
    };
    mockInvoke.mockResolvedValueOnce(medication);
    const result = await getMedication('m1');
    expect(mockInvoke).toHaveBeenCalledWith('get_medication', { id: 'm1' });
    expect(result).toEqual(medication);
  });
});

describe('updateMedication', () => {
  it('calls update_medication with id and input', async () => {
    const updated = {
      id: 'm1',
      patient_id: 'p1',
      substance: 'Sertraline',
      dosage: '100mg',
      frequency: 'once daily',
      start_date: '2025-02-01',
      end_date: null,
      notes: null,
      created_at: '',
      updated_at: '',
    };
    mockInvoke.mockResolvedValueOnce(updated);
    const result = await updateMedication('m1', { dosage: '100mg' });
    expect(mockInvoke).toHaveBeenCalledWith('update_medication', {
      id: 'm1',
      input: { dosage: '100mg' },
    });
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
    const input = {
      patient_id: 'p1',
      report_type: 'discharge_letter',
      content: 'Report text',
      model_name: null,
      prompt_hash: null,
      session_ids: null,
    };
    const created = { id: 'r1', ...input, generated_at: '', created_at: '' };
    mockInvoke.mockResolvedValueOnce(created);
    const result = await createReport(input);
    expect(mockInvoke).toHaveBeenCalledWith('create_report', { input });
    expect(result).toEqual(created);
  });
});

describe('getReport', () => {
  it('calls get_report with the report id', async () => {
    const report = {
      id: 'r1',
      patient_id: 'p1',
      report_type: 'discharge_letter',
      content: 'Report text',
      generated_at: '',
      model_name: null,
      prompt_hash: null,
      session_ids: null,
      created_at: '',
    };
    mockInvoke.mockResolvedValueOnce(report);
    const result = await getReport('r1');
    expect(mockInvoke).toHaveBeenCalledWith('get_report', { id: 'r1' });
    expect(result).toEqual(report);
  });
});

describe('updateReport', () => {
  it('calls update_report with id and input', async () => {
    const updated = {
      id: 'r1',
      patient_id: 'p1',
      report_type: 'progress_note',
      content: 'Updated text',
      generated_at: '',
      model_name: null,
      prompt_hash: null,
      session_ids: null,
      created_at: '',
    };
    mockInvoke.mockResolvedValueOnce(updated);
    const result = await updateReport('r1', { content: 'Updated text' });
    expect(mockInvoke).toHaveBeenCalledWith('update_report', {
      id: 'r1',
      input: { content: 'Updated text' },
    });
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
    const info = {
      current_version: '0.1.0',
      latest_version: '0.2.0',
      update_available: true,
      body: 'Changelog',
      date: '2025-01-01',
    };
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

describe('exportFhirBundle', () => {
  it('calls export_fhir_bundle with patientId and returns JSON string', async () => {
    mockInvoke.mockResolvedValueOnce('{"resourceType":"Bundle"}');
    const result = await exportFhirBundle('patient1');
    expect(mockInvoke).toHaveBeenCalledWith('export_fhir_bundle', { patientId: 'patient1' });
    expect(result).toBe('{"resourceType":"Bundle"}');
  });
});

describe('createVaultBackup', () => {
  it('calls create_vault_backup and returns byte array', async () => {
    const bytes = [1, 2, 3];
    mockInvoke.mockResolvedValueOnce(bytes);
    const result = await createVaultBackup();
    expect(mockInvoke).toHaveBeenCalledWith('create_vault_backup');
    expect(result).toEqual(bytes);
  });
});

describe('restoreVaultBackup', () => {
  it('calls restore_vault_backup with encrypted bytes and returns BackupInfo', async () => {
    const info = {
      schema_version: 1,
      created_at: '2026-01-01T00:00:00Z',
      db_schema_version: 12,
      file_count: 5,
    };
    mockInvoke.mockResolvedValueOnce(info);
    const result = await restoreVaultBackup([1, 2, 3]);
    expect(mockInvoke).toHaveBeenCalledWith('restore_vault_backup', { encryptedBackup: [1, 2, 3] });
    expect(result).toEqual(info);
  });
});

describe('validateBackupArchive', () => {
  it('calls validate_backup_archive and returns BackupInfo', async () => {
    const info = {
      schema_version: 1,
      created_at: '2026-01-01T00:00:00Z',
      db_schema_version: 12,
      file_count: 3,
    };
    mockInvoke.mockResolvedValueOnce(info);
    const result = await validateBackupArchive([4, 5, 6]);
    expect(mockInvoke).toHaveBeenCalledWith('validate_backup_archive', {
      encryptedBackup: [4, 5, 6],
    });
    expect(result).toEqual(info);
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
    const turnResult = {
      session_id: 'cs1',
      final_answer: 'Here is the answer',
      tool_calls_made: [],
    };
    mockInvoke.mockResolvedValueOnce(turnResult);
    const result = await runAgentTurn('cs1', 'What medications does the patient take?');
    expect(mockInvoke).toHaveBeenCalledWith('run_agent_turn', {
      sessionId: 'cs1',
      userMessage: 'What medications does the patient take?',
      thinkingEffort: null,
    });
    expect(result).toEqual(turnResult);
  });
});

describe('createChatSession', () => {
  it('calls create_chat_session with scope and optional patientId', async () => {
    const session = {
      id: 'cs1',
      scope: 'patient',
      patient_id: 'p1',
      title: 'Chat',
      created_at: '',
      updated_at: '',
    };
    mockInvoke.mockResolvedValueOnce(session);
    const result = await createChatSession('patient', 'p1');
    expect(mockInvoke).toHaveBeenCalledWith('create_chat_session', {
      scope: 'patient',
      patientId: 'p1',
      title: undefined,
    });
    expect(result).toEqual(session);
  });

  it('calls create_chat_session with a custom title', async () => {
    const session = {
      id: 'cs2',
      scope: 'global',
      patient_id: null,
      title: 'My Chat',
      created_at: '',
      updated_at: '',
    };
    mockInvoke.mockResolvedValueOnce(session);
    await createChatSession('global', undefined, 'My Chat');
    expect(mockInvoke).toHaveBeenCalledWith('create_chat_session', {
      scope: 'global',
      patientId: undefined,
      title: 'My Chat',
    });
  });
});

describe('getOrCreatePatientChatSession', () => {
  it('calls get_or_create_patient_chat_session with patientId', async () => {
    const session = {
      id: 'cs1',
      scope: 'patient',
      patient_id: 'p1',
      title: 'Chat',
      created_at: '',
      updated_at: '',
    };
    mockInvoke.mockResolvedValueOnce(session);
    const result = await getOrCreatePatientChatSession('p1');
    expect(mockInvoke).toHaveBeenCalledWith('get_or_create_patient_chat_session', {
      patientId: 'p1',
    });
    expect(result).toEqual(session);
  });
});

describe('listChatSessions', () => {
  it('calls list_chat_sessions with scope', async () => {
    mockInvoke.mockResolvedValueOnce([]);
    await listChatSessions('global');
    expect(mockInvoke).toHaveBeenCalledWith('list_chat_sessions', {
      scope: 'global',
      patientId: undefined,
    });
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
      {
        id: 'msg1',
        session_id: 'cs1',
        role: 'user',
        content: 'Hello',
        tool_name: null,
        tool_args_json: null,
        tool_result_for: null,
        created_at: '',
      },
    ];
    mockInvoke.mockResolvedValueOnce(messages);
    const result = await getChatMessages('cs1');
    expect(mockInvoke).toHaveBeenCalledWith('get_chat_messages', { sessionId: 'cs1' });
    expect(result).toEqual(messages);
  });
});

describe('renameChatSession', () => {
  it('calls rename_chat_session with sessionId and title', async () => {
    const session = {
      id: 'cs1',
      scope: 'global',
      patient_id: null,
      title: 'New Title',
      created_at: '',
      updated_at: '',
    };
    mockInvoke.mockResolvedValueOnce(session);
    const result = await renameChatSession('cs1', 'New Title');
    expect(mockInvoke).toHaveBeenCalledWith('rename_chat_session', {
      sessionId: 'cs1',
      title: 'New Title',
    });
    expect(result).toEqual(session);
  });
});

// ---------------------------------------------------------------------------
// Literature API
// ---------------------------------------------------------------------------

describe('uploadLiterature', () => {
  it('calls upload_literature with the correct parameters', async () => {
    const literature = {
      id: 'lit1',
      filename: 'paper.pdf',
      vault_path: '/vault/lit1',
      mime_type: 'application/pdf',
      size_bytes: 2048,
      description: 'A paper',
      created_at: '',
      updated_at: '',
      chunk_count: 0,
    };
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
    expect(mockInvoke).toHaveBeenCalledWith(
      'upload_literature',
      expect.objectContaining({ description: null })
    );
  });
});

describe('getLiteratureById', () => {
  it('calls get_literature_by_id with the document id', async () => {
    const literature = {
      id: 'lit1',
      filename: 'paper.pdf',
      vault_path: '',
      mime_type: 'application/pdf',
      size_bytes: 0,
      description: null,
      created_at: '',
      updated_at: '',
      chunk_count: 5,
    };
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
    const updated = {
      id: 'lit1',
      filename: 'paper.pdf',
      vault_path: '',
      mime_type: 'application/pdf',
      size_bytes: 0,
      description: 'Updated desc',
      created_at: '',
      updated_at: '',
      chunk_count: 5,
    };
    mockInvoke.mockResolvedValueOnce(updated);
    const result = await updateLiteratureMetadata('lit1', 'Updated desc');
    expect(mockInvoke).toHaveBeenCalledWith('update_literature_metadata', {
      id: 'lit1',
      description: 'Updated desc',
    });
    expect(result).toEqual(updated);
  });

  it('passes null description to clear it', async () => {
    mockInvoke.mockResolvedValueOnce({});
    await updateLiteratureMetadata('lit1', null);
    expect(mockInvoke).toHaveBeenCalledWith('update_literature_metadata', {
      id: 'lit1',
      description: null,
    });
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
    const chunks = [
      {
        chunk_id: 'c1',
        literature_id: 'lit1',
        filename: 'paper.pdf',
        chunk_index: 0,
        content: 'relevant text',
        similarity: 0.9,
      },
    ];
    mockInvoke.mockResolvedValueOnce(chunks);
    const result = await searchLiterature('relevant query');
    expect(mockInvoke).toHaveBeenCalledWith('search_literature', {
      query: 'relevant query',
      limit: 5,
    });
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
    const chunks = [
      {
        id: 'c1',
        file_id: null,
        literature_id: 'lit1',
        chunk_index: 0,
        content: 'text',
        word_count: 50,
        created_at: '',
      },
    ];
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

// ---------------------------------------------------------------------------
// Session Summary Generation
// ---------------------------------------------------------------------------

describe('generateSessionSummary', () => {
  it('calls generate_session_summary with patientContext and sessionNotes', async () => {
    const summary = 'Generated clinical summary';
    mockInvoke.mockResolvedValueOnce(summary);
    const result = await generateSessionSummary(
      'Patient: John Doe, DOB: 1980-01-01',
      'Patient reports feeling anxious'
    );
    expect(mockInvoke).toHaveBeenCalledWith('generate_session_summary', {
      patientContext: 'Patient: John Doe, DOB: 1980-01-01',
      sessionNotes: 'Patient reports feeling anxious',
      systemPrompt: undefined,
      thinkingEffort: null,
    });
    expect(result).toBe(summary);
  });

  it('calls generate_session_summary with optional systemPrompt', async () => {
    const summary = 'Generated clinical summary';
    mockInvoke.mockResolvedValueOnce(summary);
    await generateSessionSummary(
      'Patient: John Doe',
      'Session notes',
      'Use professional psychiatric terminology'
    );
    expect(mockInvoke).toHaveBeenCalledWith('generate_session_summary', {
      patientContext: 'Patient: John Doe',
      sessionNotes: 'Session notes',
      systemPrompt: 'Use professional psychiatric terminology',
      thinkingEffort: null,
    });
  });

  it('handles empty session notes', async () => {
    const summary = 'No notes provided';
    mockInvoke.mockResolvedValueOnce(summary);
    const result = await generateSessionSummary('Patient: John Doe', '');
    expect(mockInvoke).toHaveBeenCalledWith('generate_session_summary', {
      patientContext: 'Patient: John Doe',
      sessionNotes: '',
      systemPrompt: undefined,
      thinkingEffort: null,
    });
    expect(result).toBe(summary);
  });
});

// ---------------------------------------------------------------------------
// Model Management API
// ---------------------------------------------------------------------------

describe('listModels', () => {
  it('calls list_models and returns array of ModelInfo', async () => {
    const models = [
      {
        id: 'model1',
        name: 'Phi-4 Mini Q4_K_M',
        filename: 'Phi-4-mini-instruct-Q4_K_M.gguf',
        sha256: 'abc123',
        size_bytes: 3000000000,
        downloaded_at: '2025-01-01T00:00:00Z',
        last_used: null,
        is_default: true,
        is_loaded: false,
        exists_on_disk: true,
        quantization_promotion: null,
      },
    ];
    mockInvoke.mockResolvedValueOnce(models);
    const result = await listModels();
    expect(mockInvoke).toHaveBeenCalledWith('list_models');
    expect(result).toEqual(models);
  });
});

describe('getModelInfo', () => {
  it('calls get_model_info with modelId', async () => {
    const model = {
      id: 'model1',
      name: 'Phi-4 Mini Q4_K_M',
      filename: 'Phi-4-mini-instruct-Q4_K_M.gguf',
      sha256: 'abc123',
      size_bytes: 3000000000,
      downloaded_at: '2025-01-01T00:00:00Z',
      last_used: null,
      is_default: true,
      is_loaded: false,
      exists_on_disk: true,
      quantization_promotion: null,
    };
    mockInvoke.mockResolvedValueOnce(model);
    const result = await getModelInfo('model1');
    expect(mockInvoke).toHaveBeenCalledWith('get_model_info', { modelId: 'model1' });
    expect(result).toEqual(model);
  });
});

describe('downloadAndRegisterModel', () => {
  it('calls download_and_register_model with the model payload', async () => {
    const model: ModelChoice = {
      name: 'Phi-4 Mini Q4_K_M',
      filename: 'Phi-4-mini-instruct-Q4_K_M.gguf',
      size_bytes: 3 * 1024 ** 3,
      reason: 'Unter 16 GB RAM: Phi-4 Mini für minimale Ressourcen',
    };
    const registered = {
      id: 'model1',
      name: model.name,
      filename: model.filename,
      sha256: 'abc123def456',
      size_bytes: model.size_bytes,
      downloaded_at: '2025-01-01T00:00:00Z',
      last_used: null,
      is_default: false,
    };
    mockInvoke.mockResolvedValueOnce(registered);
    const result = await downloadAndRegisterModel(model);
    expect(mockInvoke).toHaveBeenCalledWith('download_and_register_model', { model });
    expect(result).toEqual(registered);
  });

  it('propagates invoke errors on download failure', async () => {
    const model: ModelChoice = {
      name: 'Phi-4 Mini Q4_K_M',
      filename: 'Phi-4-mini-instruct-Q4_K_M.gguf',
      size_bytes: 3 * 1024 ** 3,
      reason: '',
    };
    mockInvoke.mockRejectedValueOnce({
      code: 'NETWORK_ERROR',
      message: 'Failed to download model',
    });
    await expect(downloadAndRegisterModel(model)).rejects.toMatchObject({
      code: 'NETWORK_ERROR',
    });
  });
});

describe('inspectPromotedModel', () => {
  it('calls inspect_promoted_model with the selected record and optional GGUF', async () => {
    const preview = {
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
    mockInvoke.mockResolvedValueOnce(preview);
    const result = await inspectPromotedModel('/tmp/clinical-mix.promotion.json');
    expect(mockInvoke).toHaveBeenCalledWith('inspect_promoted_model', {
      promotionPath: '/tmp/clinical-mix.promotion.json',
      artifactPath: null,
    });
    expect(result).toEqual(preview);
  });
});

describe('importPromotedModel', () => {
  it('calls import_promoted_model with the selected promotion record', async () => {
    const registered = {
      id: 'clinical-mix',
      name: 'RamDoc clinical mix',
      filename: 'clinical-mix.gguf',
      sha256: 'abc123def456',
      size_bytes: 4200000000,
      downloaded_at: '2026-08-17T12:00:00Z',
      last_used: null,
      is_default: false,
    };
    mockInvoke.mockResolvedValueOnce(registered);
    const result = await importPromotedModel(
      '/tmp/clinical-mix.promotion.json',
      '/tmp/clinical-mix.gguf'
    );
    expect(mockInvoke).toHaveBeenCalledWith('import_promoted_model', {
      promotionPath: '/tmp/clinical-mix.promotion.json',
      artifactPath: '/tmp/clinical-mix.gguf',
    });
    expect(result).toEqual(registered);
  });
});

describe('deleteModel', () => {
  it('calls delete_model with modelId', async () => {
    mockInvoke.mockResolvedValueOnce(undefined);
    await deleteModel('model1');
    expect(mockInvoke).toHaveBeenCalledWith('delete_model', { modelId: 'model1' });
  });

  it('propagates error when deleting loaded model', async () => {
    mockInvoke.mockRejectedValueOnce({
      code: 'VALIDATION_ERROR',
      message: 'Cannot delete currently loaded model',
    });
    await expect(deleteModel('model1')).rejects.toMatchObject({
      code: 'VALIDATION_ERROR',
    });
  });
});

describe('setDefaultModel', () => {
  it('calls set_default_model with modelId', async () => {
    mockInvoke.mockResolvedValueOnce(undefined);
    await setDefaultModel('model1');
    expect(mockInvoke).toHaveBeenCalledWith('set_default_model', { modelId: 'model1' });
  });
});

describe('getDefaultModel', () => {
  it('calls get_default_model and returns the default model', async () => {
    const model = {
      id: 'model1',
      name: 'Phi-4 Mini Q4_K_M',
      filename: 'Phi-4-mini-instruct-Q4_K_M.gguf',
      sha256: 'abc123',
      size_bytes: 3000000000,
      downloaded_at: '2025-01-01T00:00:00Z',
      last_used: null,
      is_default: true,
    };
    mockInvoke.mockResolvedValueOnce(model);
    const result = await getDefaultModel();
    expect(mockInvoke).toHaveBeenCalledWith('get_default_model');
    expect(result).toEqual(model);
  });

  it('returns null when no default model is set', async () => {
    mockInvoke.mockResolvedValueOnce(null);
    const result = await getDefaultModel();
    expect(mockInvoke).toHaveBeenCalledWith('get_default_model');
    expect(result).toBeNull();
  });
});

describe('writing model lifecycle API', () => {
  it('asks the backend to load the durable writing-model selection', async () => {
    mockInvoke.mockResolvedValueOnce({ is_loaded: true });
    await ensureWritingModelLoaded();
    expect(mockInvoke).toHaveBeenCalledWith('ensure_writing_model_loaded');
  });

  it('explicitly unloads the writing model', async () => {
    mockInvoke.mockResolvedValueOnce(undefined);
    await unloadModel();
    expect(mockInvoke).toHaveBeenCalledWith('unload_model');
  });

  it('reads managed tool-router diagnostics', async () => {
    mockInvoke.mockResolvedValueOnce({ role: 'tool_router', mode: 'managed' });
    await getRouterDiagnostics();
    expect(mockInvoke).toHaveBeenCalledWith('get_router_diagnostics');
  });
});

// ---------------------------------------------------------------------------
// Letters
// ---------------------------------------------------------------------------

const LETTER: Letter = {
  id: 'letter1',
  patient_id: 'patient1',
  letter_type: 'referral',
  template_language: 'de',
  recipient_name: 'Dr. Müller',
  recipient_address: 'Hauptstrasse 1, Bern',
  subject: 'Referral for Patient',
  content: 'Dear Dr. Müller...',
  status: 'draft',
  model_name: null,
  session_ids: null,
  finalized_at: null,
  sent_at: null,
  created_at: '2026-01-01T00:00:00Z',
  updated_at: '2026-01-01T00:00:00Z',
};

describe('createLetter', () => {
  it('calls create_letter and returns the created letter', async () => {
    mockInvoke.mockResolvedValueOnce(LETTER);
    const input = {
      patient_id: 'patient1',
      letter_type: 'referral' as const,
      template_language: 'de' as const,
      subject: 'Referral for Patient',
      content: 'Dear Dr. Müller...',
    };
    const result = await createLetter(input);
    expect(mockInvoke).toHaveBeenCalledWith('create_letter', { input });
    expect(result).toEqual(LETTER);
  });
});

describe('getLetter', () => {
  it('calls get_letter and returns the letter', async () => {
    mockInvoke.mockResolvedValueOnce(LETTER);
    const result = await getLetter('letter1');
    expect(mockInvoke).toHaveBeenCalledWith('get_letter', { id: 'letter1' });
    expect(result).toEqual(LETTER);
  });
});

describe('listLetters', () => {
  it('calls list_letters and returns a list', async () => {
    mockInvoke.mockResolvedValueOnce([LETTER]);
    const result = await listLetters('patient1', 10, 0);
    expect(mockInvoke).toHaveBeenCalledWith('list_letters', {
      patientId: 'patient1',
      limit: 10,
      offset: 0,
    });
    expect(result).toEqual([LETTER]);
  });
});

describe('updateLetter', () => {
  it('calls update_letter and returns updated letter', async () => {
    const updated = { ...LETTER, subject: 'Updated Subject' };
    mockInvoke.mockResolvedValueOnce(updated);
    const result = await updateLetter('letter1', { subject: 'Updated Subject' });
    expect(mockInvoke).toHaveBeenCalledWith('update_letter', {
      id: 'letter1',
      input: { subject: 'Updated Subject' },
    });
    expect(result.subject).toBe('Updated Subject');
  });
});

describe('deleteLetter', () => {
  it('calls delete_letter', async () => {
    mockInvoke.mockResolvedValueOnce(undefined);
    await deleteLetter('letter1');
    expect(mockInvoke).toHaveBeenCalledWith('delete_letter', { id: 'letter1' });
  });
});

describe('markLetterAsFinalized', () => {
  it('calls mark_letter_as_finalized and returns letter', async () => {
    const finalized = { ...LETTER, status: 'finalized' as const };
    mockInvoke.mockResolvedValueOnce(finalized);
    const result = await markLetterAsFinalized('letter1');
    expect(mockInvoke).toHaveBeenCalledWith('mark_letter_as_finalized', { id: 'letter1' });
    expect(result.status).toBe('finalized');
  });
});

describe('markLetterAsSent', () => {
  it('calls mark_letter_as_sent and returns letter', async () => {
    const sent = { ...LETTER, status: 'sent' as const };
    mockInvoke.mockResolvedValueOnce(sent);
    const result = await markLetterAsSent('letter1');
    expect(mockInvoke).toHaveBeenCalledWith('mark_letter_as_sent', { id: 'letter1' });
    expect(result.status).toBe('sent');
  });
});

describe('generateLetter', () => {
  it('calls generate_letter and returns generated content', async () => {
    mockInvoke.mockResolvedValueOnce('Dear Dr. Müller, I am writing to refer...');
    const result = await generateLetter(
      'referral',
      'de',
      'Patient context',
      'Clinical summary',
      'Dr. Müller'
    );
    expect(mockInvoke).toHaveBeenCalledWith('generate_letter', {
      letterType: 'referral',
      language: 'de',
      patientContext: 'Patient context',
      clinicalSummary: 'Clinical summary',
      recipientName: 'Dr. Müller',
      systemPrompt: undefined,
      thinkingEffort: null,
    });
    expect(result).toBe('Dear Dr. Müller, I am writing to refer...');
  });
});

// ---------------------------------------------------------------------------
// Email (missing coverage)
// ---------------------------------------------------------------------------

const EMAIL = {
  id: 'email1',
  patient_id: 'patient1',
  recipient_email: 'dr.mueller@example.com',
  subject: 'Patient Referral',
  body: 'Dear Dr. Müller...',
  status: 'draft',
  sent_at: null,
  created_at: '2026-01-01T00:00:00Z',
  updated_at: '2026-01-01T00:00:00Z',
};

describe('createEmail', () => {
  it('calls create_email and returns created email', async () => {
    mockInvoke.mockResolvedValueOnce(EMAIL);
    const input = {
      patient_id: 'patient1',
      recipient_email: 'dr@example.com',
      subject: 'Test',
      body: 'Body',
    };
    const result = await createEmail(input);
    expect(mockInvoke).toHaveBeenCalledWith('create_email', { input });
    expect(result).toEqual(EMAIL);
  });
});

describe('getEmail', () => {
  it('calls get_email and returns email', async () => {
    mockInvoke.mockResolvedValueOnce(EMAIL);
    const result = await getEmail('email1');
    expect(mockInvoke).toHaveBeenCalledWith('get_email', { id: 'email1' });
    expect(result).toEqual(EMAIL);
  });
});

describe('listEmails', () => {
  it('calls list_emails and returns list', async () => {
    mockInvoke.mockResolvedValueOnce([EMAIL]);
    const result = await listEmails('patient1', 10, 0);
    expect(mockInvoke).toHaveBeenCalledWith('list_emails', {
      patientId: 'patient1',
      limit: 10,
      offset: 0,
    });
    expect(result).toEqual([EMAIL]);
  });
});

describe('updateEmail', () => {
  it('calls update_email and returns updated email', async () => {
    const updated = { ...EMAIL, subject: 'Updated' };
    mockInvoke.mockResolvedValueOnce(updated);
    const result = await updateEmail('email1', { subject: 'Updated' });
    expect(mockInvoke).toHaveBeenCalledWith('update_email', {
      id: 'email1',
      input: { subject: 'Updated' },
    });
    expect(result.subject).toBe('Updated');
  });
});

describe('deleteEmail', () => {
  it('calls delete_email with id', async () => {
    mockInvoke.mockResolvedValueOnce(undefined);
    await deleteEmail('email1');
    expect(mockInvoke).toHaveBeenCalledWith('delete_email', { id: 'email1' });
  });
});

describe('markEmailAsSent', () => {
  it('calls mark_email_as_sent and returns updated email', async () => {
    const sent = { ...EMAIL, status: 'sent', sent_at: '2026-03-27T09:00:00Z' };
    mockInvoke.mockResolvedValueOnce(sent);
    const result = await markEmailAsSent('email1');
    expect(mockInvoke).toHaveBeenCalledWith('mark_email_as_sent', { id: 'email1' });
    expect(result.status).toBe('sent');
  });
});

describe('exportPatientPdf', () => {
  it('calls export_patient_pdf with patientId and returns byte array', async () => {
    const bytes = [37, 80, 68, 70];
    mockInvoke.mockResolvedValueOnce(bytes);
    const result = await exportPatientPdf('patient1');
    expect(mockInvoke).toHaveBeenCalledWith('export_patient_pdf', { patientId: 'patient1' });
    expect(result).toEqual(bytes);
  });
});

describe('processFile', () => {
  it('calls process_file with fileId', async () => {
    mockInvoke.mockResolvedValueOnce(undefined);
    await processFile('file1');
    expect(mockInvoke).toHaveBeenCalledWith('process_file', { fileId: 'file1' });
  });
});

// ---------------------------------------------------------------------------
// Treatment Plans
// ---------------------------------------------------------------------------

const TREATMENT_PLAN = {
  id: 'plan1',
  patient_id: 'patient1',
  title: 'CBT Plan',
  description: null,
  start_date: '2026-01-01',
  end_date: null,
  status: 'active',
  created_at: '2026-01-01T00:00:00Z',
  updated_at: '2026-01-01T00:00:00Z',
};

describe('createTreatmentPlan', () => {
  it('calls create_treatment_plan and returns plan', async () => {
    mockInvoke.mockResolvedValueOnce(TREATMENT_PLAN);
    const result = await createTreatmentPlan({
      patient_id: 'patient1',
      title: 'CBT Plan',
      start_date: '2026-01-01',
    });
    expect(mockInvoke).toHaveBeenCalledWith('create_treatment_plan', {
      input: { patient_id: 'patient1', title: 'CBT Plan', start_date: '2026-01-01' },
    });
    expect(result).toEqual(TREATMENT_PLAN);
  });
});

describe('getTreatmentPlan', () => {
  it('calls get_treatment_plan and returns plan', async () => {
    mockInvoke.mockResolvedValueOnce(TREATMENT_PLAN);
    const result = await getTreatmentPlan('plan1');
    expect(mockInvoke).toHaveBeenCalledWith('get_treatment_plan', { id: 'plan1' });
    expect(result).toEqual(TREATMENT_PLAN);
  });
});

describe('listTreatmentPlansForPatient', () => {
  it('calls list_treatment_plans_for_patient and returns list', async () => {
    mockInvoke.mockResolvedValueOnce([TREATMENT_PLAN]);
    const result = await listTreatmentPlansForPatient('patient1', 10, 0);
    expect(mockInvoke).toHaveBeenCalledWith('list_treatment_plans_for_patient', {
      patientId: 'patient1',
      limit: 10,
      offset: 0,
    });
    expect(result).toEqual([TREATMENT_PLAN]);
  });
});

describe('updateTreatmentPlan', () => {
  it('calls update_treatment_plan and returns updated plan', async () => {
    mockInvoke.mockResolvedValueOnce({ ...TREATMENT_PLAN, title: 'Updated' });
    const result = await updateTreatmentPlan('plan1', { title: 'Updated' });
    expect(mockInvoke).toHaveBeenCalledWith('update_treatment_plan', {
      id: 'plan1',
      input: { title: 'Updated' },
    });
    expect(result.title).toBe('Updated');
  });
});

describe('deleteTreatmentPlan', () => {
  it('calls delete_treatment_plan', async () => {
    mockInvoke.mockResolvedValueOnce(undefined);
    await deleteTreatmentPlan('plan1');
    expect(mockInvoke).toHaveBeenCalledWith('delete_treatment_plan', { id: 'plan1' });
  });
});

// ---------------------------------------------------------------------------
// Treatment Goals
// ---------------------------------------------------------------------------

const TREATMENT_GOAL = {
  id: 'goal1',
  treatment_plan_id: 'plan1',
  description: 'Reduce anxiety',
  target_date: null,
  status: 'active',
  sort_order: 0,
  created_at: '2026-01-01T00:00:00Z',
  updated_at: '2026-01-01T00:00:00Z',
};

describe('createTreatmentGoal', () => {
  it('calls create_treatment_goal and returns goal', async () => {
    mockInvoke.mockResolvedValueOnce(TREATMENT_GOAL);
    const result = await createTreatmentGoal({
      treatment_plan_id: 'plan1',
      description: 'Reduce anxiety',
    });
    expect(mockInvoke).toHaveBeenCalledWith('create_treatment_goal', {
      input: { treatment_plan_id: 'plan1', description: 'Reduce anxiety' },
    });
    expect(result).toEqual(TREATMENT_GOAL);
  });
});

describe('getTreatmentGoal', () => {
  it('calls get_treatment_goal', async () => {
    mockInvoke.mockResolvedValueOnce(TREATMENT_GOAL);
    const result = await getTreatmentGoal('goal1');
    expect(mockInvoke).toHaveBeenCalledWith('get_treatment_goal', { id: 'goal1' });
    expect(result).toEqual(TREATMENT_GOAL);
  });
});

describe('listTreatmentGoalsForPlan', () => {
  it('calls list_treatment_goals_for_plan', async () => {
    mockInvoke.mockResolvedValueOnce([TREATMENT_GOAL]);
    const result = await listTreatmentGoalsForPlan('plan1', 10, 0);
    expect(mockInvoke).toHaveBeenCalledWith('list_treatment_goals_for_plan', {
      planId: 'plan1',
      limit: 10,
      offset: 0,
    });
    expect(result).toEqual([TREATMENT_GOAL]);
  });
});

describe('updateTreatmentGoal', () => {
  it('calls update_treatment_goal', async () => {
    mockInvoke.mockResolvedValueOnce({ ...TREATMENT_GOAL, description: 'Updated' });
    const result = await updateTreatmentGoal('goal1', { description: 'Updated' });
    expect(mockInvoke).toHaveBeenCalledWith('update_treatment_goal', {
      id: 'goal1',
      input: { description: 'Updated' },
    });
    expect(result.description).toBe('Updated');
  });
});

describe('deleteTreatmentGoal', () => {
  it('calls delete_treatment_goal', async () => {
    mockInvoke.mockResolvedValueOnce(undefined);
    await deleteTreatmentGoal('goal1');
    expect(mockInvoke).toHaveBeenCalledWith('delete_treatment_goal', { id: 'goal1' });
  });
});

// ---------------------------------------------------------------------------
// Treatment Interventions
// ---------------------------------------------------------------------------

const INTERVENTION = {
  id: 'int1',
  treatment_plan_id: 'plan1',
  type: 'exposure',
  description: 'Gradual exposure therapy',
  frequency: 'weekly',
  created_at: '2026-01-01T00:00:00Z',
  updated_at: '2026-01-01T00:00:00Z',
};

describe('createTreatmentIntervention', () => {
  it('calls create_treatment_intervention', async () => {
    mockInvoke.mockResolvedValueOnce(INTERVENTION);
    const result = await createTreatmentIntervention({
      treatment_plan_id: 'plan1',
      type: 'exposure',
      description: 'Gradual exposure therapy',
    });
    expect(mockInvoke).toHaveBeenCalledWith('create_treatment_intervention', {
      input: {
        treatment_plan_id: 'plan1',
        type: 'exposure',
        description: 'Gradual exposure therapy',
      },
    });
    expect(result).toEqual(INTERVENTION);
  });
});

describe('getTreatmentIntervention', () => {
  it('calls get_treatment_intervention', async () => {
    mockInvoke.mockResolvedValueOnce(INTERVENTION);
    const result = await getTreatmentIntervention('int1');
    expect(mockInvoke).toHaveBeenCalledWith('get_treatment_intervention', { id: 'int1' });
    expect(result).toEqual(INTERVENTION);
  });
});

describe('listTreatmentInterventionsForPlan', () => {
  it('calls list_treatment_interventions_for_plan', async () => {
    mockInvoke.mockResolvedValueOnce([INTERVENTION]);
    const result = await listTreatmentInterventionsForPlan('plan1', 10, 0);
    expect(mockInvoke).toHaveBeenCalledWith('list_treatment_interventions_for_plan', {
      planId: 'plan1',
      limit: 10,
      offset: 0,
    });
    expect(result).toEqual([INTERVENTION]);
  });
});

describe('updateTreatmentIntervention', () => {
  it('calls update_treatment_intervention', async () => {
    mockInvoke.mockResolvedValueOnce({ ...INTERVENTION, type: 'cbt' });
    const result = await updateTreatmentIntervention('int1', { type: 'cbt' });
    expect(mockInvoke).toHaveBeenCalledWith('update_treatment_intervention', {
      id: 'int1',
      input: { type: 'cbt' },
    });
    expect(result.type).toBe('cbt');
  });
});

describe('deleteTreatmentIntervention', () => {
  it('calls delete_treatment_intervention', async () => {
    mockInvoke.mockResolvedValueOnce(undefined);
    await deleteTreatmentIntervention('int1');
    expect(mockInvoke).toHaveBeenCalledWith('delete_treatment_intervention', { id: 'int1' });
  });
});

// ---------------------------------------------------------------------------
// Outcome Scores
// ---------------------------------------------------------------------------

const OUTCOME_SCORE = {
  id: 'score1',
  session_id: 'sess1',
  scale_type: 'PHQ9',
  score: 12,
  interpretation: 'moderate',
  subscores: null,
  administered_at: '2026-01-01',
  notes: null,
  created_at: '2026-01-01T00:00:00Z',
  updated_at: '2026-01-01T00:00:00Z',
};

describe('createOutcomeScore', () => {
  it('calls create_outcome_score and returns score', async () => {
    mockInvoke.mockResolvedValueOnce(OUTCOME_SCORE);
    const result = await createOutcomeScore({
      session_id: 'sess1',
      scale_type: 'PHQ9',
      score: 12,
      administered_at: '2026-01-01',
    });
    expect(mockInvoke).toHaveBeenCalledWith('create_outcome_score', {
      input: { session_id: 'sess1', scale_type: 'PHQ9', score: 12, administered_at: '2026-01-01' },
    });
    expect(result).toEqual(OUTCOME_SCORE);
  });
});

describe('getOutcomeScore', () => {
  it('calls get_outcome_score', async () => {
    mockInvoke.mockResolvedValueOnce(OUTCOME_SCORE);
    const result = await getOutcomeScore('score1');
    expect(mockInvoke).toHaveBeenCalledWith('get_outcome_score', { id: 'score1' });
    expect(result).toEqual(OUTCOME_SCORE);
  });
});

describe('listScoresForSession', () => {
  it('calls list_scores_for_session', async () => {
    mockInvoke.mockResolvedValueOnce([OUTCOME_SCORE]);
    const result = await listScoresForSession('sess1', 10, 0);
    expect(mockInvoke).toHaveBeenCalledWith('list_scores_for_session', {
      sessionId: 'sess1',
      limit: 10,
      offset: 0,
    });
    expect(result).toEqual([OUTCOME_SCORE]);
  });
});

describe('listScoresByScale', () => {
  it('calls list_scores_by_scale', async () => {
    mockInvoke.mockResolvedValueOnce([OUTCOME_SCORE]);
    const result = await listScoresByScale('PHQ9', 10, 0);
    expect(mockInvoke).toHaveBeenCalledWith('list_scores_by_scale', {
      scaleType: 'PHQ9',
      limit: 10,
      offset: 0,
    });
    expect(result).toEqual([OUTCOME_SCORE]);
  });
});

describe('listScoresForPatient', () => {
  it('calls list_scores_for_patient', async () => {
    mockInvoke.mockResolvedValueOnce([OUTCOME_SCORE]);
    const result = await listScoresForPatient('patient1', 10, 0);
    expect(mockInvoke).toHaveBeenCalledWith('list_scores_for_patient', {
      patientId: 'patient1',
      limit: 10,
      offset: 0,
    });
    expect(result).toEqual([OUTCOME_SCORE]);
  });
});

describe('updateOutcomeScore', () => {
  it('calls update_outcome_score', async () => {
    mockInvoke.mockResolvedValueOnce({ ...OUTCOME_SCORE, score: 8 });
    const result = await updateOutcomeScore('score1', { score: 8 });
    expect(mockInvoke).toHaveBeenCalledWith('update_outcome_score', {
      id: 'score1',
      input: { score: 8 },
    });
    expect(result.score).toBe(8);
  });
});

describe('deleteOutcomeScore', () => {
  it('calls delete_outcome_score', async () => {
    mockInvoke.mockResolvedValueOnce(undefined);
    await deleteOutcomeScore('score1');
    expect(mockInvoke).toHaveBeenCalledWith('delete_outcome_score', { id: 'score1' });
  });
});

// ---------------------------------------------------------------------------
// Medication Reference
// ---------------------------------------------------------------------------

describe('compareMedications', () => {
  it('calls compare_medications with currentId and replacementId', async () => {
    const detail = {
      id: 'sub1',
      name_de: 'Sertralin',
      atc_code: 'N06AB06',
      trade_names: ['Zoloft'],
      indication: 'Depression',
      side_effects: 'Nausea',
      contraindications: null,
      source_version: null,
    };
    const comparison: MedicationComparison = {
      current_medication: detail,
      replacement_medication: { ...detail, id: 'sub2', name_de: 'Fluoxetin' },
    };
    mockInvoke.mockResolvedValueOnce(comparison);
    const result = await compareMedications('sub1', 'sub2');
    expect(mockInvoke).toHaveBeenCalledWith('compare_medications', {
      currentId: 'sub1',
      replacementId: 'sub2',
    });
    expect(result).toEqual(comparison);
  });
});

// Practice Settings
// ---------------------------------------------------------------------------

const PRACTICE_SETTINGS: PracticeSettings = {
  practice_name: 'Test Practice',
  practice_address: '123 Main St',
  practice_phone: '+41 44 000 00 00',
  practice_email: 'test@example.com',
  therapist_name: 'Dr. Test',
  zsr_number: 'ZSR123',
  canton: 'ZH',
  clinical_specialty: 'Clinical Psychology',
  language_preference: 'de',
  onboarding_completed: false,
};

describe('getSettings', () => {
  it('calls get_settings and returns practice settings', async () => {
    mockInvoke.mockResolvedValueOnce(PRACTICE_SETTINGS);
    const result = await getSettings();
    expect(mockInvoke).toHaveBeenCalledWith('get_settings');
    expect(result).toEqual(PRACTICE_SETTINGS);
  });

  it('propagates invoke errors', async () => {
    const err = { code: 'DB_ERROR', message: 'Database error', ref: 'REF123' };
    mockInvoke.mockRejectedValueOnce(err);
    await expect(getSettings()).rejects.toEqual(err);
  });
});

describe('updateSettings', () => {
  it('calls update_settings with the full settings payload', async () => {
    mockInvoke.mockResolvedValueOnce(undefined);
    await updateSettings(PRACTICE_SETTINGS);
    expect(mockInvoke).toHaveBeenCalledWith('update_settings', { settings: PRACTICE_SETTINGS });
  });

  it('propagates invoke errors', async () => {
    const err = { code: 'DB_ERROR', message: 'Failed to update', ref: 'REF456' };
    mockInvoke.mockRejectedValueOnce(err);
    await expect(updateSettings(PRACTICE_SETTINGS)).rejects.toEqual(err);
  });
});

describe('completeOnboarding', () => {
  it('calls complete_onboarding', async () => {
    mockInvoke.mockResolvedValueOnce(undefined);
    await completeOnboarding();
    expect(mockInvoke).toHaveBeenCalledWith('complete_onboarding');
  });

  it('propagates invoke errors', async () => {
    const err = { code: 'DB_ERROR', message: 'Failed to complete', ref: 'REF789' };
    mockInvoke.mockRejectedValueOnce(err);
    await expect(completeOnboarding()).rejects.toEqual(err);
  });
});
// Patient History RAG Query
// ---------------------------------------------------------------------------

const evidenceManifest = {
  manifest_id: 'm1',
  patient_id: 'p1',
  patient_revision: 'p1:abc',
  question_sha256: 'deadbeef',
  created_at: '2026-03-04T10:00:00Z',
  token_budget: 11520,
  token_counter: 'model-tokenizer',
  prompt_tokens: 8123,
  tier_tokens: { structured: 2000, hot: 5000, cold: 1000, pointers: 100, overhead: 23 },
  entries: [
    {
      citation: 'E1',
      unit_id: 'u1',
      patient_id: 'p1',
      record_kind: 'session' as const,
      record_id: 's1',
      section: 'notes',
      revision: 'r1:abc',
      tier: 'hot' as const,
      label: 'Sitzung 2026-03-04 (Verlaufsgespräch)',
      occurred_at: '2026-03-04',
      char_start: 0,
      char_end: 120,
      text_sha256: 'abcd1234',
      tokens: 60,
      prompt_token_start: 100,
      prompt_token_end: 160,
      protected_spans: [{ kind: 'dose' as const, start: 10, end: 16 }],
      selection: {
        lexical_rank: 1,
        lexical_bm25: -1.8,
        semantic_rank: null,
        semantic_similarity: null,
        fused_score: 0.016,
        recency_boost: 0.02,
        matched_terms: ['medikation'],
        document_neighbor_of: [],
        temporal_neighbor_of: [],
        structured_truth: false,
      },
      selection_reasons: ['lexical rank 1 (bm25 -1.800)'],
    },
  ],
  omitted: [],
  protected_spans_retained: [{ kind: 'dose' as const, count: 1 }],
  retrieval: {
    index_units: 42,
    question_terms: ['medikation'],
    lexical_hits: 3,
    semantic_hits: 0,
    semantic_available: false,
    document_neighbors_added: 1,
    temporal_neighbors_added: 2,
    temporal_question: true,
    candidates: 6,
  },
  index: {
    sources_scanned: 20,
    sources_reindexed: 0,
    sources_removed: 0,
    units_inserted: 0,
    units_removed: 0,
    stale_chunks_removed: 0,
    units_total: 42,
    units_missing_embeddings: 0,
  },
};

describe('queryPatientHistory', () => {
  it('calls query_patient_history with patientId and question', async () => {
    const response = {
      answer: 'Die Medikation wurde am 2025-03-15 geändert [E1].',
      manifest: evidenceManifest,
      audit: {
        manifest_id: 'm1',
        citations: [
          {
            citation: 'E1',
            in_manifest: true,
            unit_id: 'u1',
            label: 'Sitzung 2026-03-04 (Verlaufsgespräch)',
            occurred_at: '2026-03-04',
            traceable: true,
            resolution: null,
          },
        ],
        unsupported_citations: [],
        stale_citations: [],
        cited_entries: 1,
        uncited_entries: 0,
      },
    };
    mockInvoke.mockResolvedValueOnce(response);
    const result = await queryPatientHistory('p1', 'Wann wurde die Medikation geändert?');
    expect(mockInvoke).toHaveBeenCalledWith('query_patient_history', {
      patientId: 'p1',
      question: 'Wann wurde die Medikation geändert?',
      systemPrompt: undefined,
      thinkingEffort: null,
    });
    expect(result.answer).toBe(response.answer);
    expect(result.manifest.entries[0].citation).toBe('E1');
    expect(result.audit.unsupported_citations).toEqual([]);
  });

  it('calls query_patient_history with optional systemPrompt', async () => {
    mockInvoke.mockResolvedValueOnce({
      answer: 'Antwort mit custom Prompt',
      manifest: evidenceManifest,
      audit: {
        manifest_id: 'm1',
        citations: [],
        unsupported_citations: [],
        stale_citations: [],
        cited_entries: 0,
        uncited_entries: 1,
      },
    });
    await queryPatientHistory('p1', 'Test Frage', 'Custom system prompt');
    expect(mockInvoke).toHaveBeenCalledWith('query_patient_history', {
      patientId: 'p1',
      question: 'Test Frage',
      systemPrompt: 'Custom system prompt',
      thinkingEffort: null,
    });
  });

  it('propagates invoke errors', async () => {
    mockInvoke.mockRejectedValueOnce({
      code: 'AUTH_REQUIRED',
      message: 'Authentication required',
      ref: 'AUTH-001',
    });
    await expect(queryPatientHistory('p1', 'Question')).rejects.toMatchObject({
      code: 'AUTH_REQUIRED',
    });
  });
});

describe('evidence assembly commands', () => {
  it('previews assembled evidence with an optional token budget', async () => {
    mockInvoke.mockResolvedValueOnce({
      evidence: '===== EVIDENZ =====',
      manifest: evidenceManifest,
    });
    const preview = await previewPatientEvidence('p1', 'Aktuelle Dosis?', 4000);
    expect(mockInvoke).toHaveBeenCalledWith('preview_patient_evidence', {
      patientId: 'p1',
      question: 'Aktuelle Dosis?',
      tokenBudget: 4000,
      thinkingEffort: null,
    });
    expect(preview.manifest.prompt_tokens).toBe(8123);

    mockInvoke.mockResolvedValueOnce({ evidence: '', manifest: evidenceManifest });
    await previewPatientEvidence('p1', 'Aktuelle Dosis?');
    expect(mockInvoke).toHaveBeenLastCalledWith('preview_patient_evidence', {
      patientId: 'p1',
      question: 'Aktuelle Dosis?',
      tokenBudget: undefined,
      thinkingEffort: null,
    });
  });

  it('refreshes the evidence index', async () => {
    mockInvoke.mockResolvedValueOnce(evidenceManifest.index);
    const stats = await indexPatientEvidence('p1');
    expect(mockInvoke).toHaveBeenCalledWith('index_patient_evidence', { patientId: 'p1' });
    expect(stats.units_total).toBe(42);
  });

  it('reads the latest manifest and tolerates none', async () => {
    mockInvoke.mockResolvedValueOnce(evidenceManifest);
    await expect(getPatientEvidenceManifest('p1')).resolves.toMatchObject({ manifest_id: 'm1' });
    mockInvoke.mockResolvedValueOnce(null);
    await expect(getPatientEvidenceManifest('p1')).resolves.toBeNull();
  });

  it('resolves cited units against the current record', async () => {
    mockInvoke.mockResolvedValueOnce([
      {
        unit_id: 'u1',
        record_kind: 'session',
        record_id: 's1',
        section: 'notes',
        label: 'Sitzung 2026-03-04 (Verlaufsgespräch)',
        occurred_at: '2026-03-04',
        revision: 'r1:abc',
        char_start: 0,
        char_end: 120,
        text: 'Sertralin auf 100 mg erhöht.',
        traceable: true,
        resolution: {
          source_present: true,
          revision_current: true,
          text_matches: true,
          current_revision: 'r1:abc',
          current_text: 'Sertralin auf 100 mg erhöht.',
        },
      },
    ]);
    const units = await resolveEvidenceUnits('p1', ['u1']);
    expect(mockInvoke).toHaveBeenCalledWith('resolve_evidence_units', {
      patientId: 'p1',
      unitIds: ['u1'],
    });
    expect(units[0].traceable).toBe(true);
  });

  it('propagates invoke errors', async () => {
    mockInvoke.mockRejectedValueOnce({ code: 'AUTH_REQUIRED', message: 'Auth', ref: 'A-1' });
    await expect(indexPatientEvidence('p1')).rejects.toMatchObject({ code: 'AUTH_REQUIRED' });
  });
});
