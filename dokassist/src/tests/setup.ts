import '@testing-library/jest-dom/vitest';
import { vi } from 'vitest';

// Mock Tauri IPC globally so tests never try to reach a native bridge.
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));
