<script lang="ts">
  import { onMount } from 'svelte';
  import { MessageSquare, MessageSquarePlus } from 'lucide-svelte';
  import {
    Button,
    EmptyState,
    ListRow,
    ListSkeleton,
    PageShell,
    SectionHeader,
  } from '$lib/components/ui';
  import { getPatientWorkspace } from '$lib/components/patient/state.svelte';
  import { chatHref, newPatientChatHref } from '$lib/components/patient/workspace';
  import { fmt } from '$lib/format';
  import { t } from '$lib/translations';

  /*
   * The patient's chats, the first section of the workspace (D4). Each row
   * opens the chat in the chat workspace. "New chat" lives in the patient
   * header (and in the empty state) and goes to `/chat?patientId={id}`,
   * which starts a chat with this patient attached.
   */
  const workspace = getPatientWorkspace();
  let chats = $derived(workspace?.chats ?? []);
  let loaded = $derived(workspace?.chatsLoaded ?? false);
  let newChatHref = $derived(workspace ? newPatientChatHref(workspace.id) : '/chat?new=1');

  // The list may be stale if a chat was created or renamed since the layout loaded.
  onMount(() => {
    if (workspace?.chatsLoaded) void workspace.refreshChats();
  });
</script>

<PageShell>
  <SectionHeader title={$t('patients.tabChats')} />

  {#if !loaded}
    <ListSkeleton count={3} />
  {:else if chats.length === 0}
    <EmptyState
      icon={MessageSquare}
      title={$t('patients.noChats')}
      description={$t('patients.noChatsHint')}
    >
      {#snippet action()}
        <Button href={newChatHref}>
          <MessageSquarePlus size={14} aria-hidden="true" />
          {$t('patients.newChat')}
        </Button>
      {/snippet}
    </EmptyState>
  {:else}
    <ul class="-mx-3" aria-label={$t('patients.tabChats')}>
      {#each chats as chat (chat.id)}
        <li>
          <ListRow
            title={chat.title || $t('patients.untitledChat')}
            meta={$fmt.relativeDate(chat.updated_at)}
            href={chatHref(chat.id)}
          >
            {#snippet leading()}
              <MessageSquare size={16} aria-hidden="true" />
            {/snippet}
          </ListRow>
        </li>
      {/each}
    </ul>
  {/if}
</PageShell>
