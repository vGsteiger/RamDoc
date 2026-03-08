import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/svelte';
import { invoke } from '@tauri-apps/api/core';
import ReportView from '../../routes/patients/[id]/reports/[reportId]/+page.svelte';

vi.mock('$app/stores', () => {
  const { readable } = require('svelte/store');
  return {
    page: readable({
      params: { id: 'patient-1', reportId: 'report-1' },
      url: new URL('http://localhost/patients/patient-1/reports/report-1'),
      route: { id: null },
    }),
  };
});

vi.mock('$app/navigation', () => ({
  goto: vi.fn(),
}));

vi.mock('$lib/components/EnhancedReportEditor.svelte', () => ({
  default: vi.fn().mockReturnValue({ $$: {} }),
}));

const mockInvoke = vi.mocked(invoke);

const REPORT = {
  id: 'report-1',
  patient_id: 'patient-1',
  report_type: 'Befundbericht',
  content: 'Patient is in good health.',
  generated_at: '2026-03-08 10:00:00.000',
  model_name: 'Phi-4',
};

beforeEach(() => {
  mockInvoke.mockReset();

  // Mock URL APIs used by the blob download logic
  vi.stubGlobal('URL', {
    createObjectURL: vi.fn(() => 'blob:mock-url'),
    revokeObjectURL: vi.fn(),
  });
});

// ---------------------------------------------------------------------------
// Loading & rendering
// ---------------------------------------------------------------------------

describe('ReportView — loading', () => {
  it('shows loading state initially', () => {
    mockInvoke.mockReturnValue(new Promise(() => {}));
    render(ReportView);
    expect(screen.getByText('Loading report...')).toBeInTheDocument();
  });

  it('renders the report content after loading', async () => {
    mockInvoke.mockResolvedValueOnce(REPORT);
    render(ReportView);
    await waitFor(() => {
      expect(screen.getByText('Patient is in good health.')).toBeInTheDocument();
    });
  });

  it('renders the report type as the heading', async () => {
    mockInvoke.mockResolvedValueOnce(REPORT);
    render(ReportView);
    await waitFor(() => {
      expect(screen.getByText('Befundbericht')).toBeInTheDocument();
    });
  });
});

// ---------------------------------------------------------------------------
// Export buttons — presence & disabled state
// ---------------------------------------------------------------------------

describe('ReportView — export buttons', () => {
  it('renders Export PDF and Export DOCX buttons after loading', async () => {
    mockInvoke.mockResolvedValueOnce(REPORT);
    render(ReportView);
    await waitFor(() => {
      expect(screen.getByRole('button', { name: /export pdf/i })).toBeInTheDocument();
      expect(screen.getByRole('button', { name: /export docx/i })).toBeInTheDocument();
    });
  });

  it('export buttons are enabled in view mode', async () => {
    mockInvoke.mockResolvedValueOnce(REPORT);
    render(ReportView);
    await waitFor(() => {
      expect(screen.getByRole('button', { name: /export pdf/i })).not.toBeDisabled();
      expect(screen.getByRole('button', { name: /export docx/i })).not.toBeDisabled();
    });
  });

  it('export buttons are disabled while in edit mode', async () => {
    mockInvoke.mockResolvedValueOnce(REPORT);
    render(ReportView);
    await waitFor(() => screen.getByRole('button', { name: /edit/i }));
    await fireEvent.click(screen.getByRole('button', { name: /edit/i }));
    expect(screen.getByRole('button', { name: /export pdf/i })).toBeDisabled();
    expect(screen.getByRole('button', { name: /export docx/i })).toBeDisabled();
  });
});

// ---------------------------------------------------------------------------
// Export PDF — happy path & error
// ---------------------------------------------------------------------------

describe('ReportView — Export PDF', () => {
  it('calls export_report_to_pdf with the correct reportId', async () => {
    mockInvoke
      .mockResolvedValueOnce(REPORT)           // get_report
      .mockResolvedValueOnce([37, 80, 68, 70]); // export_report_to_pdf

    render(ReportView);
    await waitFor(() => screen.getByRole('button', { name: /export pdf/i }));
    await fireEvent.click(screen.getByRole('button', { name: /export pdf/i }));

    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith('export_report_to_pdf', { reportId: 'report-1' });
    });
  });

  it('triggers a file download after a successful PDF export', async () => {
    mockInvoke
      .mockResolvedValueOnce(REPORT)
      .mockResolvedValueOnce([37, 80, 68, 70]);

    render(ReportView);
    await waitFor(() => screen.getByRole('button', { name: /export pdf/i }));
    await fireEvent.click(screen.getByRole('button', { name: /export pdf/i }));

    await waitFor(() => {
      expect(URL.createObjectURL).toHaveBeenCalled();
      expect(URL.revokeObjectURL).toHaveBeenCalledWith('blob:mock-url');
    });
  });

  it('shows an error when PDF export fails', async () => {
    mockInvoke
      .mockResolvedValueOnce(REPORT)
      .mockRejectedValueOnce(new Error('PDF generation failed'));

    render(ReportView);
    await waitFor(() => screen.getByRole('button', { name: /export pdf/i }));
    await fireEvent.click(screen.getByRole('button', { name: /export pdf/i }));

    await waitFor(() => {
      expect(screen.getByText(/PDF generation failed/)).toBeInTheDocument();
    });
  });
});

// ---------------------------------------------------------------------------
// Export DOCX — happy path & error
// ---------------------------------------------------------------------------

describe('ReportView — Export DOCX', () => {
  it('calls export_report_to_docx with the correct reportId', async () => {
    mockInvoke
      .mockResolvedValueOnce(REPORT)
      .mockResolvedValueOnce([80, 75, 3, 4]); // export_report_to_docx

    render(ReportView);
    await waitFor(() => screen.getByRole('button', { name: /export docx/i }));
    await fireEvent.click(screen.getByRole('button', { name: /export docx/i }));

    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith('export_report_to_docx', { reportId: 'report-1' });
    });
  });

  it('triggers a file download after a successful DOCX export', async () => {
    mockInvoke
      .mockResolvedValueOnce(REPORT)
      .mockResolvedValueOnce([80, 75, 3, 4]);

    render(ReportView);
    await waitFor(() => screen.getByRole('button', { name: /export docx/i }));
    await fireEvent.click(screen.getByRole('button', { name: /export docx/i }));

    await waitFor(() => {
      expect(URL.createObjectURL).toHaveBeenCalled();
      expect(URL.revokeObjectURL).toHaveBeenCalledWith('blob:mock-url');
    });
  });

  it('shows an error when DOCX export fails', async () => {
    mockInvoke
      .mockResolvedValueOnce(REPORT)
      .mockRejectedValueOnce(new Error('DOCX generation failed'));

    render(ReportView);
    await waitFor(() => screen.getByRole('button', { name: /export docx/i }));
    await fireEvent.click(screen.getByRole('button', { name: /export docx/i }));

    await waitFor(() => {
      expect(screen.getByText(/DOCX generation failed/)).toBeInTheDocument();
    });
  });
});
