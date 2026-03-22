<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { page } from '$app/stores';
  import { checkAuth, listPatients, type Patient } from '$lib/api';
  import { authStatus } from '$lib/stores/auth';
  import { resolvedTheme } from '$lib/stores/theme';
  import Sidebar from '$lib/components/Sidebar.svelte';
  import TopBar from '$lib/components/TopBar.svelte';
  import Toast from '$lib/components/Toast.svelte';
  import CommandPalette from '$lib/components/CommandPalette.svelte';
  import type { Snippet } from 'svelte';
  import '../app.css';

  let { children }: { children: Snippet } = $props();

  let currentPath = $derived($page.url.pathname);
  let isCommandPaletteOpen = $state(false);
  let patients = $state<Patient[]>([]);

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
        } else {
          // Load patients for command palette
          try {
            patients = await listPatients();
          } catch (error) {
            console.error('Failed to load patients:', error);
          }
        }
      } catch (error) {
        console.error('Auth check failed:', error);
        goto('/unlock', { replaceState: true });
      }
    }

    // Global keyboard shortcuts
    const handleGlobalKeydown = (e: KeyboardEvent) => {
      // Cmd+K / Ctrl+K - Open command palette
      if ((e.metaKey || e.ctrlKey) && e.key === 'k') {
        e.preventDefault();
        isCommandPaletteOpen = true;
      }
      // Cmd+N / Ctrl+N - New patient
      if ((e.metaKey || e.ctrlKey) && e.key === 'n' && !isCommandPaletteOpen) {
        e.preventDefault();
        goto('/patients/new');
      }
      // Cmd+Shift+S / Ctrl+Shift+S - New session (requires current patient page)
      if ((e.metaKey || e.ctrlKey) && e.shiftKey && e.key === 's' && !isCommandPaletteOpen) {
        e.preventDefault();
        // Check if we're on a patient page
        const match = currentPath.match(/^\/patients\/([^\/]+)$/);
        if (match && match[1] !== 'new') {
          goto(`/patients/${match[1]}/sessions/new`);
        }
      }
      // Cmd+F / Ctrl+F - Focus search (handled by TopBar)
      // Escape - Close command palette
      if (e.key === 'Escape' && isCommandPaletteOpen) {
        isCommandPaletteOpen = false;
      }
    };

    window.addEventListener('keydown', handleGlobalKeydown);

    return () => {
      window.removeEventListener('keydown', handleGlobalKeydown);
    };
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
  <CommandPalette
    bind:isOpen={isCommandPaletteOpen}
    onClose={() => (isCommandPaletteOpen = false)}
    {patients}
  />
{:else}
  {@render children()}
{/if}
