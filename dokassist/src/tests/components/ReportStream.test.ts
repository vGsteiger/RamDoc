import { describe, it, expect } from 'vitest';
import { render, screen } from '@testing-library/svelte';
import ReportStream from '$lib/components/ReportStream.svelte';

describe('ReportStream — idle (not streaming)', () => {
  it('shows placeholder text when content is empty and not streaming', () => {
    render(ReportStream, { content: '', isStreaming: false });
    expect(screen.getByText(/Report will appear here/i)).toBeInTheDocument();
  });

  it('renders provided content', () => {
    render(ReportStream, { content: 'Patient summary report', isStreaming: false });
    expect(screen.getByText('Patient summary report')).toBeInTheDocument();
  });

  it('does not show the Generating indicator when not streaming', () => {
    render(ReportStream, { content: 'Some content', isStreaming: false });
    expect(screen.queryByText('Generating...')).not.toBeInTheDocument();
  });
});

describe('ReportStream — streaming', () => {
  it('shows the Generating indicator when isStreaming is true', () => {
    render(ReportStream, { content: '', isStreaming: true });
    expect(screen.getByText('Generating...')).toBeInTheDocument();
  });

  it('shows the "Waiting for LLM..." message when streaming with no content yet', () => {
    render(ReportStream, { content: '', isStreaming: true });
    expect(screen.getByText('Waiting for LLM...')).toBeInTheDocument();
  });

  it('shows both the Generating indicator and partial content while streaming', () => {
    render(ReportStream, { content: 'Partial output so far...', isStreaming: true });
    expect(screen.getByText('Generating...')).toBeInTheDocument();
    expect(screen.getByText('Partial output so far...')).toBeInTheDocument();
  });

  it('does not show the placeholder when streaming (even with no content)', () => {
    render(ReportStream, { content: '', isStreaming: true });
    expect(screen.queryByText(/Report will appear here/i)).not.toBeInTheDocument();
  });
});
