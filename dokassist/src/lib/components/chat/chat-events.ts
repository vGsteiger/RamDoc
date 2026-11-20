import { writable } from 'svelte/store';
import { refreshChatSessions } from '$lib/stores/chat-sessions';

/**
 * Bumped whenever the chat workspace creates a chat session (a fresh chat is
 * only persisted when its first message is sent). Also refreshes the shared
 * recent-chats store, so the sidebar and command palette list the new chat
 * immediately.
 */
export const chatSessionsVersion = writable(0);

export function notifyChatSessionsChanged() {
  chatSessionsVersion.update((n) => n + 1);
  void refreshChatSessions();
}
