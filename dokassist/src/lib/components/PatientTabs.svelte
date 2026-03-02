<script lang="ts">
  import { page } from '$app/stores';

  interface Props {
    patientId: string;
  }

  let { patientId }: Props = $props();

  const tabs = [
    { path: `/patients/${patientId}`, label: 'Overview', icon: '📋' },
    { path: `/patients/${patientId}/sessions`, label: 'Sessions', icon: '🗓️' },
    { path: `/patients/${patientId}/files`, label: 'Files', icon: '📁' },
    { path: `/patients/${patientId}/diagnoses`, label: 'Diagnoses', icon: '🏥' },
    { path: `/patients/${patientId}/medications`, label: 'Medications', icon: '💊' },
    { path: `/patients/${patientId}/reports`, label: 'Reports', icon: '📄' }
  ];

  let currentPath = $derived($page.url.pathname);
</script>

<nav class="border-b border-gray-700">
  <div class="flex overflow-x-auto">
    {#each tabs as tab}
      <a
        href={tab.path}
        class="flex items-center gap-2 px-4 py-3 text-sm font-medium transition-colors whitespace-nowrap {currentPath === tab.path
          ? 'text-blue-400 border-b-2 border-blue-400'
          : 'text-gray-400 hover:text-gray-300'}"
      >
        <span>{tab.icon}</span>
        <span>{tab.label}</span>
      </a>
    {/each}
  </div>
</nav>
