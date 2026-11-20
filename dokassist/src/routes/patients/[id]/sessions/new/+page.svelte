<script lang="ts">
  import { sessionTypeLabel } from '$lib/translations/labels';
  import { page } from '$app/state';
  import { goto } from '$app/navigation';
  import { createSession, type CreateSession } from '$lib/api';
  import { addToast } from '$lib/stores/toast';
  import { AMDP_CATEGORIES, serializeAMDP, type AMDPCategory } from '$lib/amdp';
  import AMDPForm from '$lib/components/AMDPForm.svelte';
  import DictationButton from '$lib/components/DictationButton.svelte';
  import { insertDictation } from '$lib/dictation';
  import { get } from 'svelte/store';
  import { t } from '$lib/translations';
  import {
    Alert,
    Button,
    Field,
    Input,
    PageShell,
    SectionHeader,
    Select,
  } from '$lib/components/ui';

  const patientId = $derived(page.params.id);

  const prefilledDate = page.url.searchParams.get('date');
  const prefilledTime = page.url.searchParams.get('time');

  let sessionType = $state('Erstgespräch');
  let sessionDate = $state(prefilledDate ?? new Date().toISOString().split('T')[0]);
  let sessionTime = $state(prefilledTime ?? '');
  let durationMinutes = $state(50);
  let notes = $state('');
  let notesEl = $state<HTMLTextAreaElement>();
  let amdpCategories = $state<AMDPCategory[]>(JSON.parse(JSON.stringify(AMDP_CATEGORIES)));

  let saving = $state(false);
  let error = $state<string | null>(null);
  let showAMDP = $state(false);

  const sessionTypes = [
    'Erstgespräch',
    'Verlaufskontrolle',
    'Krisenintervention',
    'Psychotherapie',
    'Medikamentenanpassung',
    'Andere',
  ];

  function handleAMDPScoreChange(code: string, score: 0 | 1 | 2 | 3) {
    // Find and update the score for the specific item
    amdpCategories = amdpCategories.map((category) => ({
      ...category,
      items: category.items.map((item) => (item.code === code ? { ...item, score } : item)),
    }));
  }

  async function handleSubmit(event: Event) {
    event.preventDefault();

    if (!sessionType.trim() || !notes.trim()) {
      error = get(t)('sessions.requiredFields');
      return;
    }

    try {
      saving = true;
      error = null;

      const input: CreateSession = {
        patient_id: patientId!,
        session_date: sessionDate,
        session_type: sessionType,
        duration_minutes: durationMinutes || undefined,
        scheduled_time: sessionTime ? `${sessionDate}T${sessionTime}:00` : undefined,
        notes,
        amdp_data: showAMDP ? serializeAMDP(amdpCategories) : undefined,
      };

      await createSession(input);
      addToast(get(t)('sessions.saved'));
      goto(`/patients/${patientId}/sessions`);
    } catch (err) {
      error =
        get(t)('common.saveFailed') + ': ' + (err instanceof Error ? err.message : String(err));
      console.error('Failed to create session:', err);
    } finally {
      saving = false;
    }
  }

  function handleCancel() {
    goto(`/patients/${patientId}/sessions`);
  }
</script>

<PageShell>
  <div>
    <SectionHeader title={$t('sessions.newSessionTitle')} />

    {#if error}
      <Alert tone="danger" class="mb-4">{error}</Alert>
    {/if}

    <form onsubmit={handleSubmit} class="space-y-4">
      <div class="grid grid-cols-1 gap-4 sm:grid-cols-2">
        <Field label={$t('sessions.sessionType')} for="session-type" required class="sm:col-span-2">
          <Select id="session-type" bind:value={sessionType} required>
            {#each sessionTypes as type (type)}
              <option value={type}>{$sessionTypeLabel(type)}</option>
            {/each}
          </Select>
        </Field>

        <Field label={$t('sessions.date')} for="session-date" required>
          <Input id="session-date" type="date" bind:value={sessionDate} required />
        </Field>

        <Field label={$t('sessions.timeOptional')} for="session-time">
          <Input id="session-time" type="time" bind:value={sessionTime} />
        </Field>

        <Field label={$t('sessions.duration')} for="duration">
          <Input
            id="duration"
            type="number"
            bind:value={durationMinutes}
            min="0"
            step="5"
            placeholder="50"
          />
        </Field>
      </div>

      <div>
        <div class="mb-1 flex items-center justify-between gap-2">
          <label for="notes" class="text-label font-medium text-fg">
            {$t('common.notes')} <span class="text-fg-muted" aria-hidden="true">*</span>
          </label>
          <DictationButton
            onTranscript={(text) => (notes = insertDictation(notesEl, notes, text))}
          />
        </div>
        <!-- Raw textarea on purpose: dictation inserts at the caret, so it needs
             the element (bind:this), which the Textarea primitive does not expose. -->
        <textarea
          id="notes"
          bind:this={notesEl}
          bind:value={notes}
          required
          rows="8"
          placeholder={$t('sessions.notesPlaceholderDetailed')}
          class="w-full resize-y rounded-control border border-line bg-surface-raised px-2.5 py-2 text-body text-fg transition-colors duration-150 ease-standard focus:border-focus focus:outline-none focus:ring-2 focus:ring-focus/25 focus-visible:outline-none"
        ></textarea>
      </div>

      <label class="flex cursor-pointer items-center gap-2">
        <input type="checkbox" bind:checked={showAMDP} class="size-4 accent-current" />
        <span class="text-body text-fg">{$t('sessions.amdpToggle')}</span>
      </label>

      {#if showAMDP}
        <section class="rounded-card border border-line p-4" aria-labelledby="amdp-title">
          <h2 id="amdp-title" class="mb-3 text-heading text-fg">{$t('sessions.amdpTitle')}</h2>
          <AMDPForm categories={amdpCategories} onScoreChange={handleAMDPScoreChange} />
        </section>
      {/if}

      <div class="flex justify-end gap-2 border-t border-line-subtle pt-4">
        <Button onclick={handleCancel} disabled={saving}>{$t('common.cancel')}</Button>
        <Button type="submit" variant="primary" loading={saving}>
          {$t('sessions.saveSession')}
        </Button>
      </div>
    </form>
  </div>
</PageShell>
