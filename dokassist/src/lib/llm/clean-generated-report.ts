import { stripThinkTags } from './strip-think';

const SIGNATURE_PLACEHOLDER =
  /^\s*\[(?:name|praxis|adresse|telefon|e-?mail|unterschrift)[^\]]*\]\s*$/i;

/** Normalize model-only artifacts without changing clinical prose. */
export function cleanGeneratedReport(content: string): string {
  const lines = stripThinkTags(content)
    .replaceAll('ß', 'ss')
    .split('\n')
    .filter((line) => !SIGNATURE_PLACEHOLDER.test(line));

  while (lines.length > 0 && lines.at(-1)?.trim() === '') lines.pop();
  return lines.join('\n').trim();
}
