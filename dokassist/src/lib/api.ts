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

export interface GenerationStats {
  ttft_ms: number;
  tps: number;
  completion_tokens: number;
  prompt_tokens: number;
  evaluated_prompt_tokens: number;
  reused_prompt_tokens: number;
  cache_hit: boolean;
  prefill_ms: number;
  estimated_prefill_saved_ms: number;
  total_latency_ms: number;
  peak_rss_bytes: number;
}

export interface ContextCacheTelemetry {
  hits: number;
  misses: number;
  invalidations: number;
  evictions: number;
  reused_tokens: number;
  evaluated_tokens: number;
  estimated_prefill_saved_ms: number;
  resident_contexts: number;
  max_contexts: number;
}

export interface LlmEngineStatus {
  is_loaded: boolean;
  model_name: string | null;
  model_path: string | null;
  total_ram_bytes: number;
  is_downloaded: boolean;
  downloaded_filename: string | null;
  last_generation_stats: GenerationStats | null;
  inference_config: InferenceDiagnostics | null;
  context_cache: ContextCacheTelemetry;
  desired_model: DesiredModelStatus | null;
  lifecycle: EngineLifecycleStatus;
}

export interface DesiredModelStatus {
  id: string;
  name: string;
  filename: string;
  exists_on_disk: boolean;
}

export type EngineLifecyclePhase = 'idle' | 'loading' | 'ready' | 'unloading' | 'error';

export interface EngineLifecycleStatus {
  phase: EngineLifecyclePhase;
  requested_filename: string | null;
  active_filename: string | null;
  error: string | null;
}

export type RouterMode = 'managed' | 'advanced_override';

export interface RouterDiagnostics {
  role: 'tool_router';
  mode: RouterMode;
  managed_model_filename: string | null;
  active_model_filename: string | null;
  resident_engine_count: number;
  advanced_override: {
    available: boolean;
    configured_filename: string | null;
    requires_separate_engine: boolean;
    limitation: string;
  };
}

export type InferenceProfile = 'conservative' | 'f16-32k' | 'q8-32k' | 'q4-32k';
export type ThinkingEffort = 'low' | 'medium' | 'high' | 'extra_high';
export type FlashAttentionMode = 'enabled' | 'auto';
export type InferenceFallbackCode =
  | 'native_context_cap'
  | 'flash_auto'
  | 'kv_f16'
  | 'kv_f16_flash_auto';

export interface InferenceDiagnostics {
  profile: InferenceProfile;
  context_size: number;
  kv_cache_k: string;
  kv_cache_v: string;
  n_batch: number;
  n_ubatch: number;
  flash_attention: FlashAttentionMode;
  completion_headroom: number;
  fallback: string | null;
  fallback_code: InferenceFallbackCode | null;
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

export async function loadModel(
  modelFilename: string,
  inferenceProfile?: InferenceProfile,
): Promise<void> {
  const args: { modelFilename: string; inferenceProfile?: InferenceProfile } = { modelFilename };
  if (inferenceProfile) args.inferenceProfile = inferenceProfile;
  return await invoke<void>('load_model', args);
}

/** Load the configured default writing model, if it is not already resident. */
export async function ensureWritingModelLoaded(): Promise<LlmEngineStatus> {
  return await invoke<LlmEngineStatus>('ensure_writing_model_loaded');
}

/** Explicitly release the resident writing model from application state. */
export async function unloadModel(): Promise<void> {
  return await invoke<void>('unload_model');
}

/** Report the managed tool-router role and advanced-override limitations. */
export async function getRouterDiagnostics(): Promise<RouterDiagnostics> {
  return await invoke<RouterDiagnostics>('get_router_diagnostics');
}

// === Model Management ===

export interface Model {
  id: string;
  name: string;
  filename: string;
  sha256: string;
  size_bytes: number;
  downloaded_at: string;
  last_used: string | null;
  is_default: boolean;
}

export interface ModelInfo {
  id: string;
  name: string;
  filename: string;
  sha256: string;
  size_bytes: number;
  downloaded_at: string;
  last_used: string | null;
  is_default: boolean;
  is_loaded: boolean;
  exists_on_disk: boolean;
  quantization_promotion: QuantizationPromotionSummary | null;
}

export interface QuantizationPromotionSummary {
  study_id: string;
  created_at: string;
  quantization: string;
  recipe_sha256: string;
  study_manifest_sha256: string;
  held_out_results_sha256: string;
  llama_cpp_commit: string;
  categories: string[];
  baseline_artifacts: string[];
  dominates: string[];
  worst_category_regression: number;
}

export interface PromotedModelPreview {
  display_name: string;
  study_id: string;
  filename: string;
  size_bytes: number;
  quantization: string;
  artifact_found: boolean;
  artifact_size_matches: boolean;
  artifact_bytes: number | null;
  artifact_path: string | null;
  dominates: string[];
  baseline_artifacts: string[];
  worst_category_regression: number;
}

export async function listModels(): Promise<ModelInfo[]> {
  return await invoke<ModelInfo[]>('list_models');
}

export async function getModelInfo(modelId: string): Promise<ModelInfo> {
  return await invoke<ModelInfo>('get_model_info', { modelId });
}

export async function downloadAndRegisterModel(model: ModelChoice): Promise<Model> {
  return await invoke<Model>('download_and_register_model', { model });
}

export async function inspectPromotedModel(
  promotionPath: string,
  artifactPath?: string | null
): Promise<PromotedModelPreview> {
  return await invoke<PromotedModelPreview>('inspect_promoted_model', {
    promotionPath,
    artifactPath: artifactPath ?? null,
  });
}

export async function importPromotedModel(
  promotionPath: string,
  artifactPath?: string | null
): Promise<Model> {
  return await invoke<Model>('import_promoted_model', {
    promotionPath,
    artifactPath: artifactPath ?? null,
  });
}

export async function deleteModel(modelId: string): Promise<void> {
  return await invoke<void>('delete_model', { modelId });
}

export async function setDefaultModel(modelId: string): Promise<void> {
  return await invoke<void>('set_default_model', { modelId });
}

export async function getDefaultModel(): Promise<Model | null> {
  return await invoke<Model | null>('get_default_model');
}

export interface AvailableModel {
  name: string;
  filename: string;
  size_bytes: number;
  min_ram_gb: number;
  description: string;
  context_window_tokens: number;
  parameters: string;
  license: string;
  disclaimer: string | null;
  is_downloaded: boolean;
  model_id: string | null;
}

export async function listAvailableModels(): Promise<AvailableModel[]> {
  return await invoke<AvailableModel[]>('list_available_models');
}

// === File Management ===

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
  scheduled_time: string | null;
  notes: string | null;
  amdp_data: string | null;
  clinical_summary: string | null;
  created_at: string;
  updated_at: string;
}

export interface CreateSession {
  patient_id: string;
  session_date: string;
  session_type: string;
  duration_minutes?: number;
  scheduled_time?: string;
  notes?: string;
  amdp_data?: string;
}

export interface UpdateSession {
  session_date?: string;
  session_type?: string;
  duration_minutes?: number;
  scheduled_time?: string;
  notes?: string;
  amdp_data?: string;
  clinical_summary?: string;
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

// === Medication Reference Types ===

export interface SubstanceSummary {
  id: string;
  name_de: string;
  atc_code: string | null;
  trade_names: string[];
}

export interface SubstanceDetail extends SubstanceSummary {
  indication: string | null;
  side_effects: string | null;
  contraindications: string | null;
  source_version: string | null;
}

export async function searchMedicationReference(query: string): Promise<SubstanceSummary[]> {
  return await invoke<SubstanceSummary[]>('search_medication_reference', { query });
}

export async function getMedicationReferenceDetail(id: string): Promise<SubstanceDetail> {
  return await invoke<SubstanceDetail>('get_medication_reference_detail', { id });
}

export async function getMedicationReferenceVersion(): Promise<string | null> {
  return await invoke<string | null>('get_medication_reference_version');
}

export async function downloadMedicationReference(): Promise<void> {
  return await invoke<void>('download_medication_reference');
}

export interface MedicationComparison {
  current_medication: SubstanceDetail;
  replacement_medication: SubstanceDetail;
}

export async function compareMedications(
  currentId: string,
  replacementId: string
): Promise<MedicationComparison> {
  return await invoke<MedicationComparison>('compare_medications', {
    currentId,
    replacementId,
  });
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

// === Outcome Score Types ===

export interface OutcomeScore {
  id: string;
  session_id: string;
  scale_type: string;
  score: number;
  interpretation: string | null;
  subscores: string | null;
  administered_at: string;
  notes: string | null;
  created_at: string;
  updated_at: string;
}

export interface CreateOutcomeScore {
  session_id: string;
  scale_type: string;
  score: number;
  subscores?: string;
  administered_at: string;
  notes?: string;
}

export interface UpdateOutcomeScore {
  scale_type?: string;
  score?: number;
  subscores?: string;
  administered_at?: string;
  notes?: string;
}

export async function createOutcomeScore(
  input: CreateOutcomeScore,
): Promise<OutcomeScore> {
  return await invoke<OutcomeScore>("create_outcome_score", { input });
}

export async function getOutcomeScore(id: string): Promise<OutcomeScore> {
  return await invoke<OutcomeScore>("get_outcome_score", { id });
}

export async function listScoresForSession(
  sessionId: string,
  limit?: number,
  offset?: number,
): Promise<OutcomeScore[]> {
  return await invoke<OutcomeScore[]>("list_scores_for_session", {
    sessionId,
    limit,
    offset,
  });
}

export async function listScoresByScale(
  scaleType: string,
  limit?: number,
  offset?: number,
): Promise<OutcomeScore[]> {
  return await invoke<OutcomeScore[]>("list_scores_by_scale", {
    scaleType,
    limit,
    offset,
  });
}

export async function listScoresForPatient(
  patientId: string,
  limit?: number,
  offset?: number,
): Promise<OutcomeScore[]> {
  return await invoke<OutcomeScore[]>("list_scores_for_patient", {
    patientId,
    limit,
    offset,
  });
}

export async function updateOutcomeScore(
  id: string,
  input: UpdateOutcomeScore,
): Promise<OutcomeScore> {
  return await invoke<OutcomeScore>("update_outcome_score", { id, input });
}

export async function deleteOutcomeScore(id: string): Promise<void> {
  return await invoke<void>("delete_outcome_score", { id });
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

export interface SamplerConfig {
  temperature: number;
  top_k: number;
  top_p: number;
  min_p: number;
  repeat_penalty: number;
  presence_penalty: number;
  seed: number;
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
  systemPrompt?: string,
  thinkingEffort?: ThinkingEffort,
  sampler?: SamplerConfig
): Promise<string> {
  return await invoke<string>('generate_report', {
    patientContext,
    reportType,
    sessionNotes,
    systemPrompt,
    thinkingEffort: thinkingEffort ?? null,
    sampler: sampler ?? null,
  });
}

export async function generateSessionSummary(
  patientContext: string,
  sessionNotes: string,
  systemPrompt?: string,
  thinkingEffort?: ThinkingEffort
): Promise<string> {
  return await invoke<string>('generate_session_summary', {
    patientContext,
    sessionNotes,
    systemPrompt,
    thinkingEffort: thinkingEffort ?? null,
  });
}

// ---------------------------------------------------------------------------
// Provenance-bearing evidence assembly (patient-history RAG)
// ---------------------------------------------------------------------------

export type EvidenceRecordKind =
  | 'patient'
  | 'session'
  | 'file'
  | 'diagnosis'
  | 'medication'
  | 'outcome_score'
  | 'treatment_plan'
  | 'treatment_goal'
  | 'treatment_intervention';

export type EvidenceTier = 'structured' | 'hot' | 'cold';

export type ProtectedSpanKind =
  'medication' | 'dose' | 'date' | 'negation' | 'uncertainty' | 'risk' | 'provenance';

export interface ProtectedSpan {
  kind: ProtectedSpanKind;
  start: number;
  end: number;
}

/** Why one evidence unit was retrieved. */
export interface EvidenceSelection {
  lexical_rank: number | null;
  lexical_bm25: number | null;
  semantic_rank: number | null;
  semantic_similarity: number | null;
  fused_score: number;
  recency_boost: number;
  matched_terms: string[];
  document_neighbor_of: string[];
  temporal_neighbor_of: string[];
  structured_truth: boolean;
}

/** One included evidence unit, with exact provenance into the source record. */
export interface EvidenceManifestEntry {
  citation: string;
  unit_id: string;
  patient_id: string;
  record_kind: EvidenceRecordKind;
  record_id: string;
  section: string;
  revision: string;
  tier: EvidenceTier;
  label: string;
  occurred_at: string;
  char_start: number;
  char_end: number;
  text_sha256: string;
  tokens: number;
  prompt_token_start: number;
  prompt_token_end: number;
  protected_spans: ProtectedSpan[];
  selection: EvidenceSelection;
  selection_reasons: string[];
}

export interface OmittedEvidenceEntry {
  unit_id: string;
  record_kind: EvidenceRecordKind;
  record_id: string;
  section: string;
  tier: EvidenceTier;
  occurred_at: string;
  tokens: number;
  reason: string;
}

export interface EvidenceRetrievalDiagnostics {
  index_units: number;
  question_terms: string[];
  lexical_hits: number;
  semantic_hits: number;
  semantic_available: boolean;
  document_neighbors_added: number;
  temporal_neighbors_added: number;
  temporal_question: boolean;
  candidates: number;
}

export interface EvidenceIndexStats {
  sources_scanned: number;
  sources_reindexed: number;
  sources_removed: number;
  units_inserted: number;
  units_removed: number;
  stale_chunks_removed: number;
  units_total: number;
  units_missing_embeddings: number;
}

/** Metadata for one assembled evidence prompt. Never contains record text. */
export interface EvidenceManifest {
  manifest_id: string;
  patient_id: string;
  patient_revision: string;
  question_sha256: string;
  created_at: string;
  token_budget: number;
  token_counter: string;
  prompt_tokens: number;
  tier_tokens: {
    structured: number;
    hot: number;
    cold: number;
    pointers: number;
    overhead: number;
  };
  entries: EvidenceManifestEntry[];
  omitted: OmittedEvidenceEntry[];
  protected_spans_retained: { kind: ProtectedSpanKind; count: number }[];
  retrieval: EvidenceRetrievalDiagnostics;
  index: EvidenceIndexStats;
}

export interface EvidenceSpanResolution {
  source_present: boolean;
  revision_current: boolean;
  text_matches: boolean;
  current_revision: string | null;
  current_text: string | null;
}

export interface CitationCheck {
  citation: string;
  in_manifest: boolean;
  unit_id: string | null;
  label: string | null;
  occurred_at: string | null;
  traceable: boolean;
  resolution: EvidenceSpanResolution | null;
}

/** Whether an answer's citations are backed by current source revisions. */
export interface AnswerAudit {
  manifest_id: string;
  citations: CitationCheck[];
  unsupported_citations: string[];
  stale_citations: string[];
  cited_entries: number;
  uncited_entries: number;
}

export interface PatientHistoryAnswer {
  answer: string;
  manifest: EvidenceManifest;
  audit: AnswerAudit;
}

export interface EvidencePreview {
  evidence: string;
  manifest: EvidenceManifest;
}

export interface ResolvedEvidenceUnit {
  unit_id: string;
  record_kind: EvidenceRecordKind;
  record_id: string;
  section: string;
  label: string;
  occurred_at: string;
  revision: string;
  char_start: number;
  char_end: number;
  text: string;
  traceable: boolean;
  resolution: EvidenceSpanResolution;
}

export async function queryPatientHistory(
  patientId: string,
  question: string,
  systemPrompt?: string,
  thinkingEffort?: ThinkingEffort
): Promise<PatientHistoryAnswer> {
  return await invoke<PatientHistoryAnswer>('query_patient_history', {
    patientId,
    question,
    systemPrompt,
    thinkingEffort: thinkingEffort ?? null,
  });
}

/** Assemble the evidence for a question without running inference. */
export async function previewPatientEvidence(
  patientId: string,
  question: string,
  tokenBudget?: number,
  thinkingEffort?: ThinkingEffort
): Promise<EvidencePreview> {
  return await invoke<EvidencePreview>('preview_patient_evidence', {
    patientId,
    question,
    tokenBudget,
    thinkingEffort: thinkingEffort ?? null,
  });
}

/** Refresh the evidence index and embed units that lack a vector. */
export async function indexPatientEvidence(patientId: string): Promise<EvidenceIndexStats> {
  return await invoke<EvidenceIndexStats>('index_patient_evidence', { patientId });
}

export async function getPatientEvidenceManifest(
  patientId: string
): Promise<EvidenceManifest | null> {
  return await invoke<EvidenceManifest | null>('get_patient_evidence_manifest', { patientId });
}

/** Re-resolve cited evidence units against the current record. */
export async function resolveEvidenceUnits(
  patientId: string,
  unitIds: string[]
): Promise<ResolvedEvidenceUnit[]> {
  return await invoke<ResolvedEvidenceUnit[]>('resolve_evidence_units', {
    patientId,
    unitIds,
  });
}

export async function exportReportToPdf(reportId: string): Promise<number[]> {
  return await invoke<number[]>('export_report_to_pdf', { reportId });
}

export async function exportReportToDocx(reportId: string): Promise<number[]> {
  return await invoke<number[]>('export_report_to_docx', { reportId });
}

export async function exportPatientPdf(patientId: string): Promise<number[]> {
  return await invoke<number[]>("export_patient_pdf", { patientId });
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
// Letters
// ---------------------------------------------------------------------------

export type LetterType = 'referral' | 'insurance_authorization' | 'therapy_extension';
export type LetterLanguage = 'de' | 'fr';
export type LetterStatus = 'draft' | 'finalized' | 'sent';

export interface Letter {
  id: string;
  patient_id: string;
  letter_type: LetterType;
  template_language: LetterLanguage;
  recipient_name: string | null;
  recipient_address: string | null;
  subject: string;
  content: string;
  status: LetterStatus;
  model_name: string | null;
  session_ids: string | null;
  created_at: string;
  updated_at: string;
  finalized_at: string | null;
  sent_at: string | null;
}

export interface CreateLetter {
  patient_id: string;
  letter_type: LetterType;
  template_language: LetterLanguage;
  recipient_name?: string;
  recipient_address?: string;
  subject: string;
  content: string;
  model_name?: string;
  session_ids?: string;
}

export interface UpdateLetter {
  letter_type?: LetterType;
  template_language?: LetterLanguage;
  recipient_name?: string;
  recipient_address?: string;
  subject?: string;
  content?: string;
  status?: LetterStatus;
  model_name?: string;
  session_ids?: string;
}

export async function createLetter(input: CreateLetter): Promise<Letter> {
  return await invoke<Letter>('create_letter', { input });
}

export async function getLetter(id: string): Promise<Letter> {
  return await invoke<Letter>('get_letter', { id });
}

export async function listLetters(
  patientId: string,
  limit?: number,
  offset?: number
): Promise<Letter[]> {
  return await invoke<Letter[]>('list_letters', {
    patientId,
    limit,
    offset,
  });
}

export async function updateLetter(id: string, input: UpdateLetter): Promise<Letter> {
  return await invoke<Letter>('update_letter', { id, input });
}

export async function deleteLetter(id: string): Promise<void> {
  return await invoke<void>('delete_letter', { id });
}

export async function markLetterAsFinalized(id: string): Promise<Letter> {
  return await invoke<Letter>('mark_letter_as_finalized', { id });
}

export async function markLetterAsSent(id: string): Promise<Letter> {
  return await invoke<Letter>('mark_letter_as_sent', { id });
}

export async function generateLetter(
  letterType: LetterType,
  language: LetterLanguage,
  patientContext: string,
  clinicalSummary: string,
  recipientName?: string,
  systemPrompt?: string,
  thinkingEffort?: ThinkingEffort
): Promise<string> {
  return await invoke<string>('generate_letter', {
    letterType,
    language,
    patientContext,
    clinicalSummary,
    recipientName,
    systemPrompt,
    thinkingEffort: thinkingEffort ?? null,
  });
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

export async function exportFhirBundle(patientId: string): Promise<string> {
  return await invoke<string>('export_fhir_bundle', { patientId });
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
  userMessage: string,
  thinkingEffort?: ThinkingEffort
): Promise<AgentTurnResult> {
  return await invoke<AgentTurnResult>('run_agent_turn', {
    sessionId,
    userMessage,
    thinkingEffort: thinkingEffort ?? null,
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
// Settings API
// ---------------------------------------------------------------------------

export interface PracticeSettings {
  practice_name?: string | null;
  practice_address?: string | null;
  practice_phone?: string | null;
  practice_email?: string | null;
  therapist_name?: string | null;
  zsr_number?: string | null;
  canton?: string | null;
  clinical_specialty?: string | null;
  language_preference: string;
  onboarding_completed: boolean;
}

export async function getSettings(): Promise<PracticeSettings> {
  return await invoke<PracticeSettings>('get_settings');
}

export async function updateSettings(settings: PracticeSettings): Promise<void> {
  return await invoke<void>('update_settings', { settings });
}

export async function completeOnboarding(): Promise<void> {
  return await invoke<void>('complete_onboarding');
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

// ---------------------------------------------------------------------------
// CSV Import API
// ---------------------------------------------------------------------------

export interface ColumnMapping {
  csv_header: string;
  patient_field: string;
}

export interface CsvWarning {
  row: number | null;
  column: string | null;
  message: string;
}

export interface CsvPreview {
  headers: string[];
  sample_rows: string[][];
  total_rows: number;
  detected_mappings: ColumnMapping[];
  warnings: CsvWarning[];
}

export interface ImportResult {
  success: boolean;
  imported_count: number;
  failed_count: number;
  warnings: CsvWarning[];
  errors: CsvWarning[];
}

export async function parseCsvPreview(filePath: string): Promise<CsvPreview> {
  return await invoke<CsvPreview>('parse_csv_preview', {
    filePath,
  });
}

export async function importCsvData(
  filePath: string,
  columnMappings: ColumnMapping[]
): Promise<ImportResult> {
  return await invoke<ImportResult>('import_csv_data', {
    filePath,
    columnMappings,
  });
}
