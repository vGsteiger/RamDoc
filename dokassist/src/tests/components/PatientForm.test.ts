import { describe, it, expect } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/svelte';
import PatientForm from '$lib/components/PatientForm.svelte';
import type { Patient } from '$lib/api';

const MOCK_PATIENT: Patient = {
  id: 'p1',
  first_name: 'Jane',
  last_name: 'Doe',
  date_of_birth: '1990-01-15',
  ahv_number: '7561234567897',
  gender: 'female',
  address: '123 Test St',
  phone: '+41 79 123 45 67',
  email: 'jane@example.com',
  insurance: 'CSS',
  gp_name: 'Dr. Smith',
  gp_address: '456 Medical Ave',
  notes: 'Test notes',
  created_at: '2025-01-01T00:00:00Z',
  updated_at: '2025-01-01T00:00:00Z',
};

describe('PatientForm — create mode', () => {
  it('renders "Create Patient" submit button when no patient prop is provided', () => {
    render(PatientForm);
    expect(screen.getByRole('button', { name: /create patient/i })).toBeInTheDocument();
  });

  it('renders a Cancel button', () => {
    render(PatientForm);
    expect(screen.getByRole('button', { name: /cancel/i })).toBeInTheDocument();
  });

  it('shows first-name required error on empty submit', async () => {
    const { container } = render(PatientForm);
    const form = container.querySelector('form')!;
    await fireEvent.submit(form);
    expect(screen.getByText(/first name.*required/i)).toBeInTheDocument();
  });

  it('shows last-name required error on empty submit', async () => {
    const { container } = render(PatientForm);
    await fireEvent.submit(container.querySelector('form')!);
    expect(screen.getByText(/last name.*required/i)).toBeInTheDocument();
  });

  it('shows date-of-birth required error on empty submit', async () => {
    const { container } = render(PatientForm);
    await fireEvent.submit(container.querySelector('form')!);
    expect(screen.getByText(/date of birth.*required/i)).toBeInTheDocument();
  });

  it('disables the submit button while isSubmitting is true', () => {
    render(PatientForm, { isSubmitting: true });
    const submitBtn = screen.getByRole('button', { name: /saving/i });
    expect(submitBtn).toBeDisabled();
  });
});

describe('PatientForm — update mode', () => {
  it('renders "Update Patient" submit button when a patient is provided', () => {
    render(PatientForm, { patient: MOCK_PATIENT });
    expect(screen.getByRole('button', { name: /update patient/i })).toBeInTheDocument();
  });

  it('pre-fills the first name input', () => {
    render(PatientForm, { patient: MOCK_PATIENT });
    expect(screen.getByDisplayValue('Jane')).toBeInTheDocument();
  });

  it('pre-fills the last name input', () => {
    render(PatientForm, { patient: MOCK_PATIENT });
    expect(screen.getByDisplayValue('Doe')).toBeInTheDocument();
  });

  it('pre-fills the email input', () => {
    render(PatientForm, { patient: MOCK_PATIENT });
    expect(screen.getByDisplayValue('jane@example.com')).toBeInTheDocument();
  });

  it('pre-fills the insurance input', () => {
    render(PatientForm, { patient: MOCK_PATIENT });
    expect(screen.getByDisplayValue('CSS')).toBeInTheDocument();
  });
});
