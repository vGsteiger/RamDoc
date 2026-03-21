<script lang="ts">
  import type { Diagnosis } from '$lib/api';

  interface Props {
    diagnosis: Diagnosis;
    onEdit?: () => void;
    onDelete?: () => void;
  }

  let { diagnosis, onEdit, onDelete }: Props = $props();

  function formatDate(dateStr: string): string {
    try {
      const date = new Date(dateStr);
      return date.toLocaleDateString('de-CH', {
        year: 'numeric',
        month: '2-digit',
        day: '2-digit',
      });
    } catch {
      return dateStr;
    }
  }

  function getStatusColor(status: string): string {
    switch (status) {
      case 'active':
        return 'bg-green-500/20 text-green-400 border-green-500/30';
      case 'remission':
        return 'bg-yellow-500/20 text-yellow-400 border-yellow-500/30';
      case 'resolved':
        return 'bg-gray-500/20 text-gray-400 border-gray-500/30';
      default:
        return 'bg-blue-500/20 text-blue-400 border-blue-500/30';
    }
  }

  function getStatusLabel(status: string): string {
    switch (status) {
      case 'active':
        return 'Aktiv';
      case 'remission':
        return 'Remission';
      case 'resolved':
        return 'Aufgelöst';
      default:
        return status;
    }
  }
</script>

<div class="p-4 bg-white dark:bg-gray-800 rounded-lg border border-gray-200 dark:border-gray-700">
  <div class="flex justify-between items-start mb-2">
    <div class="flex-1">
      <div class="flex items-center gap-2 mb-1">
        <span class="font-mono text-sm text-blue-600 dark:text-blue-400"
          >{diagnosis.icd10_code}</span
        >
        <span class="px-2 py-0.5 rounded-full text-xs border {getStatusColor(diagnosis.status)}">
          {getStatusLabel(diagnosis.status)}
        </span>
      </div>
      <h3 class="text-base font-medium text-gray-900 dark:text-gray-100">
        {diagnosis.description}
      </h3>
      <p class="text-sm text-gray-500 dark:text-gray-400 mt-1">
        Diagnostiziert: {formatDate(diagnosis.diagnosed_date)}
        {#if diagnosis.resolved_date}
          • Aufgelöst: {formatDate(diagnosis.resolved_date)}
        {/if}
      </p>
    </div>
    <div class="flex gap-2 ml-2">
      {#if onEdit}
        <button
          type="button"
          class="p-2 text-gray-400 hover:text-blue-500 dark:hover:text-blue-400 hover:bg-gray-100 dark:hover:bg-gray-700 rounded transition-colors"
          onclick={onEdit}
          title="Bearbeiten"
        >
          <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path
              stroke-linecap="round"
              stroke-linejoin="round"
              stroke-width="2"
              d="M11 5H6a2 2 0 00-2 2v11a2 2 0 002 2h11a2 2 0 002-2v-5m-1.414-9.414a2 2 0 112.828 2.828L11.828 15H9v-2.828l8.586-8.586z"
            />
          </svg>
        </button>
      {/if}
      {#if onDelete}
        <button
          type="button"
          class="p-2 text-gray-400 hover:text-red-500 dark:hover:text-red-400 hover:bg-gray-100 dark:hover:bg-gray-700 rounded transition-colors"
          onclick={onDelete}
          title="Löschen"
        >
          <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path
              stroke-linecap="round"
              stroke-linejoin="round"
              stroke-width="2"
              d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16"
            />
          </svg>
        </button>
      {/if}
    </div>
  </div>
  {#if diagnosis.notes}
    <p class="text-sm text-gray-600 dark:text-gray-300 mt-2">{diagnosis.notes}</p>
  {/if}
</div>
