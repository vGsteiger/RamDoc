import { describe, expect, it } from 'vitest';
import { cleanGeneratedReport } from '$lib/llm/clean-generated-report';

describe('cleanGeneratedReport', () => {
  it('removes hidden reasoning and exact signature placeholder lines', () => {
    expect(
      cleanGeneratedReport(
        '<think>internal</think>\nMit freundlichen Grüßen\n[Name des Psychiaters]\n[Praxisname]\n[Adresse]'
      )
    ).toBe('Mit freundlichen Grüssen');
  });

  it('preserves clinical bracketed text and ordinary report content', () => {
    expect(cleanGeneratedReport('Diagnose\nF33.1 [gesichert]\nSertralin 100 mg')).toBe(
      'Diagnose\nF33.1 [gesichert]\nSertralin 100 mg'
    );
  });
});
