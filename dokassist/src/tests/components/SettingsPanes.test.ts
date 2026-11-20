import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/svelte';
import { invoke } from '@tauri-apps/api/core';
import PracticePane from '../../lib/components/settings/PracticePane.svelte';
import UpdatesPane from '../../lib/components/settings/UpdatesPane.svelte';
import { normalizePracticeSettings } from '../../lib/components/PracticeForm.svelte';
import { sectionFromPath } from '../../lib/components/settings/sections';

vi.mock('$app/navigation', () => ({ goto: vi.fn() }));
vi.mock('@tauri-apps/api/event', () => ({ listen: vi.fn().mockResolvedValue(vi.fn()) }));
vi.mock('@tauri-apps/api/app', () => ({ getVersion: vi.fn().mockResolvedValue('0.23.0') }));

const mockInvoke = vi.mocked(invoke);

const STORED = {
  practice_name: 'Praxis Muster',
  practice_address: 'Seefeldstrasse 45\n8008 Zürich',
  practice_phone: null,
  practice_email: 'praxis@example.ch',
  therapist_name: 'Dr. med. V. Muster',
  zsr_number: 'Z123456',
  canton: 'ZH',
  clinical_specialty: null,
  language_preference: 'de',
  onboarding_completed: true,
};

beforeEach(() => {
  mockInvoke.mockReset();
});

describe('PracticePane', () => {
  it('loads the stored practice details and saves an edit through update_settings', async () => {
    mockInvoke.mockImplementation(async (cmd: string) => {
      if (cmd === 'get_settings') return { ...STORED };
      if (cmd === 'update_settings') return undefined;
      throw new Error(`unexpected ${cmd}`);
    });

    render(PracticePane);

    const name = await screen.findByLabelText('Practice name');
    expect(name).toHaveValue('Praxis Muster');
    const save = screen.getByRole('button', { name: 'Save changes' });
    expect(save).toBeDisabled();

    await fireEvent.input(name, { target: { value: '  Praxis Seefeld  ' } });
    await fireEvent.input(screen.getByLabelText('Phone'), {
      target: { value: '+41 44 555 66 77' },
    });
    expect(save).not.toBeDisabled();
    await fireEvent.click(save);

    await waitFor(() =>
      expect(mockInvoke).toHaveBeenCalledWith('update_settings', {
        settings: {
          ...STORED,
          practice_name: 'Praxis Seefeld',
          practice_phone: '+41 44 555 66 77',
        },
      })
    );
    expect(await screen.findByText('Saved')).toBeInTheDocument();
    expect(save).toBeDisabled();
  });

  it('does not offer the form when the stored details cannot be loaded', async () => {
    mockInvoke.mockRejectedValue(new Error('vault locked'));
    render(PracticePane);
    expect(await screen.findByText("Couldn't load the practice details.")).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'Save changes' })).not.toBeInTheDocument();
  });
});

describe('normalizePracticeSettings', () => {
  it('trims text and stores blanks as null, leaving the flags alone', () => {
    expect(
      normalizePracticeSettings({ ...STORED, practice_name: '   ', zsr_number: ' Z1 ', canton: '' })
    ).toEqual({ ...STORED, practice_name: null, zsr_number: 'Z1', canton: null });
  });
});

describe('UpdatesPane', () => {
  it('shows the version once', async () => {
    render(UpdatesPane);
    expect(await screen.findByText('0.23.0')).toBeInTheDocument();
    expect(screen.getAllByText(/0\.23\.0/)).toHaveLength(1);
  });

  it('replaces an earlier "up to date" with the error when a later check fails', async () => {
    mockInvoke
      .mockResolvedValueOnce({
        current_version: '0.23.0',
        latest_version: '0.23.0',
        update_available: false,
        body: null,
        date: null,
      })
      .mockRejectedValueOnce({ code: 'UPDATE_INFO_UNAVAILABLE', message: 'latest.json missing' });

    render(UpdatesPane);
    const check = screen.getByRole('button', { name: 'Check for Updates' });

    await fireEvent.click(check);
    expect(await screen.findByText("You're up to date")).toBeInTheDocument();

    await fireEvent.click(check);
    expect(await screen.findByText('Error checking for updates')).toBeInTheDocument();
    expect(screen.queryByText("You're up to date")).not.toBeInTheDocument();
    expect(mockInvoke).toHaveBeenCalledWith('check_for_updates');
  });

  it('renders release notes as markdown and offers the install', async () => {
    mockInvoke.mockResolvedValueOnce({
      current_version: '0.22.4',
      latest_version: '0.23.0',
      update_available: true,
      body: '## Neu\n- **Diktat** lokal\n- <script>x</script>',
      date: null,
    });
    render(UpdatesPane);
    await fireEvent.click(screen.getByRole('button', { name: 'Check for Updates' }));

    expect(await screen.findByText('Version 0.23.0 is available')).toBeInTheDocument();
    expect(screen.getByText('Diktat').tagName).toBe('STRONG');
    expect(document.querySelector('script')).toBeNull();
    expect(screen.getByRole('button', { name: 'Install Update' })).toBeInTheDocument();
  });
});

describe('sectionFromPath', () => {
  it('maps sub-routes to their section and /settings to none', () => {
    expect(sectionFromPath('/settings/models')?.labelKey).toBe('settings.nav.models');
    expect(sectionFromPath('/settings/danger-zone/')?.path).toBe('/settings/danger-zone');
    expect(sectionFromPath('/settings')).toBeNull();
  });
});
