import { describe, expect, it } from 'vitest';
import { cleanGeneratedReport } from '$lib/llm/clean-generated-report';

describe('cleanGeneratedReport', () => {
  it('removes hidden reasoning and exact signature placeholder lines without rewriting prose', () => {
    expect(
      cleanGeneratedReport(
        '<think>internal</think>\nMit freundlichen Grüßen\n[Name des Psychiaters]\n[Praxisname]\n[Adresse]'
      )
    ).toBe('Mit freundlichen Grüßen');
  });

  it('preserves clinical bracketed text and ordinary report content', () => {
    expect(cleanGeneratedReport('Diagnose\nF33.1 [gesichert]\nSertralin 100 mg')).toBe(
      'Diagnose\nF33.1 [gesichert]\nSertralin 100 mg'
    );
  });

  it('preserves legitimate ß characters in names and addresses', () => {
    expect(cleanGeneratedReport('Dr. Weiß\nGoethestraße 4\nGrüßen')).toBe(
      'Dr. Weiß\nGoethestraße 4\nGrüßen'
    );
  });
});
