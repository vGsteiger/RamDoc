<script lang="ts">
  /*
   * D3: the dashboard folded into the chat home. An empty chat shows today's
   * appointments and the session notes still to finish, each with one-click
   * "Summarize" (prefills a chat prompt with that patient attached) and "Open"
   * (the session), plus a few neutral prompt suggestions.
   */
  import { onMount } from 'svelte';
  import { CornerDownRight } from 'lucide-svelte';
  import { resolve } from '$app/paths';
  import { getDashboardData, type DashboardData, type SessionWithPatient } from '$lib/api';
  import { t } from '$lib/translations';
  import { errorText, sessionTypeLabel } from '$lib/translations/labels';
  import { fmt } from '$lib/format';
  import { Alert, Button, ListRow, SectionHeader, Skeleton } from '$lib/components/ui';

  let {
    onsummarize,
    onprompt,
  }: {
    /** Start a chat with this patient attached and the prompt prefilled. */
    onsummarize: (patientId: string, prompt: string) => void;
    /** Prefill the composer with a suggested prompt. */
    onprompt: (prompt: string) => void;
  } = $props();

  let data = $state<DashboardData | null>(null);
  let loading = $state(true);
  let error = $state('');

  const PROMPT_KEYS = ['global1', 'global2', 'global3', 'global4'];

  let appointments = $derived(
    [...(data?.todays_sessions ?? [])].sort((a, b) =>
      (a.session.scheduled_time ?? '').localeCompare(b.session.scheduled_time ?? '')
    )
  );
  let todayLabel = $derived(
    `${$fmt.weekday(new Date())}, ${$fmt.date(new Date(), { style: 'long' })}`
  );
  let notesToFinish = $derived(data?.sessions_with_incomplete_notes ?? []);

  function sessionHref(item: SessionWithPatient) {
    return resolve('/patients/[id]/sessions/[sessionId]', {
      id: item.session.patient_id,
      sessionId: item.session.id,
    });
  }

  function appointmentMeta(item: SessionWithPatient) {
    return item.session.scheduled_time ? $fmt.time(item.session.scheduled_time) : '';
  }

  function summarizeAppointment(item: SessionWithPatient) {
    onsummarize(
      item.session.patient_id,
      $t('chat.summarizeAppointmentPrompt').replace('{name}', item.patient_name)
    );
  }

  function summarizeNote(item: SessionWithPatient) {
    onsummarize(
      item.session.patient_id,
      $t('chat.summarizeNotePrompt')
        .replace('{name}', item.patient_name)
        .replace('{date}', $fmt.date(item.session.session_date))
    );
  }

  onMount(async () => {
    try {
      data = await getDashboardData();
    } catch (err) {
      error = $errorText(err, $t('chat.todayLoadError'));
    } finally {
      loading = false;
    }
  });
</script>

{#snippet sessionRow(item: SessionWithPatient, meta: string, summarize: () => void)}
  <ListRow
    title={item.patient_name}
    secondary={$sessionTypeLabel(item.session.session_type)}
    {meta}
  >
    {#snippet trailing()}
      <Button
        size="sm"
        variant="ghost"
        onclick={summarize}
        aria-label={$t('chat.summarizeFor').replace('{name}', item.patient_name)}
        >{$t('chat.summarize')}</Button
      >
      <Button
        size="sm"
        variant="ghost"
        href={sessionHref(item)}
        aria-label={$t('chat.openSessionWith').replace('{name}', item.patient_name)}
        >{$t('chat.open')}</Button
      >
    {/snippet}
  </ListRow>
{/snippet}

<section class="space-y-6" aria-labelledby="today-panel-title" data-testid="today-panel">
  <header>
    <h2 id="today-panel-title" class="text-title text-fg">{$t('chat.today')}</h2>
    <p class="mt-0.5 text-label text-fg-muted">
      {todayLabel}
    </p>
  </header>

  {#if loading}
    <div class="space-y-2" aria-hidden="true">
      <Skeleton class="h-row w-full" />
      <Skeleton class="h-row w-full" />
      <Skeleton class="h-row w-2/3" />
    </div>
  {:else if error}
    <Alert tone="warning">{error}</Alert>
  {:else}
    <div class="space-y-5">
      <div class="min-w-0">
        <SectionHeader title={$t('chat.todayAppointments')} level={3} />
        {#if appointments.length === 0}
          <p class="px-3 text-label text-fg-muted">{$t('chat.todayNoAppointments')}</p>
        {:else}
          <div class="-mx-3">
            {#each appointments as item (item.session.id)}
              {@render sessionRow(item, appointmentMeta(item), () => summarizeAppointment(item))}
            {/each}
          </div>
        {/if}
      </div>
      <div class="min-w-0">
        <SectionHeader title={$t('chat.todayNotesToFinish')} level={3} />
        {#if notesToFinish.length === 0}
          <p class="px-3 text-label text-fg-muted">{$t('chat.todayNoNotes')}</p>
        {:else}
          <div class="-mx-3">
            {#each notesToFinish.slice(0, 5) as item (item.session.id)}
              {@render sessionRow(item, $fmt.relativeDate(item.session.session_date), () =>
                summarizeNote(item)
              )}
            {/each}
          </div>
        {/if}
      </div>
    </div>
  {/if}

  <div>
    <SectionHeader title={$t('chat.suggestions')} level={3} />
    <div class="-mx-3">
      {#each PROMPT_KEYS as key (key)}
        {@const prompt = $t(`chat.suggestedPrompts.${key}`)}
        <ListRow title={prompt} onclick={() => onprompt(prompt)}>
          {#snippet leading()}<CornerDownRight size={14} aria-hidden="true" />{/snippet}
        </ListRow>
      {/each}
    </div>
  </div>
</section>
