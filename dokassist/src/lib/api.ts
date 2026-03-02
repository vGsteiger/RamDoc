import { invoke } from "@tauri-apps/api/core";
import type { AuthStatus } from "./stores/auth";

export async function checkAuth(): Promise<AuthStatus> {
  return await invoke<AuthStatus>("check_auth");
}

export async function initializeApp(): Promise<string[]> {
  return await invoke<string[]>("initialize_app");
}

export async function unlockApp(): Promise<boolean> {
  return await invoke<boolean>("unlock_app");
}

export async function recoverApp(words: string[]): Promise<boolean> {
  return await invoke<boolean>("recover_app", { words });
}

export async function lockApp(): Promise<void> {
  return await invoke<void>("lock_app");
}

export interface LlmEngineStatus {
  is_loaded: boolean;
  model_name: string | null;
  model_path: string | null;
  total_ram_bytes: number;
}

export async function getEngineStatus(): Promise<LlmEngineStatus> {
  return await invoke<LlmEngineStatus>("get_engine_status");
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
  mimeType: string,
): Promise<FileRecord> {
  return await invoke<FileRecord>("upload_file", {
    patient_id: patientId,
    filename,
    data,
    mime_type: mimeType,
  });
}

export async function downloadFile(fileId: string): Promise<number[]> {
  return await invoke<number[]>("download_file", { file_id: fileId });
}

export async function listFiles(patientId: string): Promise<FileRecord[]> {
  return await invoke<FileRecord[]>("list_files", { patient_id: patientId });
}

export async function deleteFile(fileId: string): Promise<void> {
  return await invoke<void>("delete_file", { file_id: fileId });
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
  return await invoke<Patient>("create_patient", { input });
}

export async function getPatient(id: string): Promise<Patient> {
  return await invoke<Patient>("get_patient", { id });
}

export async function listPatients(
  limit?: number,
  offset?: number,
): Promise<Patient[]> {
  return await invoke<Patient[]>("list_patients", { limit, offset });
}

export async function updatePatient(
  id: string,
  input: UpdatePatient,
): Promise<Patient> {
  return await invoke<Patient>("update_patient", { id, input });
}

export async function deletePatient(id: string): Promise<void> {
  return await invoke<void>("delete_patient", { id });
}

// === Search API ===

export async function globalSearch(
  query: string,
  limit?: number,
): Promise<SearchResult[]> {
  return await invoke<SearchResult[]>("global_search", { query, limit });
}

// Session types and commands
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

export async function listSessionsForPatient(
  patientId: string,
  limit?: number,
  offset?: number
): Promise<Session[]> {
  return await invoke<Session[]>('list_sessions_for_patient', {
    patientId,
    limit,
    offset
  });
}

export async function updateSession(id: string, input: UpdateSession): Promise<Session> {
  return await invoke<Session>('update_session', { id, input });
}

export async function deleteSession(id: string): Promise<void> {
  return await invoke<void>('delete_session', { id });
}

// Diagnosis types and commands
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
    offset
  });
}

export async function updateDiagnosis(id: string, input: UpdateDiagnosis): Promise<Diagnosis> {
  return await invoke<Diagnosis>('update_diagnosis', { id, input });
}

export async function deleteDiagnosis(id: string): Promise<void> {
  return await invoke<void>('delete_diagnosis', { id });
}

// Medication types and commands
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
    offset
  });
}

export async function updateMedication(id: string, input: UpdateMedication): Promise<Medication> {
  return await invoke<Medication>('update_medication', { id, input });
}

export async function deleteMedication(id: string): Promise<void> {
  return await invoke<void>('delete_medication', { id });
}
