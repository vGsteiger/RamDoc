<script lang="ts">
  import type { Snippet } from 'svelte';
  import { page } from '$app/state';
  import { Alert, Skeleton, Tabs, type TabItem } from '$lib/components/ui';
  import PatientHeader from '$lib/components/patient/PatientHeader.svelte';
  import { PatientWorkspace, setPatientWorkspace } from '$lib/components/patient/state.svelte';
  import {
    PATIENT_TABS,
    activePatientTab,
    patientTabHref,
    routeOwnsPrimary,
  } from '$lib/components/patient/workspace';
  import { t } from '$lib/translations';

  /*
   * The patient workspace (D4, "patient as project"): one compact
   * identification header and the route tabs, pinned above the scrolling
   * tab content. Header + tabs share the PageShell column and left edge.
   */
  let { children }: { children: Snippet } = $props();

  let patientId = $derived(page.params.id!);
  // A fresh workspace per patient; the layout is reused when only the id changes.
  let current = $derived(new PatientWorkspace(patientId));
  setPatientWorkspace(() => current);

  $effect(() => {
    void current.load();
  });

  let pathname = $derived(page.url.pathname);
  let activeTab = $derived(activePatientTab(pathname, patientId));
  let tabs = $derived<TabItem[]>(
    PATIENT_TABS.map((tab) => ({
      id: tab.id,
      label: $t(tab.labelKey),
      href: patientTabHref(patientId, tab.id),
      count: tab.id === 'chats' && current.chatsLoaded ? current.chats.length : undefined,
    }))
  );
</script>

<div class="flex h-full min-h-0 flex-col">
  <div class="shrink-0 border-b border-line-subtle bg-surface">
    <div class="mx-auto w-full min-w-0 max-w-page px-page-x pt-3">
      {#if current.patient}
        <PatientHeader
          patient={current.patient}
          diagnoses={current.diagnoses}
          medications={current.medications}
          newChatPrimary={!routeOwnsPrimary(pathname, patientId)}
        />
      {:else if current.failed}
        <Alert tone="danger" class="mb-3">{$t('patients.loadError')}</Alert>
      {:else}
        <div class="flex h-[54px] flex-col justify-center gap-2" aria-busy="true">
          <span class="sr-only">{$t('patients.loadingPatient')}</span>
          <Skeleton class="h-5 w-56" />
          <Skeleton class="h-4 w-80 max-w-full" />
        </div>
      {/if}
      <Tabs
        items={tabs}
        value={activeTab ?? ''}
        label={$t('patients.workspaceNavigation')}
        class="mt-2 -mb-px"
      />
    </div>
  </div>

  <div class="min-h-0 flex-1 overflow-y-auto" data-patient-content>
    {#if current.patient}
      {#key current}
        {@render children()}
      {/key}
    {/if}
  </div>
</div>
