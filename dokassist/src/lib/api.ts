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
