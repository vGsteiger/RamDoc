import { invoke } from '@tauri-apps/api/core';
import type { AuthStatus } from './stores/auth';

// ---------------------------------------------------------------------------
// Error handling
// ---------------------------------------------------------------------------

/** Structured error returned by every Tauri command on failure. */
export interface AppError {
  code: string;
  message: string;
  ref: string;
}

/**
 * Parse an unknown catch-block value into an {@link AppError}.
 * Tauri rejects with `{ code, message, ref }` objects; any other shape is wrapped
 * into an `UNKNOWN_ERROR`.
 */
export function parseError(err: unknown): AppError {
  if (err !== null && typeof err === 'object' && 'code' in err && 'message' in err) {
    const ref = 'ref' in err ? String(err.ref) : 'UNKNOWN_REF';
    return {
      code: String(err.code),
      message: String(err.message),
      ref,
    };
  }
  return {
    code: 'UNKNOWN_ERROR',
    message: String(err),
    ref: 'UNKNOWN_REF',
  };
}

/**
 * Format an AppError for display to the user.
 * Includes the error message and a shareable error reference for support.
 */
export function formatError(err: AppError): string {
  return `${err.message}\n\nError Reference: ${err.ref}\n(Share this reference with support if you need help)`;
}

/**
 * Get a user-friendly error message based on the error code.
 * Falls back to the original message if no specific handling exists.
 */
export function getUserFriendlyMessage(err: AppError): string {
  switch (err.code) {
    case 'REPORT_NOT_FOUND':
      return 'The requested report could not be found. It may have been deleted.';
    case 'PATIENT_NOT_FOUND':
      return 'The requested patient could not be found. They may have been deleted.';
    case 'SESSION_NOT_FOUND':
      return 'The requested session could not be found. It may have been deleted.';
    case 'FILE_NOT_FOUND':
      return 'The requested file could not be found. It may have been deleted.';
    case 'REPORT_VALIDATION_ERROR':
      return 'The report data is invalid. Please check your input and try again.';
    case 'PATIENT_VALIDATION_ERROR':
      return 'The patient data is invalid. Please check your input and try again.';
    case 'DB_UNIQUE_CONSTRAINT':
      return 'This record already exists in the database.';
    case 'DB_FOREIGN_KEY':
      return "Cannot complete this operation because it references data that doesn't exist.";
    case 'AUTH_REQUIRED':
      return 'Please unlock the application to continue.';
    case 'LLM_ERROR':
      return 'An error occurred while generating content with the language model.';
    default:
      return err.message;
  }
}

export async function checkAuth(): Promise<AuthStatus> {
  return await invoke<AuthStatus>('check_auth');
}

export async function initializeApp(): Promise<string[]> {
  return await invoke<string[]>('initialize_app');
}

export async function unlockApp(): Promise<boolean> {
  return await invoke<boolean>('unlock_app');
}

export async function recoverApp(words: string[]): Promise<boolean> {
  return await invoke<boolean>('recover_app', { words });
}

export async function lockApp(): Promise<void> {
  return await invoke<void>('lock_app');
}

/**
 * Factory reset — permanently deletes all keychain keys, the encrypted vault,
 * the database, and any model files in the data directory.  The app returns to
 * `first_run` state.  **Irreversible.**
 */
export async function resetApp(): Promise<void> {
  return await invoke<void>('reset_app');
}

export interface LlmEngineStatus {
  is_loaded: boolean;
  model_name: string | null;
  model_path: string | null;
  total_ram_bytes: number;
  is_downloaded: boolean;
  downloaded_filename: string | null;
}

export interface EmbedStatus {
  is_loaded: boolean;
  is_downloaded: boolean;
}

export async function getEmbedStatus(): Promise<EmbedStatus> {
  return await invoke<EmbedStatus>('get_embed_status');
}

export async function initializeEmbedEngine(): Promise<void> {
  return await invoke<void>('initialize_embed_engine');
}

export interface ModelChoice {
  name: string;
  filename: string;
  size_bytes: number;
  reason: string;
}

export async function getEngineStatus(): Promise<LlmEngineStatus> {
  return await invoke<LlmEngineStatus>('get_engine_status');
}

export async function getRecommendedModel(): Promise<ModelChoice> {
  return await invoke<ModelChoice>('get_recommended_model');
}

export async function downloadModel(model: ModelChoice): Promise<void> {
  return await invoke<void>('download_model', { model });
}

export async function loadModel(modelFilename: string): Promise<void> {
  return await invoke<void>('load_model', { modelFilename });
}

export interface FileRecord {
  id: string;
  patient_id: string;
  filename: string;
  vault_path: string;
  mime_type: string;
  size_bytes: number;
  created_at: string;
}

export async function uploadFile(
  patientId: string,
  filename: string,
  data: number[],
  mimeType: string
): Promise<FileRecord> {
  return await invoke<FileRecord>('upload_file', {
    patientId,
    filename,
    data,
    mimeType,
  });
}

export async function downloadFile(fileId: string): Promise<number[]> {
  return await invoke<number[]>('download_file', { fileId });
}

export async function listFiles(patientId: string): Promise<FileRecord[]> {
  return await invoke<FileRecord[]>('list_files', { patientId });
}

export async function deleteFile(fileId: string): Promise<void> {
  return await invoke<void>('delete_file', { fileId });
}

/**
 * Trigger background text extraction and semantic embedding for a file.
 * Call this after `uploadFile` returns.  The backend emits a `"file-processed"`
 * event when done.  Fire-and-forget: the upload UI should not await this.
 */
export async function processFile(fileId: string): Promise<void> {
  return await invoke<void>('process_file', { fileId });
}

export interface Patient {
  id: string;
  first_name: string;
  last_name: string;
  date_of_birth: string;
  gender: string | null;
  ahv_number: string | null;
  address: string | null;
  phone: string | null;
  email: string | null;
  insurance: string | null;
  gp_name: string | null;
  gp_address: string | null;
  notes: string | null;
  created_at: string;
  updated_at: string;
}

export interface CreatePatient {
  ahv_number: string;
  first_name: string;
  last_name: string;
  date_of_birth: string;
  gender?: string | null;
  address?: string | null;
  phone?: string | null;
  email?: string | null;
  insurance?: string | null;
  gp_name?: string | null;
  gp_address?: string | null;
  notes?: string | null;
}

export interface UpdatePatient {
  ahv_number?: string | null;
  first_name?: string | null;
  last_name?: string | null;
  date_of_birth?: string | null;
  gender?: string | null;
  address?: string | null;
  phone?: string | null;
  email?: string | null;
  insurance?: string | null;
  gp_name?: string | null;
  gp_address?: string | null;
  notes?: string | null;
}

export interface SearchResult {
  result_type: string;
  entity_id: string;
  patient_id: string;
  patient_name: string;
  title: string;
  snippet: string;
  date: string | null;
  rank: number;
}

// === Patient API ===

export async function createPatient(input: CreatePatient): Promise<Patient> {
  return await invoke<Patient>('create_patient', { input });
}

export async function getPatient(id: string): Promise<Patient> {
  return await invoke<Patient>('get_patient', { id });
}

export async function listPatients(limit?: number, offset?: number): Promise<Patient[]> {
  return await invoke<Patient[]>('list_patients', { limit, offset });
}

export async function updatePatient(id: string, input: UpdatePatient): Promise<Patient> {
  return await invoke<Patient>('update_patient', { id, input });
}

export async function deletePatient(id: string): Promise<void> {
  return await invoke<void>('delete_patient', { id });
}

// === Search API ===

export async function globalSearch(query: string, limit?: number): Promise<SearchResult[]> {
  return await invoke<SearchResult[]>('global_search', { query, limit });
}

// === Session Types ===

export interface Session {
  id: string;
  patient_id: string;
  session_date: string;
  session_type: string;
  duration_minutes: number | null;
  notes: string | null;
  amdp_data: string | null;
  created_at: string;
  updated_at: string;
}

export interface CreateSession {
  patient_id: string;
  session_date: string;
  session_type: string;
  duration_minutes?: number;
  notes?: string;
  amdp_data?: string;
}

export interface UpdateSession {
  session_date?: string;
  session_type?: string;
  duration_minutes?: number;
  notes?: string;
  amdp_data?: string;
}

export async function createSession(input: CreateSession): Promise<Session> {
  return await invoke<Session>('create_session', { input });
}

export async function getSession(id: string): Promise<Session> {
  return await invoke<Session>('get_session', { id });
}

export interface SessionWithPatient {
  session: Session;
  patient_name: string;
}

export async function listAllSessions(
  limit?: number,
  offset?: number
): Promise<SessionWithPatient[]> {
  return await invoke<SessionWithPatient[]>('list_all_sessions', {
    limit,
    offset,
  });
}

export async function listSessionsForPatient(
  patientId: string,
  limit?: number,
  offset?: number
): Promise<Session[]> {
  return await invoke<Session[]>('list_sessions_for_patient', {
    patientId,
    limit,
    offset,
  });
}

export async function updateSession(id: string, input: UpdateSession): Promise<Session> {
  return await invoke<Session>('update_session', { id, input });
}

export async function deleteSession(id: string): Promise<void> {
  return await invoke<void>('delete_session', { id });
}

// === Diagnosis Types ===

export interface Diagnosis {
  id: string;
  patient_id: string;
  icd10_code: string;
  description: string;
  status: string;
  diagnosed_date: string;
  resolved_date: string | null;
  notes: string | null;
  created_at: string;
  updated_at: string;
}

export interface CreateDiagnosis {
  patient_id: string;
  icd10_code: string;
  description: string;
  status?: string;
  diagnosed_date: string;
  resolved_date?: string;
  notes?: string;
}

export interface UpdateDiagnosis {
  icd10_code?: string;
  description?: string;
  status?: string;
  diagnosed_date?: string;
  resolved_date?: string;
  notes?: string;
}

export async function createDiagnosis(input: CreateDiagnosis): Promise<Diagnosis> {
  return await invoke<Diagnosis>('create_diagnosis', { input });
}

export async function getDiagnosis(id: string): Promise<Diagnosis> {
  return await invoke<Diagnosis>('get_diagnosis', { id });
}

export async function listDiagnosesForPatient(
  patientId: string,
  limit?: number,
  offset?: number
): Promise<Diagnosis[]> {
  return await invoke<Diagnosis[]>('list_diagnoses_for_patient', {
    patientId,
    limit,
    offset,
  });
}

export async function updateDiagnosis(id: string, input: UpdateDiagnosis): Promise<Diagnosis> {
  return await invoke<Diagnosis>('update_diagnosis', { id, input });
}

export async function deleteDiagnosis(id: string): Promise<void> {
  return await invoke<void>('delete_diagnosis', { id });
}

// === Medication Types ===

export interface Medication {
  id: string;
  patient_id: string;
  substance: string;
  dosage: string;
  frequency: string;
  start_date: string;
  end_date: string | null;
  notes: string | null;
  created_at: string;
  updated_at: string;
}

export interface CreateMedication {
  patient_id: string;
  substance: string;
  dosage: string;
  frequency: string;
  start_date: string;
  end_date?: string;
  notes?: string;
}

export interface UpdateMedication {
  substance?: string;
  dosage?: string;
  frequency?: string;
  start_date?: string;
  end_date?: string;
  notes?: string;
}

export async function createMedication(input: CreateMedication): Promise<Medication> {
  return await invoke<Medication>('create_medication', { input });
}

export async function getMedication(id: string): Promise<Medication> {
  return await invoke<Medication>('get_medication', { id });
}

export async function listMedicationsForPatient(
  patientId: string,
  limit?: number,
  offset?: number
): Promise<Medication[]> {
  return await invoke<Medication[]>('list_medications_for_patient', {
    patientId,
    limit,
    offset,
  });
}

export async function updateMedication(id: string, input: UpdateMedication): Promise<Medication> {
  return await invoke<Medication>('update_medication', { id, input });
}

export async function deleteMedication(id: string): Promise<void> {
  return await invoke<void>('delete_medication', { id });
}

// === Treatment Plan Types ===

export interface TreatmentPlan {
  id: string;
  patient_id: string;
  title: string;
  description: string | null;
  start_date: string;
  end_date: string | null;
  status: string;
  created_at: string;
  updated_at: string;
}

export interface CreateTreatmentPlan {
  patient_id: string;
  title: string;
  description?: string;
  start_date: string;
  end_date?: string;
  status?: string;
}

export interface UpdateTreatmentPlan {
  title?: string;
  description?: string;
  start_date?: string;
  end_date?: string;
  status?: string;
}

export async function createTreatmentPlan(
  input: CreateTreatmentPlan,
): Promise<TreatmentPlan> {
  return await invoke<TreatmentPlan>("create_treatment_plan", { input });
}

export async function getTreatmentPlan(id: string): Promise<TreatmentPlan> {
  return await invoke<TreatmentPlan>("get_treatment_plan", { id });
}

export async function listTreatmentPlansForPatient(
  patientId: string,
  limit?: number,
  offset?: number,
): Promise<TreatmentPlan[]> {
  return await invoke<TreatmentPlan[]>("list_treatment_plans_for_patient", {
    patientId,
    limit,
    offset,
  });
}

export async function updateTreatmentPlan(
  id: string,
  input: UpdateTreatmentPlan,
): Promise<TreatmentPlan> {
  return await invoke<TreatmentPlan>("update_treatment_plan", { id, input });
}

export async function deleteTreatmentPlan(id: string): Promise<void> {
  return await invoke<void>("delete_treatment_plan", { id });
}

// === Treatment Goal Types ===

export interface TreatmentGoal {
  id: string;
  treatment_plan_id: string;
  description: string;
  target_date: string | null;
  status: string;
  sort_order: number;
  created_at: string;
  updated_at: string;
}

export interface CreateTreatmentGoal {
  treatment_plan_id: string;
  description: string;
  target_date?: string;
  status?: string;
  sort_order?: number;
}

export interface UpdateTreatmentGoal {
  description?: string;
  target_date?: string;
  status?: string;
  sort_order?: number;
}

export async function createTreatmentGoal(
  input: CreateTreatmentGoal,
): Promise<TreatmentGoal> {
  return await invoke<TreatmentGoal>("create_treatment_goal", { input });
}

export async function getTreatmentGoal(id: string): Promise<TreatmentGoal> {
  return await invoke<TreatmentGoal>("get_treatment_goal", { id });
}

export async function listTreatmentGoalsForPlan(
  planId: string,
  limit?: number,
  offset?: number,
): Promise<TreatmentGoal[]> {
  return await invoke<TreatmentGoal[]>("list_treatment_goals_for_plan", {
    planId,
    limit,
    offset,
  });
}

export async function updateTreatmentGoal(
  id: string,
  input: UpdateTreatmentGoal,
): Promise<TreatmentGoal> {
  return await invoke<TreatmentGoal>("update_treatment_goal", { id, input });
}

export async function deleteTreatmentGoal(id: string): Promise<void> {
  return await invoke<void>("delete_treatment_goal", { id });
}

// === Treatment Intervention Types ===

export interface TreatmentIntervention {
  id: string;
  treatment_plan_id: string;
  type: string;
  description: string;
  frequency: string | null;
  created_at: string;
  updated_at: string;
}

export interface CreateTreatmentIntervention {
  treatment_plan_id: string;
  type: string;
  description: string;
  frequency?: string;
}

export interface UpdateTreatmentIntervention {
  type?: string;
  description?: string;
  frequency?: string;
}

export async function createTreatmentIntervention(
  input: CreateTreatmentIntervention,
): Promise<TreatmentIntervention> {
  return await invoke<TreatmentIntervention>("create_treatment_intervention", {
    input,
  });
}

export async function getTreatmentIntervention(
  id: string,
): Promise<TreatmentIntervention> {
  return await invoke<TreatmentIntervention>("get_treatment_intervention", {
    id,
  });
}

export async function listTreatmentInterventionsForPlan(
  planId: string,
  limit?: number,
  offset?: number,
): Promise<TreatmentIntervention[]> {
  return await invoke<TreatmentIntervention[]>(
    "list_treatment_interventions_for_plan",
    {
      planId,
      limit,
      offset,
    },
  );
}

export async function updateTreatmentIntervention(
  id: string,
  input: UpdateTreatmentIntervention,
): Promise<TreatmentIntervention> {
  return await invoke<TreatmentIntervention>("update_treatment_intervention", {
    id,
    input,
  });
}

export async function deleteTreatmentIntervention(id: string): Promise<void> {
  return await invoke<void>("delete_treatment_intervention", { id });
}

// === Report Types ===

export interface Report {
  id: string;
  patient_id: string;
  report_type: string;
  content: string;
  generated_at: string;
  model_name: string | null;
  prompt_hash: string | null;
  session_ids: string | null;
  created_at: string;
}

export interface CreateReport {
  patient_id: string;
  report_type: string;
  content: string;
  model_name: string | null;
  prompt_hash: string | null;
  session_ids: string | null;
}

export interface UpdateReport {
  report_type?: string;
  content?: string;
  model_name?: string;
  prompt_hash?: string;
  session_ids?: string;
}

// === Report API ===

export async function createReport(input: CreateReport): Promise<Report> {
  return await invoke<Report>('create_report', { input });
}

export async function getReport(id: string): Promise<Report> {
  return await invoke<Report>('get_report', { id });
}

export async function listReports(
  patientId: string,
  limit?: number,
  offset?: number
): Promise<Report[]> {
  return await invoke<Report[]>('list_reports', {
    patientId,
    limit,
    offset,
  });
}

export async function updateReport(id: string, input: UpdateReport): Promise<Report> {
  return await invoke<Report>('update_report', { id, input });
}

export async function deleteReport(id: string): Promise<void> {
  return await invoke<void>('delete_report', { id });
}

export async function generateReport(
  patientContext: string,
  reportType: string,
  sessionNotes: string,
  systemPrompt?: string
): Promise<string> {
  return await invoke<string>('generate_report', {
    patientContext,
    reportType,
    sessionNotes,
    systemPrompt,
  });
}

export async function exportReportToPdf(reportId: string): Promise<number[]> {
  return await invoke<number[]>('export_report_to_pdf', { reportId });
}

export async function exportReportToDocx(reportId: string): Promise<number[]> {
  return await invoke<number[]>('export_report_to_docx', { reportId });
}

// ---------------------------------------------------------------------------
// Email
// ---------------------------------------------------------------------------

export interface Email {
  id: string;
  patient_id: string;
  recipient_email: string;
  subject: string;
  body: string;
  status: string;
  sent_at: string | null;
  created_at: string;
  updated_at: string;
}

export interface CreateEmail {
  patient_id: string;
  recipient_email: string;
  subject: string;
  body: string;
}

export interface UpdateEmail {
  recipient_email?: string;
  subject?: string;
  body?: string;
  status?: string;
}

export async function createEmail(input: CreateEmail): Promise<Email> {
  return await invoke<Email>('create_email', { input });
}

export async function getEmail(id: string): Promise<Email> {
  return await invoke<Email>('get_email', { id });
}

export async function listEmails(
  patientId: string,
  limit?: number,
  offset?: number
): Promise<Email[]> {
  return await invoke<Email[]>('list_emails', {
    patientId,
    limit,
    offset,
  });
}

export async function updateEmail(id: string, input: UpdateEmail): Promise<Email> {
  return await invoke<Email>('update_email', { id, input });
}

export async function deleteEmail(id: string): Promise<void> {
  return await invoke<void>('delete_email', { id });
}

export async function markEmailAsSent(id: string): Promise<Email> {
  return await invoke<Email>('mark_email_as_sent', { id });
}

// ---------------------------------------------------------------------------
// Updater
// ---------------------------------------------------------------------------

export interface UpdateInfo {
  current_version: string;
  latest_version: string | null;
  update_available: boolean;
  body: string | null;
  date: string | null;
}

export async function checkForUpdates(): Promise<UpdateInfo> {
  return await invoke<UpdateInfo>('check_for_updates');
}

export async function installUpdate(): Promise<void> {
  return await invoke<void>('install_update');
}

export async function getAppVersion(): Promise<string> {
  return await invoke<string>('get_app_version');
}

// ---------------------------------------------------------------------------
// Export
// ---------------------------------------------------------------------------

export async function exportAllPatientData(): Promise<number[]> {
  return await invoke<number[]>('export_all_patient_data');
}

// ---------------------------------------------------------------------------
// Backup & Restore
// ---------------------------------------------------------------------------

export interface BackupInfo {
  schema_version: number;
  created_at: string;
  db_schema_version: number;
  file_count: number;
}

/**
 * Create an encrypted full-vault backup.
 * Returns the encrypted backup archive as a byte array.
 */
export async function createVaultBackup(): Promise<number[]> {
  return await invoke<number[]>("create_vault_backup");
}

/**
 * Restore a full-vault backup from an encrypted archive.
 * WARNING: This replaces ALL current data with the backup contents.
 */
export async function restoreVaultBackup(
  encryptedBackup: number[],
): Promise<BackupInfo> {
  return await invoke<BackupInfo>("restore_vault_backup", { encryptedBackup });
}

/**
 * Validate a backup archive without restoring it.
 * Returns metadata about the backup if validation succeeds.
 */
export async function validateBackupArchive(
  encryptedBackup: number[],
): Promise<BackupInfo> {
  return await invoke<BackupInfo>("validate_backup_archive", {
    encryptedBackup,
  });
}

// Chat / Agent API
// ---------------------------------------------------------------------------

export interface ChatSession {
  id: string;
  scope: string;
  patient_id: string | null;
  title: string;
  created_at: string;
  updated_at: string;
}

export interface ChatMessageRow {
  id: string;
  session_id: string;
  role: string; // 'user' | 'assistant' | 'tool_call' | 'tool_result'
  content: string;
  tool_name: string | null;
  tool_args_json: string | null;
  tool_result_for: string | null;
  created_at: string;
}

export interface AgentToolCall {
  name: string;
  args_json: string;
  result_json: string;
}

export interface AgentTurnResult {
  session_id: string;
  final_answer: string;
  tool_calls_made: AgentToolCall[];
}

export async function runAgentTurn(
  sessionId: string,
  userMessage: string
): Promise<AgentTurnResult> {
  return await invoke<AgentTurnResult>('run_agent_turn', {
    sessionId,
    userMessage,
  });
}

export async function createChatSession(
  scope: string,
  patientId?: string,
  title?: string
): Promise<ChatSession> {
  return await invoke<ChatSession>('create_chat_session', {
    scope,
    patientId,
    title,
  });
}

export async function getOrCreatePatientChatSession(patientId: string): Promise<ChatSession> {
  return await invoke<ChatSession>('get_or_create_patient_chat_session', {
    patientId,
  });
}

export async function listChatSessions(scope: string, patientId?: string): Promise<ChatSession[]> {
  return await invoke<ChatSession[]>('list_chat_sessions', {
    scope,
    patientId,
  });
}

export async function deleteChatSession(sessionId: string): Promise<void> {
  return await invoke<void>('delete_chat_session', { sessionId });
}

export async function getChatMessages(sessionId: string): Promise<ChatMessageRow[]> {
  return await invoke<ChatMessageRow[]>('get_chat_messages', { sessionId });
}

export async function renameChatSession(sessionId: string, title: string): Promise<ChatSession> {
  return await invoke<ChatSession>('rename_chat_session', { sessionId, title });
}

// ---------------------------------------------------------------------------
// Literature API
// ---------------------------------------------------------------------------

export interface Literature {
  id: string;
  filename: string;
  vault_path: string;
  mime_type: string;
  size_bytes: number;
  description: string | null;
  created_at: string;
  updated_at: string;
  chunk_count: number;
}

export interface LiteratureChunkResult {
  chunk_id: string;
  literature_id: string;
  filename: string;
  chunk_index: number;
  content: string;
  similarity: number;
}

export interface DocumentChunk {
  id: string;
  file_id: string | null;
  literature_id: string | null;
  chunk_index: number;
  content: string;
  word_count: number;
  created_at: string;
}

export async function uploadLiterature(
  filename: string,
  data: Uint8Array,
  mimeType: string,
  description: string | null = null
): Promise<Literature> {
  return await invoke<Literature>('upload_literature', {
    filename,
    data: Array.from(data),
    mimeType,
    description,
  });
}

export async function getLiteratureById(id: string): Promise<Literature> {
  return await invoke<Literature>('get_literature_by_id', { id });
}

export async function listAllLiterature(
  limit: number = 100,
  offset: number = 0
): Promise<Literature[]> {
  return await invoke<Literature[]>('list_all_literature', { limit, offset });
}

export async function updateLiteratureMetadata(
  id: string,
  description: string | null
): Promise<Literature> {
  return await invoke<Literature>('update_literature_metadata', {
    id,
    description,
  });
}

export async function deleteLiteratureDocument(id: string): Promise<void> {
  return await invoke<void>('delete_literature_document', { id });
}

export async function downloadLiterature(id: string): Promise<Uint8Array> {
  const data = await invoke<number[]>('download_literature', { id });
  return new Uint8Array(data);
}

export async function processLiterature(id: string): Promise<void> {
  return await invoke<void>('process_literature', { id });
}

export async function searchLiterature(
  query: string,
  limit: number = 5
): Promise<LiteratureChunkResult[]> {
  return await invoke<LiteratureChunkResult[]>('search_literature', {
    query,
    limit,
  });
}

export async function getLiteratureDocumentChunks(id: string): Promise<DocumentChunk[]> {
  return await invoke<DocumentChunk[]>('get_literature_document_chunks', { id });
}

// ---------------------------------------------------------------------------
// Dashboard API
// ---------------------------------------------------------------------------

export interface DashboardData {
  todays_sessions: SessionWithPatient[];
  recent_patients: Patient[];
  sessions_with_incomplete_notes: SessionWithPatient[];
}

export async function getDashboardData(): Promise<DashboardData> {
  return await invoke<DashboardData>("get_dashboard_data");
}

