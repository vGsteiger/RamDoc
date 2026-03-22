<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { page } from '$app/stores';
  import { checkAuth } from '$lib/api';
  import { authStatus } from '$lib/stores/auth';
  import { resolvedTheme } from '$lib/stores/theme';
  import Sidebar from '$lib/components/Sidebar.svelte';
  import TopBar from '$lib/components/TopBar.svelte';
  import Toast from '$lib/components/Toast.svelte';
  import type { Snippet } from 'svelte';
  import '../app.css';

  let { children }: { children: Snippet } = $props();

  let currentPath = $derived($page.url.pathname);

  const authPaths = ['/', '/setup', '/unlock', '/recover'];
  let showLayout = $derived(!authPaths.includes(currentPath));

  // Apply theme to document element
  $effect(() => {
    if (typeof document !== 'undefined') {
      const html = document.documentElement;
      const body = document.body;

      if ($resolvedTheme === 'dark') {
        html.classList.add('dark');
        body.classList.add('bg-gray-950', 'text-gray-100');
        body.classList.remove('bg-white', 'text-gray-900');
      } else {
        html.classList.remove('dark');
        body.classList.add('bg-white', 'text-gray-900');
        body.classList.remove('bg-gray-950', 'text-gray-100');
      }
    }
  });

  onMount(async () => {
    // Only enforce auth on protected routes
    if (!authPaths.includes(currentPath)) {
      try {
        const status = await checkAuth();
        authStatus.set(status);

        // Redirect to appropriate auth page if not unlocked
        if (status !== 'unlocked') {
          if (status === 'first_run') {
            goto('/setup', { replaceState: true });
          } else if (status === 'locked') {
            goto('/unlock', { replaceState: true });
          } else if (status === 'recovery_required') {
            goto('/recover', { replaceState: true });
          }
        }
      } catch (error) {
        console.error('Auth check failed:', error);
        goto('/unlock', { replaceState: true });
      }
    }
  });
</script>

{#if showLayout}
  <div class="flex h-screen bg-white dark:bg-gray-950">
    <Sidebar />
    <div class="flex-1 flex flex-col overflow-hidden">
      <TopBar />
      <main class="flex-1 overflow-auto">
        {@render children()}
      </main>
    </div>
  </div>
  <Toast />
{:else}
  {@render children()}
{/if}
