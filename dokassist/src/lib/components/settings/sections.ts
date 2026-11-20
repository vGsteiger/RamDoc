import type { ComponentType } from 'svelte';
import {
  SlidersHorizontal,
  Building2,
  Cpu,
  Mic,
  BookOpen,
  DatabaseBackup,
  RefreshCw,
  TriangleAlert,
} from 'lucide-svelte';

/** Every Settings sub-page. These are URLs: add to them, never rename. */
export type SettingsPath =
  | '/settings/general'
  | '/settings/practice'
  | '/settings/models'
  | '/settings/dictation'
  | '/settings/reference-data'
  | '/settings/import-backup'
  | '/settings/updates'
  | '/settings/danger-zone';

export interface SettingsSection {
  path: SettingsPath;
  labelKey: string;
  icon: ComponentType;
}

/** The Settings sub-pages, in the order the section list shows them (D6). */
export const SETTINGS_SECTIONS: readonly SettingsSection[] = [
  { path: '/settings/general', labelKey: 'settings.nav.general', icon: SlidersHorizontal },
  { path: '/settings/practice', labelKey: 'settings.nav.practice', icon: Building2 },
  { path: '/settings/models', labelKey: 'settings.nav.models', icon: Cpu },
  { path: '/settings/dictation', labelKey: 'settings.nav.dictation', icon: Mic },
  { path: '/settings/reference-data', labelKey: 'settings.nav.referenceData', icon: BookOpen },
  { path: '/settings/import-backup', labelKey: 'settings.nav.importBackup', icon: DatabaseBackup },
  { path: '/settings/updates', labelKey: 'settings.nav.updates', icon: RefreshCw },
  { path: '/settings/danger-zone', labelKey: 'settings.nav.dangerZone', icon: TriangleAlert },
];

/** Where /settings itself lands. */
export const DEFAULT_SETTINGS_PATH: SettingsPath = '/settings/general';

/** The section a pathname belongs to, or null for /settings itself. */
export function sectionFromPath(pathname: string): SettingsSection | null {
  const clean = pathname.replace(/\/+$/, '');
  return (
    SETTINGS_SECTIONS.find((section) => clean === section.path || clean.endsWith(section.path)) ??
    null
  );
}
