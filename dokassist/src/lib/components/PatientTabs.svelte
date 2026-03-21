<script lang="ts">
  import { page } from '$app/stores';
  import { ClipboardList, CalendarDays, FolderOpen, Hospital, Pill, FileText, ClipboardCheck } from 'lucide-svelte';

  interface Props {
    patientId: string;
  }

  let { patientId }: Props = $props();

  const tabs = [
    { path: `/patients/${patientId}`, label: 'Overview', icon: ClipboardList },
    { path: `/patients/${patientId}/sessions`, label: 'Sessions', icon: CalendarDays },
    { path: `/patients/${patientId}/files`, label: 'Files', icon: FolderOpen },
    { path: `/patients/${patientId}/diagnoses`, label: 'Diagnoses', icon: Hospital },
    { path: `/patients/${patientId}/medications`, label: 'Medications', icon: Pill },
    { path: `/patients/${patientId}/treatment-plans`, label: 'Treatment Plans', icon: ClipboardCheck },
    { path: `/patients/${patientId}/reports`, label: 'Reports', icon: FileText }
  ];

  let currentPath = $derived($page.url.pathname);
</script>

<nav class="border-b border-gray-700">
  <div class="flex overflow-x-auto">
    {#each tabs as tab}
      {@const Icon = tab.icon}
      <a
        href={tab.path}
        class="flex items-center gap-2 px-4 py-3 text-sm font-medium transition-colors whitespace-nowrap {currentPath === tab.path
          ? 'text-blue-400 border-b-2 border-blue-400'
          : 'text-gray-400 hover:text-gray-300'}"
      >
        <Icon size={16} />
        <span>{tab.label}</span>
      </a>
    {/each}
  </div>
</nav>
