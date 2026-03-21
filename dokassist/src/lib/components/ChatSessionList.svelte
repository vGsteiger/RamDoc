<script lang="ts">
  import { renameChatSession, deleteChatSession, type ChatSession } from '$lib/api';
  import { Check, X, Pencil, Trash2 } from 'lucide-svelte';

  interface Props {
    sessions: ChatSession[];
    activeSessionId: string | null;
    onsessionselect: (sessionId: string) => void;
    onsessionnew: () => void;
    onlistchange?: () => void;
  }

  let {
    sessions = $bindable(),
    activeSessionId,
    onsessionselect,
    onsessionnew,
    onlistchange,
  }: Props = $props();

  let renamingId = $state<string | null>(null);
  let renameValue = $state('');
  let renameInputEl = $state<HTMLInputElement | null>(null);

  $effect(() => {
    if (renamingId && renameInputEl) {
      renameInputEl.focus();
    }
  });

  function startRename(session: ChatSession) {
    renamingId = session.id;
    renameValue = session.title;
  }

  async function confirmRename(sessionId: string) {
    if (!renameValue.trim()) return;
    try {
      const updated = await renameChatSession(sessionId, renameValue.trim());
      sessions = sessions.map((s) => (s.id === sessionId ? updated : s));
      renamingId = null;
      onlistchange?.();
    } catch (e) {
      console.error('Rename failed:', e);
    }
  }

  async function handleDelete(sessionId: string) {
    try {
      await deleteChatSession(sessionId);
      sessions = sessions.filter((s) => s.id !== sessionId);
      onlistchange?.();
      if (activeSessionId === sessionId && sessions.length > 0) {
        onsessionselect(sessions[0].id);
      }
    } catch (e) {
      console.error('Delete failed:', e);
    }
  }

  function formatDate(iso: string): string {
    return new Date(iso).toLocaleDateString('de-CH', {
      day: '2-digit',
      month: '2-digit',
      year: 'numeric',
    });
  }
</script>

<div class="flex flex-col h-full">
  <div class="p-3 border-b border-gray-200 dark:border-gray-700">
    <button
      onclick={onsessionnew}
      class="w-full flex items-center justify-center gap-2 px-3 py-2 bg-blue-600 hover:bg-blue-700
             text-white text-sm font-medium rounded-lg transition-colors"
    >
      <span>+</span>
      <span>Neuer Chat</span>
    </button>
  </div>

  <div class="flex-1 overflow-y-auto">
    {#if sessions.length === 0}
      <p class="text-center text-gray-400 dark:text-gray-500 text-sm p-4">Keine Chats vorhanden</p>
    {:else}
      <ul class="py-2">
        {#each sessions as session (session.id)}
          <li
            class="group px-3 py-2 hover:bg-gray-100 dark:hover:bg-gray-800 transition-colors cursor-pointer
                   {activeSessionId === session.id ? 'bg-gray-100 dark:bg-gray-800' : ''}"
          >
            {#if renamingId === session.id}
              <div class="flex gap-1">
                <input
                  bind:this={renameInputEl}
                  bind:value={renameValue}
                  onkeydown={(e) => {
                    if (e.key === 'Enter') confirmRename(session.id);
                    if (e.key === 'Escape') renamingId = null;
                  }}
                  class="flex-1 text-sm bg-gray-100 dark:bg-gray-700 border border-gray-300 dark:border-gray-600 rounded px-2 py-0.5
                         text-gray-900 dark:text-gray-100 focus:outline-none focus:border-blue-500"
                />
                <button
                  onclick={() => confirmRename(session.id)}
                  class="text-xs text-green-400 hover:text-green-300 px-1"
                  aria-label="Bestätigen"><Check size={14} /></button
                >
                <button
                  onclick={() => (renamingId = null)}
                  class="text-xs text-gray-500 hover:text-gray-400 px-1"
                  aria-label="Abbrechen"><X size={14} /></button
                >
              </div>
            {:else}
              <div
                class="flex items-start gap-2"
                role="button"
                tabindex="0"
                onclick={() => onsessionselect(session.id)}
                onkeydown={(e) => e.key === 'Enter' && onsessionselect(session.id)}
              >
                <div class="flex-1 min-w-0">
                  <p class="text-sm text-gray-800 dark:text-gray-200 truncate">{session.title}</p>
                  <p class="text-xs text-gray-400 dark:text-gray-500">
                    {formatDate(session.updated_at)}
                  </p>
                </div>
                <div class="hidden group-hover:flex items-center gap-1 shrink-0">
                  <button
                    onclick={(e) => {
                      e.stopPropagation();
                      startRename(session);
                    }}
                    class="text-xs text-gray-500 hover:text-gray-300 p-0.5"
                    title="Umbenennen"><Pencil size={14} /></button
                  >
                  <button
                    onclick={(e) => {
                      e.stopPropagation();
                      handleDelete(session.id);
                    }}
                    class="text-xs text-gray-500 hover:text-red-400 p-0.5"
                    title="Löschen"><Trash2 size={14} /></button
                  >
                </div>
              </div>
            {/if}
          </li>
        {/each}
      </ul>
    {/if}
  </div>
</div>
