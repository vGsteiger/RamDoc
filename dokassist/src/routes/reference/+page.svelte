<script lang="ts">
  import { page } from '$app/state';
  import LiteratureLibrary from '$lib/components/reference/LiteratureLibrary.svelte';
  import MedicationReference from '$lib/components/reference/MedicationReference.svelte';
  import { PageShell, Tabs, type TabItem } from '$lib/components/ui';
  import { referenceHref, referenceTab } from '$lib/reference';
  import { t } from '$lib/translations';

  /*
   * Reference (D9): Medications first, then the Literature library.
   * The tab lives in the URL (`?tab=literature`) so `/literature` can
   * redirect straight to it and the sidebar can link to either.
   */
  let tab = $derived(referenceTab(page.url.searchParams));
  let query = $derived(page.url.searchParams.get('q') ?? '');

  let tabs = $derived<TabItem[]>([
    {
      id: 'medications',
      label: $t('reference.medicationsTab'),
      href: referenceHref('medications'),
    },
    { id: 'literature', label: $t('reference.literatureTab'), href: referenceHref('literature') },
  ]);
</script>

<PageShell title={$t('reference.title')} description={$t('reference.description')}>
  <Tabs items={tabs} value={tab} label={$t('reference.tabsLabel')} class="mb-section" />

  <!-- Keyed so each tab mounts fresh with its own query, and a tab switch
       never leaves the other tab's search running. -->
  {#key tab}
    {#if tab === 'literature'}
      <LiteratureLibrary initialQuery={query} />
    {:else}
      <MedicationReference initialQuery={query} />
    {/if}
  {/key}
</PageShell>
