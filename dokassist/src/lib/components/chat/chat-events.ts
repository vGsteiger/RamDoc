import { writable } from 'svelte/store';

/**
 * Bumped whenever the chat workspace creates a chat session (a fresh chat is
 * only persisted when its first message is sent). Lists of chats, such as the
 * sidebar's recent chats, subscribe to it and refetch.
 */
export const chatSessionsVersion = writable(0);

export function notifyChatSessionsChanged() {
  chatSessionsVersion.update((n) => n + 1);
}
