<script lang="ts">
  import type { Session } from '$lib/api';

  interface Props {
    session: Session;
    onclick?: () => void;
  }

  let { session, onclick }: Props = $props();

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

  function getSnippet(notes: string | null): string {
    if (!notes) return 'Keine Notizen';
    return notes.length > 100 ? notes.substring(0, 100) + '...' : notes;
  }
</script>

<button
  type="button"
  class="w-full text-left p-4 bg-white dark:bg-gray-800 rounded-lg border border-gray-200 dark:border-gray-700 hover:border-blue-500 hover:bg-gray-50 dark:hover:bg-gray-700 transition-all"
  {onclick}
>
  <div class="flex justify-between items-start mb-2">
    <div class="flex-1">
      <h3 class="text-lg font-semibold text-gray-900 dark:text-gray-100">{session.session_type}</h3>
      <p class="text-sm text-gray-500 dark:text-gray-400">{formatDate(session.session_date)}</p>
    </div>
    {#if session.duration_minutes}
      <span class="text-sm text-gray-500 dark:text-gray-400">{session.duration_minutes} Min.</span>
    {/if}
  </div>
  <p class="text-sm text-gray-600 dark:text-gray-300 line-clamp-2">{getSnippet(session.notes)}</p>
</button>
