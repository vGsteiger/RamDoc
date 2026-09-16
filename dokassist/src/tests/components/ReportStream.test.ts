import { describe, it, expect } from 'vitest';
import { render, screen } from '@testing-library/svelte';
import ReportStream from '$lib/components/ReportStream.svelte';

describe('ReportStream — idle (not streaming)', () => {
  it('shows placeholder text when content is empty and not streaming', () => {
    render(ReportStream, { content: '', isStreaming: false });
    expect(screen.getByText(/report appears here/i)).toBeInTheDocument();
  });

  it('renders provided content', () => {
    render(ReportStream, { content: 'Patient summary report', isStreaming: false });
    expect(screen.getByText('Patient summary report')).toBeInTheDocument();
  });

  it('does not show the writing indicator when not streaming', () => {
    render(ReportStream, { content: 'Some content', isStreaming: false });
    expect(screen.queryByText('Writing the report')).not.toBeInTheDocument();
  });
});

describe('ReportStream — streaming', () => {
  it('shows the writing indicator when isStreaming is true with partial content', () => {
    render(ReportStream, { content: 'Partial report...', isStreaming: true });
    expect(screen.getByRole('status')).toHaveTextContent('Writing the report');
  });

  it('shows a thinking status when streaming with no content yet', () => {
    render(ReportStream, { content: '', isStreaming: true });
    expect(screen.getByRole('status')).toHaveTextContent('Thinking');
  });

  it('shows both the writing indicator and partial content while streaming', () => {
    render(ReportStream, { content: 'Partial output so far...', isStreaming: true });
    expect(screen.getByRole('status')).toHaveTextContent('Writing the report');
    expect(screen.getByText('Partial output so far...')).toBeInTheDocument();
  });

  it('does not show the placeholder when streaming (even with no content)', () => {
    render(ReportStream, { content: '', isStreaming: true });
    expect(screen.queryByText(/report appears here/i)).not.toBeInTheDocument();
  });
});
