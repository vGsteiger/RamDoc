import { invoke } from '@tauri-apps/api/core';
import type { AuthStatus } from './stores/auth';

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

export interface LlmEngineStatus {
  is_loaded: boolean;
  model_name: string | null;
  model_path: string | null;
  total_ram_bytes: number;
}

export async function getEngineStatus(): Promise<LlmEngineStatus> {
  return await invoke<LlmEngineStatus>('get_engine_status');
}

// === Patient Types ===

export interface Patient {
  id: string;
  ahv_number: string;
  first_name: string;
  last_name: string;
  date_of_birth: string;
  gender: string | null;
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
