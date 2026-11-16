<script lang="ts">
  import { t } from '$lib/translations';
  import { page } from '$app/state';
  import {
    ClipboardList,
    CalendarDays,
    FolderOpen,
    Hospital,
    Pill,
    FileText,
    ClipboardCheck,
  } from 'lucide-svelte';

  interface Props {
    patientId: string;
  }

  let { patientId }: Props = $props();

  const tabs = $derived([
    { path: `/patients/${patientId}`, label: $t('patients.overview'), icon: ClipboardList },
    { path: `/patients/${patientId}/sessions`, label: $t('patients.sessions'), icon: CalendarDays },
    { path: `/patients/${patientId}/files`, label: $t('patients.files'), icon: FolderOpen },
    { path: `/patients/${patientId}/diagnoses`, label: $t('patients.diagnoses'), icon: Hospital },
    { path: `/patients/${patientId}/medications`, label: $t('patients.medications'), icon: Pill },
    {
      path: `/patients/${patientId}/treatment-plans`,
      label: $t('treatmentPlans.title'),
      icon: ClipboardCheck,
    },
    { path: `/patients/${patientId}/reports`, label: $t('patients.reports'), icon: FileText },
  ]);

  let currentPath = $derived(page.url.pathname);
</script>

<nav class="border-b border-line">
  <div class="flex overflow-x-auto">
    {#each tabs as tab}
      {@const Icon = tab.icon}
      <a
        href={tab.path}
        class="flex items-center gap-2 px-4 py-3 text-body font-medium transition-colors whitespace-nowrap {currentPath ===
        tab.path
          ? 'text-accent-fg border-b-2 border-accent'
          : 'text-fg-muted hover:text-fg'}"
      >
        <Icon size={16} />
        <span>{tab.label}</span>
      </a>
    {/each}
  </div>
</nav>
