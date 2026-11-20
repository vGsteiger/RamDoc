/** Model sizes and RAM are shown in GB with one decimal, as before the split. */
export function formatGigabytes(bytes: number): string {
  return `${(bytes / 1024 ** 3).toFixed(1)} GB`;
}

/** Whether a machine with `totalRamBytes` meets a model's minimum. */
export function hasEnoughRam(totalRamBytes: number | null | undefined, minRamGb: number): boolean {
  return totalRamBytes ? totalRamBytes >= minRamGb * 1024 ** 3 : false;
}

/**
 * The backend catalogue (download.rs) ships English descriptions. The UI
 * shows the translated text for known models and falls back to the
 * catalogue's own text for anything newer than the translation files.
 */
export function catalogText(
  translate: (key: string) => string,
  kind: 'descriptions' | 'disclaimers',
  filename: string,
  fallback: string
): string {
  const key = `modelCatalog.${kind}.${filename.replace(/[^A-Za-z0-9]/g, '_')}`;
  const text = translate(key);
  return text === key ? fallback : text;
}
