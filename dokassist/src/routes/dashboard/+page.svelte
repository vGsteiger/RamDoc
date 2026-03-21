<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { getDashboardData, type DashboardData } from '$lib/api';
  import { t } from '$lib/translations';
  import { language } from '$lib/stores/language';
  import { Calendar, Users, FileText, Plus } from 'lucide-svelte';

  let data = $state<DashboardData | null>(null);
  let isLoading = $state(true);
  let error = $state<string | null>(null);

  function getSessionTypeLabel(sessionType: string): string {
    const key = `sessions.types.${sessionType}`;
    const translated = $t(key);
    // If translation doesn't exist, $t returns the key itself
    return translated === key ? sessionType : translated;
  }

  function formatDate(isoDate: string): string {
    const d = new Date(isoDate + 'T00:00:00');
    const locale = $language === 'de' ? 'de-CH' : 'en-US';
    return d.toLocaleDateString(locale, {
      day: 'numeric',
      month: 'long',
      year: 'numeric',
    });
  }

  onMount(async () => {
    try {
      data = await getDashboardData();
    } catch (err) {
      console.error('Failed to load dashboard data:', err);
      error = err instanceof Error ? err.message : $t('dashboard.loadError');
    } finally {
      isLoading = false;
    }
  });
</script>

<div class="p-8 max-w-7xl mx-auto">
  <div class="mb-8">
    <h1 class="text-3xl font-bold text-gray-900 dark:text-gray-100">{$t('dashboard.title')}</h1>
  </div>

  {#if isLoading}
    <div class="text-center py-12">
      <div class="animate-spin rounded-full h-12 w-12 border-b-2 border-blue-500 mx-auto"></div>
      <p class="mt-4 text-gray-500 dark:text-gray-400">{$t('common.loading')}</p>
    </div>
  {:else if error}
    <div class="bg-red-900/20 border border-red-500 rounded-lg p-6 text-center">
      <p class="text-red-400">{error}</p>
    </div>
  {:else if data}
    <div class="grid grid-cols-1 lg:grid-cols-3 gap-6">
      <!-- Today's Sessions -->
      <div class="bg-white dark:bg-gray-800 border border-gray-200 dark:border-gray-700 rounded-lg p-6">
        <div class="flex items-center gap-3 mb-4">
          <div class="p-2 bg-blue-100 dark:bg-blue-900 rounded-lg">
            <Calendar size={20} class="text-blue-600 dark:text-blue-300" />
          </div>
          <h2 class="text-lg font-semibold text-gray-900 dark:text-gray-100">{$t('dashboard.todaysSessions')}</h2>
        </div>

        {#if data.todays_sessions.length === 0}
          <p class="text-sm text-gray-500 dark:text-gray-400">{$t('dashboard.noSessionsToday')}</p>
        {:else}
          <div class="space-y-3">
            {#each data.todays_sessions as item}
              <button
                onclick={() => goto(`/patients/${item.session.patient_id}/sessions`)}
                class="w-full text-left bg-gray-50 dark:bg-gray-700 hover:bg-gray-100 dark:hover:bg-gray-600 rounded-lg p-3 transition-colors"
              >
                <p class="text-sm font-medium text-gray-900 dark:text-gray-100 truncate">{item.patient_name}</p>
                <div class="flex items-center gap-2 mt-1">
                  <span class="text-xs px-2 py-0.5 rounded-full bg-blue-100 dark:bg-blue-900 text-blue-700 dark:text-blue-200">
                    {getSessionTypeLabel(item.session.session_type)}
                  </span>
                  {#if item.session.duration_minutes}
                    <span class="text-xs text-gray-500 dark:text-gray-400">{item.session.duration_minutes} {$t('dashboard.minutes')}</span>
                  {/if}
                </div>
              </button>
            {/each}
          </div>
        {/if}

        <button
          onclick={() => goto('/calendar')}
          class="w-full mt-4 px-4 py-2 text-sm font-medium text-blue-600 dark:text-blue-400 hover:bg-blue-50 dark:hover:bg-blue-900/20 rounded-lg transition-colors"
        >
          {$t('dashboard.viewCalendar')}
        </button>
      </div>

      <!-- Recent Patients -->
      <div class="bg-white dark:bg-gray-800 border border-gray-200 dark:border-gray-700 rounded-lg p-6">
        <div class="flex items-center gap-3 mb-4">
          <div class="p-2 bg-green-100 dark:bg-green-900 rounded-lg">
            <Users size={20} class="text-green-600 dark:text-green-300" />
          </div>
          <h2 class="text-lg font-semibold text-gray-900 dark:text-gray-100">{$t('dashboard.recentPatients')}</h2>
        </div>

        {#if data.recent_patients.length === 0}
          <p class="text-sm text-gray-500 dark:text-gray-400">{$t('dashboard.noRecentPatients')}</p>
        {:else}
          <div class="space-y-3">
            {#each data.recent_patients as patient}
              <button
                onclick={() => goto(`/patients/${patient.id}`)}
                class="w-full text-left bg-gray-50 dark:bg-gray-700 hover:bg-gray-100 dark:hover:bg-gray-600 rounded-lg p-3 transition-colors"
              >
                <p class="text-sm font-medium text-gray-900 dark:text-gray-100">
                  {patient.first_name} {patient.last_name}
                </p>
                <p class="text-xs text-gray-500 dark:text-gray-400 mt-1">
                  {formatDate(patient.date_of_birth)}
                </p>
              </button>
            {/each}
          </div>
        {/if}

        <div class="flex gap-2 mt-4">
          <button
            onclick={() => goto('/patients/new')}
            class="flex-1 px-4 py-2 text-sm font-medium text-white bg-green-600 hover:bg-green-700 rounded-lg transition-colors flex items-center justify-center gap-2"
          >
            <Plus size={16} />
            {$t('dashboard.newPatient')}
          </button>
          <button
            onclick={() => goto('/patients')}
            class="flex-1 px-4 py-2 text-sm font-medium text-green-600 dark:text-green-400 hover:bg-green-50 dark:hover:bg-green-900/20 rounded-lg transition-colors"
          >
            {$t('dashboard.viewAllPatients')}
          </button>
        </div>
      </div>

      <!-- Sessions with Incomplete Notes -->
      <div class="bg-white dark:bg-gray-800 border border-gray-200 dark:border-gray-700 rounded-lg p-6">
        <div class="flex items-center gap-3 mb-4">
          <div class="p-2 bg-amber-100 dark:bg-amber-900 rounded-lg">
            <FileText size={20} class="text-amber-600 dark:text-amber-300" />
          </div>
          <h2 class="text-lg font-semibold text-gray-900 dark:text-gray-100">{$t('dashboard.incompleteNotes')}</h2>
        </div>

        {#if data.sessions_with_incomplete_notes.length === 0}
          <p class="text-sm text-gray-500 dark:text-gray-400">{$t('dashboard.noIncompleteNotes')}</p>
        {:else}
          <div class="space-y-3 max-h-96 overflow-y-auto">
            {#each data.sessions_with_incomplete_notes as item}
              <button
                onclick={() => goto(`/patients/${item.session.patient_id}/sessions/${item.session.id}`)}
                class="w-full text-left bg-gray-50 dark:bg-gray-700 hover:bg-gray-100 dark:hover:bg-gray-600 rounded-lg p-3 transition-colors"
              >
                <p class="text-sm font-medium text-gray-900 dark:text-gray-100 truncate">{item.patient_name}</p>
                <div class="flex items-center gap-2 mt-1">
                  <span class="text-xs text-gray-500 dark:text-gray-400">
                    {formatDate(item.session.session_date)}
                  </span>
                  <span class="text-xs px-2 py-0.5 rounded-full bg-amber-100 dark:bg-amber-900 text-amber-700 dark:text-amber-200">
                    {getSessionTypeLabel(item.session.session_type)}
                  </span>
                </div>
              </button>
            {/each}
          </div>
        {/if}
      </div>
    </div>
  {/if}
</div>
