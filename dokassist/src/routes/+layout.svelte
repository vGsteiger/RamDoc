<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { page } from '$app/stores';
  import { checkAuth } from '$lib/api';
  import { authStatus } from '$lib/stores/auth';
  import Sidebar from '$lib/components/Sidebar.svelte';
  import TopBar from '$lib/components/TopBar.svelte';
  import '../app.css';

  let currentPath = $derived($page.url.pathname);

  const authPaths = ['/', '/setup', '/unlock', '/recover'];
  let showLayout = $derived(!authPaths.includes(currentPath));

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
  <div class="flex h-screen bg-gray-950">
    <Sidebar />
    <div class="flex-1 flex flex-col overflow-hidden">
      <TopBar />
      <main class="flex-1 overflow-auto">
        <slot />
      </main>
    </div>
  </div>
{:else}
  <slot />
{/if}
