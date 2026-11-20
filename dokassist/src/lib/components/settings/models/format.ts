/** Model sizes and RAM are shown in GB with one decimal, as before the split. */
export function formatGigabytes(bytes: number): string {
  return `${(bytes / 1024 ** 3).toFixed(1)} GB`;
}

/** Whether a machine with `totalRamBytes` meets a model's minimum. */
export function hasEnoughRam(totalRamBytes: number | null | undefined, minRamGb: number): boolean {
  return totalRamBytes ? totalRamBytes >= minRamGb * 1024 ** 3 : false;
}
