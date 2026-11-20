<script lang="ts">
  import { tick } from 'svelte';
  import { goto } from '$app/navigation';
  import { t } from '$lib/translations';
  import { fmt } from '$lib/format';
  import { renameChatSession, deleteChatSession, type ChatSession } from '$lib/api';
  import { chatSessionHref } from '$lib/shell';
  import { Pencil, Trash2 } from 'lucide-svelte';
  import { Button, ConfirmDialog, Input, Menu, type MenuItem } from '$lib/components/ui';
  import { addToast } from '$lib/stores/toast';
  import { errorText } from '$lib/translations/labels';

  /*
   * Recent chats, nested under "Chat" in the app sidebar (D2). Each row is a
   * link to the chat (title, attached patient, relative date) with a visible
   * ⋯ menu for rename and delete. Delete goes through ConfirmDialog.
   *
   * `onsessionselect`, `onsessionnew` and `onlistchange` are kept so the chat
   * page's existing usage still type-checks while WS2 moves the list out of
   * the page; the sidebar only needs `sessions`, `activeSessionId` and
   * `patientNames`.
   */
  interface Props {
    sessions: ChatSession[];
    activeSessionId: string | null;
    /** patient id → display name, for patient-scoped chats. */
    patientNames?: Record<string, string>;
    /** Rows shown before "Show all". */
    limit?: number;
    onsessionselect?: (sessionId: string) => void;
    /** @deprecated The sidebar owns "New chat" now; ignored. */
    onsessionnew?: () => void;
    onlistchange?: () => void;
    /** After the user deleted a chat (the list is already updated). */
    onsessiondeleted?: (sessionId: string) => void;
    onnavigate?: () => void;
  }

  let {
    sessions = $bindable(),
    activeSessionId,
    patientNames = {},
    limit = 8,
    onsessionselect,
    onlistchange,
    onsessiondeleted,
    onnavigate,
  }: Props = $props();

  let expanded = $state(false);
  let renamingId = $state<string | null>(null);
  let renameValue = $state('');
  let renameError = $state(false);
  let pendingDelete = $state<ChatSession | null>(null);
  let confirmOpen = $state(false);

  // The active chat stays visible even when it is older than the first `limit`.
  let visible = $derived.by(() => {
    if (expanded || sessions.length <= limit) return sessions;
    const head = sessions.slice(0, limit);
    const active = sessions.find((s) => s.id === activeSessionId);
    return active && !head.includes(active) ? [...head, active] : head;
  });

  function untitled(session: ChatSession) {
    return session.title.trim() || $t('nav.untitledChat');
  }

  function secondary(session: ChatSession) {
    const date = $fmt.relativeDate(session.updated_at);
    const patient = session.patient_id ? patientNames[session.patient_id] : undefined;
    return patient ? `${patient} · ${date}` : date;
  }

  async function startRename(session: ChatSession) {
    renamingId = session.id;
    renameValue = session.title;
    renameError = false;
    await tick();
    const input = document.getElementById(`chat-rename-${session.id}`) as HTMLInputElement | null;
    input?.focus();
    input?.select();
  }

  async function confirmRename(session: ChatSession) {
    const title = renameValue.trim();
    if (!title) {
      renameError = true;
      return;
    }
    if (title === session.title) {
      renamingId = null;
      return;
    }
    try {
      const updated = await renameChatSession(session.id, title);
      sessions = sessions.map((s) => (s.id === session.id ? updated : s));
      renamingId = null;
      onlistchange?.();
    } catch (error) {
      addToast($errorText(error), 'error');
    }
  }

  function askDelete(session: ChatSession) {
    pendingDelete = session;
    confirmOpen = true;
  }

  async function confirmDelete() {
    const session = pendingDelete;
    if (!session) return;
    try {
      await deleteChatSession(session.id);
    } catch (error) {
      addToast($errorText(error), 'error');
      throw error;
    }
    sessions = sessions.filter((s) => s.id !== session.id);
    pendingDelete = null;
    onlistchange?.();
    onsessiondeleted?.(session.id);
  }

  function menuItems(session: ChatSession): MenuItem[] {
    return [
      { label: $t('common.rename'), icon: Pencil, onSelect: () => void startRename(session) },
      {
        label: $t('common.delete'),
        icon: Trash2,
        danger: true,
        onSelect: () => askDelete(session),
      },
    ];
  }

  function select(session: ChatSession, event: MouseEvent) {
    onnavigate?.();
    if (onsessionselect) {
      onsessionselect(session.id);
    }
    if (event.metaKey || event.ctrlKey) return;
    event.preventDefault();
    void goto(chatSessionHref(session));
  }
</script>

{#if sessions.length === 0}
  <p class="py-1 pl-8 pr-2 text-caption text-fg-subtle">{$t('chat.noChats')}</p>
{:else}
  <ul class="space-y-px" aria-label={$t('nav.recentChats')}>
    {#each visible as session (session.id)}
      {@const active = session.id === activeSessionId}
      <li class="relative">
        {#if renamingId === session.id}
          <div class="py-0.5 pl-7 pr-1">
            <Input
              id="chat-rename-{session.id}"
              controlSize="sm"
              bind:value={renameValue}
              invalid={renameError}
              aria-label={$t('nav.renameChat')}
              onkeydown={(e: KeyboardEvent) => {
                if (e.key === 'Enter') {
                  e.preventDefault();
                  void confirmRename(session);
                }
                if (e.key === 'Escape') {
                  e.preventDefault();
                  e.stopPropagation();
                  renamingId = null;
                }
              }}
              onblur={() => {
                if (renamingId === session.id) void confirmRename(session);
              }}
            />
          </div>
        {:else}
          <a
            href={chatSessionHref(session)}
            aria-current={active ? 'page' : undefined}
            onclick={(e) => select(session, e)}
            class="relative flex min-h-9 flex-col justify-center rounded-control py-1 pl-8 pr-8 transition-colors duration-150 ease-standard {active
              ? 'bg-fill-strong'
              : 'hover:bg-fill'}"
          >
            {#if active}
              <span
                class="absolute left-0 top-1/2 h-4 w-0.5 -translate-y-1/2 rounded-full bg-brand"
                aria-hidden="true"
              ></span>
            {/if}
            <span class="truncate text-label {active ? 'font-medium text-fg' : 'text-fg'}"
              >{untitled(session)}</span
            >
            <span class="truncate text-caption text-fg-subtle">{secondary(session)}</span>
          </a>
          <div class="absolute inset-y-0 right-0.5 flex items-center">
            <Menu
              size="sm"
              label={$t('nav.chatActions').replace('{title}', untitled(session))}
              items={menuItems(session)}
            />
          </div>
        {/if}
      </li>
    {/each}
  </ul>
  {#if sessions.length > limit}
    <div class="pl-6">
      <Button variant="ghost" size="sm" onclick={() => (expanded = !expanded)}>
        {expanded
          ? $t('nav.showFewerChats')
          : $t('nav.showAllChats').replace('{count}', String(sessions.length))}
      </Button>
    </div>
  {/if}
{/if}

<ConfirmDialog
  bind:open={confirmOpen}
  tone="danger"
  title={$t('nav.deleteChatTitle')}
  message={pendingDelete
    ? $t('nav.deleteChatMessage').replace('{title}', untitled(pendingDelete))
    : undefined}
  onConfirm={confirmDelete}
  onCancel={() => (pendingDelete = null)}
/>
