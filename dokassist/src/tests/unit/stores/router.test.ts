import { beforeEach, describe, expect, it, vi } from 'vitest';
import { get } from 'svelte/store';
import { invoke } from '@tauri-apps/api/core';
import {
  refreshRouterDiagnostics,
  resetRouterState,
  router,
  setRouterOverride,
  unloadRouter,
} from '$lib/stores/router';

const mockInvoke = vi.mocked(invoke);
const DIAGNOSTICS = {
  role: 'tool_router' as const,
  mode: 'managed' as const,
  selection_source: 'smallest_compatible_installed' as const,
  managed_model_filename: 'small.gguf',
  selected_model_filename: 'small.gguf',
  active_model_filename: null,
  resident_engine_count: 1,
  lifecycle: {
    phase: 'idle' as const,
    requested_filename: null,
    active_filename: null,
    error: null,
  },
  residency: 'unloaded' as const,
  dual_residency_safe: true,
  total_ram_bytes: 32,
  dual_residency_weight_budget_bytes: 16,
  estimated_resident_weight_bytes: 12,
  fallback_reason: null,
  benchmark: {
    probe_count: 0,
    dedicated_probe_count: 0,
    writing_fallback_count: 0,
    last_probe_duration_ms: null,
  },
  advanced_override: {
    available: true,
    configured_filename: null,
    requires_separate_engine: true,
    limitation: 'Final prose remains writing.',
  },
};

beforeEach(() => {
  mockInvoke.mockReset();
  resetRouterState();
});

describe('router store', () => {
  it('keeps the backend residency diagnostic authoritative', async () => {
    mockInvoke.mockResolvedValueOnce(DIAGNOSTICS);
    await refreshRouterDiagnostics();
    expect(mockInvoke).toHaveBeenCalledWith('get_router_diagnostics');
    expect(get(router).diagnostics?.residency).toBe('unloaded');
  });

  it('persists an override then refreshes the selected runtime state', async () => {
    mockInvoke.mockResolvedValueOnce(undefined).mockResolvedValueOnce({
      ...DIAGNOSTICS,
      mode: 'advanced_override',
      selection_source: 'expert_override',
      advanced_override: { ...DIAGNOSTICS.advanced_override, configured_filename: 'expert.gguf' },
    });
    await setRouterOverride('expert-id');
    expect(mockInvoke).toHaveBeenNthCalledWith(1, 'set_router_model_override', {
      modelId: 'expert-id',
    });
    expect(get(router).diagnostics?.selection_source).toBe('expert_override');
  });

  it('unloads the dedicated role before refreshing diagnostics', async () => {
    mockInvoke.mockResolvedValueOnce(undefined).mockResolvedValueOnce(DIAGNOSTICS);
    await unloadRouter();
    expect(mockInvoke).toHaveBeenNthCalledWith(1, 'unload_router_model');
    expect(mockInvoke).toHaveBeenNthCalledWith(2, 'get_router_diagnostics');
  });
});
