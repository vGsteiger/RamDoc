<script lang="ts">
  import { goto } from '$app/navigation';
  import { Spinner } from '$lib/components/ui';
  import { getPatientWorkspace } from '$lib/components/patient/state.svelte';
  import { landingTab, patientTabHref } from '$lib/components/patient/workspace';
  import { t } from '$lib/translations';

  /*
   * `/patients/{id}` is the workspace's landing URL (sidebar, search, chat
   * "open record" links all point here). It forwards to Chats when the
   * patient has chats and to the Overview otherwise; see `landingTab`.
   * replaceState keeps Back from bouncing through this redirect.
   */
  const workspace = getPatientWorkspace();

  $effect(() => {
    if (!workspace?.chatsLoaded) return;
    void goto(patientTabHref(workspace.id, landingTab(workspace.chats.length)), {
      replaceState: true,
    });
  });
</script>

<div class="flex justify-center py-12">
  <Spinner label={$t('common.loading')} />
</div>
