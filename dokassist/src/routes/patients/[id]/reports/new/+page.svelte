<script lang="ts">
  import { onMount } from 'svelte';
  import { page } from '$app/stores';
  import { goto } from '$app/navigation';
  import { resolve } from '$app/paths';
  import { t } from '$lib/translations';

  // Report drafting is a patient-attached chat workflow. The starter prompt is
  // editable in ChatThread and is deliberately not submitted automatically.
  onMount(() => {
    void goto(
      `${resolve('/chat')}?patientId=${encodeURIComponent($page.params.id ?? '')}&intent=report`
    );
  });
</script>

<div class="p-8 text-fg-muted">{$t('chat.openingReportChat')}</div>
