<script lang="ts">
  import { onMount } from 'svelte';
  import { getSettings, updateSettings, type PracticeSettings } from '$lib/api';
  import { Alert, Button, ListSkeleton, PageShell, StatusIndicator } from '$lib/components/ui';
  import PracticeForm, {
    emptyPracticeSettings,
    normalizePracticeSettings,
  } from '$lib/components/PracticeForm.svelte';
  import { errorText } from '$lib/translations/labels';
  import { t } from '$lib/translations';

  /*
   * Practice details were only editable during onboarding; this pane makes
   * them editable at any time through the same get/update_settings API.
   */
  let settings = $state<PracticeSettings>(emptyPracticeSettings());
  let savedSnapshot = $state('');
  let loading = $state(true);
  let saving = $state(false);
  let error = $state('');
  let justSaved = $state(false);
  // Without the stored record, saving would overwrite it with blanks.
  let loadFailed = $state(false);

  const snapshot = (value: PracticeSettings) => JSON.stringify(normalizePracticeSettings(value));
  let dirty = $derived(!loading && snapshot(settings) !== savedSnapshot);

  onMount(async () => {
    try {
      settings = await getSettings();
      savedSnapshot = snapshot(settings);
    } catch (e) {
      loadFailed = true;
      error = $errorText(e, $t('settings.practice.loadFailed'));
    } finally {
      loading = false;
    }
  });

  async function save(event: SubmitEvent) {
    event.preventDefault();
    if (!dirty || saving) return;
    saving = true;
    error = '';
    justSaved = false;
    try {
      const next = normalizePracticeSettings(settings);
      await updateSettings(next);
      settings = next;
      savedSnapshot = snapshot(next);
      justSaved = true;
    } catch (e) {
      error = $errorText(e);
    } finally {
      saving = false;
    }
  }
</script>

<PageShell
  width="full"
  title={$t('settings.nav.practice')}
  description={$t('settings.panes.practiceDesc')}
>
  {#if loading}
    <ListSkeleton count={6} />
  {:else if loadFailed}
    <Alert tone="danger" title={$t('settings.practice.loadFailed')}>{error}</Alert>
  {:else}
    <form onsubmit={save} novalidate>
      <PracticeForm bind:settings disabled={saving} />

      {#if error}
        <Alert tone="danger" class="mb-3">{error}</Alert>
      {/if}

      <div class="flex flex-wrap items-center justify-end gap-3">
        {#if dirty}
          <span class="text-label text-fg-muted">{$t('settings.practice.unsaved')}</span>
        {:else if justSaved}
          <StatusIndicator tone="ok" label={$t('settings.practice.saved')} live />
        {/if}
        <Button type="submit" variant="primary" disabled={!dirty} loading={saving}>
          {$t('settings.practice.save')}
        </Button>
      </div>
    </form>
  {/if}
</PageShell>
