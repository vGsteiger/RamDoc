/** Small presentation helpers for patient files. */
import type { ComponentType } from 'svelte';
import { File, FileImage, FileMusic, FileText } from 'lucide-svelte';
import { isAudioMime } from '$lib/api';

/** 1023 B · 88.9 KB · 1.7 MB */
export function formatFileSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

/** `report.final.PDF` → `PDF`; no extension → ''. */
export function fileExtension(filename: string): string {
  const parts = filename.split('.');
  return parts.length > 1 ? parts[parts.length - 1].toUpperCase() : '';
}

export function fileIcon(mimeType: string): ComponentType {
  if (isAudioMime(mimeType)) return FileMusic as unknown as ComponentType;
  if (mimeType.startsWith('image/')) return FileImage as unknown as ComponentType;
  if (mimeType === 'application/pdf' || mimeType.includes('word')) {
    return FileText as unknown as ComponentType;
  }
  return File as unknown as ComponentType;
}
