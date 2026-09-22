import { beforeEach, describe, expect, it, vi } from 'vitest';
import { invoke } from '@tauri-apps/api/core';
import { fireEvent, render, screen } from '@testing-library/svelte';
import ContextPicker from '$lib/components/context-picker/ContextPicker.svelte';
import {
  createContextPlan,
  refreshContextPlan,
  selectContextPatient,
  setComparisonMode,
  toggleContextSource,
  togglePinnedContextSource,
} from '$lib/components/context-picker/state';
import type { Patient } from '$lib/api';

const ada: Patient = {
  id: 'ada',
  first_name: 'Ada',
  last_name: 'Lovelace',
  date_of_birth: '1815-12-10',
  gender: null,
  ahv_number: null,
  address: null,
  phone: null,
  email: null,
  insurance: null,
  gp_name: null,
  gp_address: null,
  notes: null,
  created_at: '',
  updated_at: '',
};
const grace: Patient = { ...ada, id: 'grace', first_name: 'Grace', last_name: 'Hopper' };

describe('context plan state', () => {
  beforeEach(() => {
    vi.mocked(invoke).mockReset();
  });
  it('keeps exactly one patient outside explicit comparison mode', () => {
    let plan = selectContextPatient(createContextPlan(), ada);
    plan = selectContextPatient(plan, grace);
    expect(plan.selectedPatients.map((patient) => patient.id)).toEqual(['grace']);
  });

  it('only retains multiple patients while comparison mode is explicit', () => {
    let plan = setComparisonMode(createContextPlan(), true);
    plan = selectContextPatient(selectContextPatient(plan, ada), grace);
    expect(plan.selectedPatients).toHaveLength(2);
    expect(setComparisonMode(plan, false).selectedPatients).toHaveLength(1);
  });

  it('pins sources as included and prevents source toggling from removing them', () => {
    let plan = togglePinnedContextSource(createContextPlan(), 'files');
    expect(plan.sources.find((source) => source.kind === 'files')).toMatchObject({
      included: true,
      pinned: true,
    });
    plan = toggleContextSource(plan, 'files');
    expect(plan.sources.find((source) => source.kind === 'files')?.included).toBe(true);
  });

  it('aggregates planning counts across selected patients in comparison mode', async () => {
    vi.mocked(invoke).mockImplementation(async (command, args) => {
      const patientId =
        (args as { patientId?: string; id?: string }).patientId ??
        (args as { patientId?: string; id?: string }).id;
      if (!patientId) throw new Error(`Missing patient id for ${command}`);
      if (command === 'get_patient') return { id: patientId };
      if (command === 'list_diagnoses_for_patient') return patientId === 'ada' ? [{}] : [{}, {}];
      if (command === 'list_medications_for_patient') return [{}];
      if (command === 'list_sessions_for_patient') return patientId === 'ada' ? [{}, {}] : [{}];
      return [];
    });

    let plan = setComparisonMode(createContextPlan(), true);
    plan = selectContextPatient(selectContextPatient(plan, ada), grace);
    const refreshed = await refreshContextPlan(plan);

    expect(refreshed.planning.retrieved).toBe(10);
    expect(refreshed.planning.summarized).toBe(10);
  });
});

describe('ContextPicker', () => {
  it('searches and selects a patient with accessible source controls', async () => {
    render(ContextPicker, { patients: [ada, grace] });
    await fireEvent.input(screen.getByRole('searchbox'), { target: { value: 'grace' } });
    expect(screen.getByRole('button', { name: /grace hopper/i })).toBeInTheDocument();
    await fireEvent.click(screen.getByRole('button', { name: /grace hopper/i }));
    expect(screen.getByLabelText(/clear selected patients: grace hopper/i)).toBeInTheDocument();
    expect(screen.getByRole('checkbox', { name: 'Overview' })).toBeChecked();
  });
});
