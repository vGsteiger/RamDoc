<script lang="ts">
  import { sessionTypeLabel, errorText } from '$lib/translations/labels';
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { getDashboardData, type DashboardData } from '$lib/api';
  import { t } from '$lib/translations';
  import { language } from '$lib/stores/language';
  import { Calendar, Users, FileText, Plus } from 'lucide-svelte';
  import { Alert, Badge, Button, Card, ListSkeleton, PageHeader } from '$lib/components/ui';

  let data = $state<DashboardData | null>(null);
  let isLoading = $state(true);
  let error = $state<string | null>(null);

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
      error = $errorText(err, $t('dashboard.loadError'));
    } finally {
      isLoading = false;
    }
  });
</script>

<div class="p-8 max-w-7xl mx-auto">
  <PageHeader title={$t('dashboard.title')} />

  {#if isLoading}
    <ListSkeleton variant="dashboard" count={3} />
  {:else if error}
    <Alert tone="danger">{error}</Alert>
  {:else if data}
    <div class="grid grid-cols-1 lg:grid-cols-3 gap-4">
      <!-- Today's Sessions -->
      <Card>
        <!-- Panel icons are neutral: three differently-tinted chips read as
             decoration, and colour here is reserved for status. -->
        <div class="mb-3 flex items-center gap-2">
          <Calendar size={16} class="text-fg-subtle" />
          <h2 class="text-heading text-fg">{$t('dashboard.todaysSessions')}</h2>
        </div>

        {#if data.todays_sessions.length === 0}
          <p class="text-body text-fg-muted">{$t('dashboard.noSessionsToday')}</p>
        {:else}
          <div class="space-y-1">
            {#each data.todays_sessions as item}
              <button
                onclick={() => goto(`/patients/${item.session.patient_id}/sessions`)}
                class="w-full rounded-control p-2 text-left transition-colors duration-150 ease-standard hover:bg-surface-hover"
              >
                <p class="truncate text-body font-medium text-fg">{item.patient_name}</p>
                <div class="mt-1 flex items-center gap-2">
                  <Badge>{$sessionTypeLabel(item.session.session_type)}</Badge>
                  {#if item.session.duration_minutes}
                    <span class="text-caption text-fg-subtle">
                      {item.session.duration_minutes}
                      {$t('dashboard.minutes')}
                    </span>
                  {/if}
                </div>
              </button>
            {/each}
          </div>
        {/if}

        <Button full class="mt-3" onclick={() => goto('/calendar')}>
          {$t('dashboard.viewCalendar')}
        </Button>
      </Card>

      <!-- Recent Patients -->
      <Card>
        <div class="mb-3 flex items-center gap-2">
          <Users size={16} class="text-fg-subtle" />
          <h2 class="text-heading text-fg">{$t('dashboard.recentPatients')}</h2>
        </div>

        {#if data.recent_patients.length === 0}
          <p class="text-body text-fg-muted">{$t('dashboard.noRecentPatients')}</p>
        {:else}
          <div class="space-y-1">
            {#each data.recent_patients as patient}
              <button
                onclick={() => goto(`/patients/${patient.id}`)}
                class="w-full rounded-control p-2 text-left transition-colors duration-150 ease-standard hover:bg-surface-hover"
              >
                <p class="text-body font-medium text-fg">
                  {patient.first_name}
                  {patient.last_name}
                </p>
                <p class="mt-0.5 text-caption text-fg-subtle">
                  {formatDate(patient.date_of_birth)}
                </p>
              </button>
            {/each}
          </div>
        {/if}

        <div class="mt-3 flex gap-2">
          <Button variant="primary" class="flex-1" onclick={() => goto('/patients/new')}>
            <Plus size={14} />
            {$t('dashboard.newPatient')}
          </Button>
          <Button class="flex-1" onclick={() => goto('/patients')}>
            {$t('dashboard.viewAllPatients')}
          </Button>
        </div>
      </Card>

      <!-- Sessions with Incomplete Notes -->
      <Card>
        <div class="mb-3 flex items-center gap-2">
          <FileText size={16} class="text-fg-subtle" />
          <h2 class="text-heading text-fg">{$t('dashboard.incompleteNotes')}</h2>
        </div>

        {#if data.sessions_with_incomplete_notes.length === 0}
          <p class="text-body text-fg-muted">{$t('dashboard.noIncompleteNotes')}</p>
        {:else}
          <div class="max-h-96 space-y-1 overflow-y-auto">
            {#each data.sessions_with_incomplete_notes as item}
              <button
                onclick={() =>
                  goto(`/patients/${item.session.patient_id}/sessions/${item.session.id}`)}
                class="w-full rounded-control p-2 text-left transition-colors duration-150 ease-standard hover:bg-surface-hover"
              >
                <p class="truncate text-body font-medium text-fg">{item.patient_name}</p>
                <div class="mt-1 flex items-center gap-2">
                  <span class="text-caption text-fg-subtle">
                    {formatDate(item.session.session_date)}
                  </span>
                  <Badge tone="warning">{$sessionTypeLabel(item.session.session_type)}</Badge>
                </div>
              </button>
            {/each}
          </div>
        {/if}
      </Card>
    </div>
  {/if}
</div>
