import { stripThinkTags } from './strip-think';

const SIGNATURE_PLACEHOLDER =
  /^\s*\[(?:name|praxis|adresse|telefon|e-?mail|unterschrift)[^\]]*\]\s*$/i;

type CleanGeneratedReportOptions = {
  normalizeSwissOrthography?: boolean;
};

/** Normalize model-only artifacts without changing clinical prose. */
export function cleanGeneratedReport(
  content: string,
  { normalizeSwissOrthography = false }: CleanGeneratedReportOptions = {}
): string {
  const normalizedContent = normalizeSwissOrthography ? content.replaceAll('ß', 'ss') : content;
  const lines = stripThinkTags(normalizedContent)
    .split('\n')
    .filter((line) => !SIGNATURE_PLACEHOLDER.test(line));

  while (lines.length > 0 && lines.at(-1)?.trim() === '') lines.pop();
  return lines.join('\n').trim();
}
