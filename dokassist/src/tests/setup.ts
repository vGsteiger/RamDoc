import '@testing-library/jest-dom/vitest';
import { vi } from 'vitest';

// Mock Tauri IPC globally so tests never try to reach a native bridge.
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

// Provide a working localStorage for all tests.
// jsdom's localStorage can be unavailable when --localstorage-file is not set;
// this mock runs before any module-level store initialisation.
const _localStorageStore: Record<string, string> = {};
const localStorageMock: Storage = {
  getItem: (key: string) => _localStorageStore[key] ?? null,
  setItem: (key: string, value: string) => { _localStorageStore[key] = String(value); },
  removeItem: (key: string) => { delete _localStorageStore[key]; },
  clear: () => { Object.keys(_localStorageStore).forEach(k => delete _localStorageStore[k]); },
  key: (index: number) => Object.keys(_localStorageStore)[index] ?? null,
  get length() { return Object.keys(_localStorageStore).length; },
};
Object.defineProperty(globalThis, 'localStorage', { value: localStorageMock, writable: true });
