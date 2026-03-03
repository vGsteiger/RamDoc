import { describe, it, expect } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/svelte';
import AhvInput from '$lib/components/AhvInput.svelte';

// Valid AHV: 756.1234.5678.97  (EAN-13 checksum verified)
const VALID_AHV_DIGITS = '7561234567897';
// Invalid: same digits but wrong check digit
const INVALID_CHECKSUM_AHV = '7561234567890';

describe('AhvInput rendering', () => {
  it('renders an input with the correct placeholder', () => {
    render(AhvInput);
    expect(screen.getByPlaceholderText(/756\.__/)).toBeInTheDocument();
  });

  it('shows an external error prop when no internal error exists', () => {
    render(AhvInput, { error: 'AHV number is required' });
    expect(screen.getByText('AHV number is required')).toBeInTheDocument();
  });

  it('shows no error message initially', () => {
    render(AhvInput);
    expect(screen.queryByRole('paragraph')).not.toBeInTheDocument();
  });
});

describe('AhvInput validation', () => {
  it('shows a progress message while fewer than 13 digits are entered', async () => {
    render(AhvInput);
    const input = screen.getByRole('textbox');
    await fireEvent.input(input, { target: { value: '756123' } });
    expect(screen.getByText(/Enter 13 digits/)).toBeInTheDocument();
  });

  it('shows no error for an empty input', async () => {
    render(AhvInput);
    const input = screen.getByRole('textbox');
    await fireEvent.input(input, { target: { value: '' } });
    expect(screen.queryByText(/Enter 13 digits/)).not.toBeInTheDocument();
    expect(screen.queryByText(/Invalid AHV/)).not.toBeInTheDocument();
  });

  it('shows a checksum error for 13-digit AHV with wrong check digit', async () => {
    render(AhvInput);
    const input = screen.getByRole('textbox');
    await fireEvent.input(input, { target: { value: INVALID_CHECKSUM_AHV } });
    expect(screen.getByText('Invalid AHV checksum')).toBeInTheDocument();
  });

  it('shows no error for a fully valid AHV number', async () => {
    render(AhvInput);
    const input = screen.getByRole('textbox');
    await fireEvent.input(input, { target: { value: VALID_AHV_DIGITS } });
    expect(screen.queryByText('Invalid AHV checksum')).not.toBeInTheDocument();
    expect(screen.queryByText(/Enter 13 digits/)).not.toBeInTheDocument();
  });

  it('shows an error when more than 13 digits are entered', async () => {
    render(AhvInput);
    const input = screen.getByRole('textbox');
    await fireEvent.input(input, { target: { value: '75612345678971' } }); // 14 digits
    expect(screen.getByText('AHV must contain exactly 13 digits')).toBeInTheDocument();
  });
});
