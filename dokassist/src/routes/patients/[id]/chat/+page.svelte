<script lang="ts">
  /*
   * "The patient's chat": opens this patient's most recent chat, or a fresh
   * chat with the patient attached when there is none yet (D4).
   *   → /chat?sessionId=<latest>   or   /chat?patientId=<id>
   */
  import { onMount } from 'svelte';
  import { page } from '$app/state';
  import { goto } from '$app/navigation';
  import { resolve } from '$app/paths';
  import { listChatSessions } from '$lib/api';
  import { t } from '$lib/translations';

  onMount(async () => {
    const patientId = page.params.id ?? '';
    let target = `${resolve('/chat')}?patientId=${encodeURIComponent(patientId)}`;
    try {
      const [latest] = await listChatSessions('patient', patientId);
      if (latest) target = `${resolve('/chat')}?sessionId=${encodeURIComponent(latest.id)}`;
    } catch (error) {
      console.error('Failed to list the patient chats:', error);
    }
    void goto(target, { replace: true });
  });
</script>

<p class="px-page-x py-page-y text-body text-fg-muted">{$t('chat.openingChat')}</p>
