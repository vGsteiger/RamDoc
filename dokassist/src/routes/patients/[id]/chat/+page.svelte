<script lang="ts">
  import { onMount } from 'svelte';
  import { page } from '$app/stores';
  import { goto } from '$app/navigation';
  import { resolve } from '$app/paths';
  import { t } from '$lib/translations';

  onMount(() => {
    void goto(`${resolve('/chat')}?patientId=${encodeURIComponent($page.params.id ?? '')}`);
  });
</script>

<div class="p-8 text-fg-muted">{$t('chat.openingChat')}</div>
