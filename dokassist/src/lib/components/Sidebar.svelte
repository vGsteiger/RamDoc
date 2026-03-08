<script lang="ts">
  import { page } from '$app/stores';
  import { lockApp } from '$lib/api';
  import { authStatus } from '$lib/stores/auth';
  import { goto } from '$app/navigation';
  import { Users, Calendar, BookOpen, MessageSquare, Settings, Lock } from 'lucide-svelte';

  const navItems = [
    { path: '/patients', label: 'Patients', icon: Users },
    { path: '/calendar', label: 'Calendar', icon: Calendar },
    { path: '/literature', label: 'Literature', icon: BookOpen },
    { path: '/chat', label: 'Chat', icon: MessageSquare },
    { path: '/settings', label: 'Settings', icon: Settings }
  ];

  async function handleLock() {
    try {
      await lockApp();
      authStatus.set('locked');
      goto('/unlock');
    } catch (error) {
      console.error('Failed to lock app:', error);
    }
  }

  let currentPath = $derived($page.url.pathname);
</script>

<aside class="w-64 bg-gray-900 border-r border-gray-800 flex flex-col h-screen">
  <div class="p-6 border-b border-gray-800">
    <h1 class="text-xl font-bold text-gray-100">RamDoc</h1>
  </div>

  <nav class="flex-1 p-4">
    <ul class="space-y-2">
      {#each navItems as item}
        {@const Icon = item.icon}
        <li>
          <a
            href={item.path}
            class="flex items-center gap-3 px-4 py-3 rounded-lg transition-colors {currentPath === item.path
              ? 'bg-blue-600 text-white'
              : 'text-gray-300 hover:bg-gray-800'}"
          >
            <Icon size={20} />
            <span class="font-medium">{item.label}</span>
          </a>
        </li>
      {/each}
    </ul>
  </nav>

  <div class="p-4 border-t border-gray-800">
    <button
      onclick={handleLock}
      class="w-full flex items-center gap-3 px-4 py-3 rounded-lg text-gray-300 hover:bg-gray-800 transition-colors"
    >
      <Lock size={20} />
      <span class="font-medium">Lock</span>
    </button>
  </div>
</aside>
