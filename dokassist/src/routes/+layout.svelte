<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { page } from '$app/state';
  import { checkAuth } from '$lib/api';
  import { authStatus } from '$lib/stores/auth';
  import { resolvedTheme } from '$lib/stores/theme';
  import { sidebarCollapsed } from '$lib/stores/sidebar';
  import { matchShellShortcut, NEW_CHAT_URL, patientIdFromPath } from '$lib/shell';
  import { t } from '$lib/translations';
  import Sidebar from '$lib/components/Sidebar.svelte';
  import Toast from '$lib/components/Toast.svelte';
  import CommandPalette from '$lib/components/CommandPalette.svelte';
  import type { Snippet } from 'svelte';
  import '../app.css';

  let { children }: { children: Snippet } = $props();

  let currentPath = $derived(page.url.pathname);
  let isCommandPaletteOpen = $state(false);
  let isOverlayOpen = $state(false);
  // ≥1024px: full sidebar (or the rail, if the user collapsed it). Below: rail.
  let isWide = $state(
    typeof window === 'undefined' || typeof window.matchMedia !== 'function'
      ? true
      : window.matchMedia('(min-width: 1024px)').matches
  );

  const authPaths = ['/', '/setup', '/unlock', '/recover', '/reset'];
  const layoutlessPaths = [
    ...authPaths,
    '/onboarding/step1',
    '/onboarding/step2',
    '/onboarding/step3',
    '/onboarding/step4',
  ];
  let showLayout = $derived(!layoutlessPaths.includes(currentPath));

  // Toggling `dark` on <html> is the whole theme switch: every design token
  // flips with it, and body colours come from app.css. Nothing else to set.
  $effect(() => {
    if (typeof document !== 'undefined') {
      document.documentElement.classList.toggle('dark', $resolvedTheme === 'dark');
    }
  });

  // Any navigation closes the overlaid sidebar.
  $effect(() => {
    void page.url.href;
    isOverlayOpen = false;
  });

  function toggleSidebar() {
    if (isWide) sidebarCollapsed.update((value) => !value);
    else isOverlayOpen = !isOverlayOpen;
  }

  onMount(() => {
    // Async initialization
    (async () => {
      // Only enforce auth on protected routes
      if (!layoutlessPaths.includes(currentPath)) {
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
    })();

    let media: MediaQueryList | null = null;
    const onMediaChange = (event: MediaQueryListEvent) => {
      isWide = event.matches;
      isOverlayOpen = false;
    };
    if (typeof window.matchMedia === 'function') {
      media = window.matchMedia('(min-width: 1024px)');
      isWide = media.matches;
      media.addEventListener('change', onMediaChange);
    }

    // Global keyboard shortcuts (D10). See matchShellShortcut for the map.
    const handleGlobalKeydown = (e: KeyboardEvent) => {
      // Don't handle shortcuts on auth routes
      if (layoutlessPaths.includes(currentPath)) {
        return;
      }

      const shortcut = matchShellShortcut(e);
      if (shortcut === 'commandPalette') {
        e.preventDefault();
        isCommandPaletteOpen = true;
        return;
      }
      if (shortcut === 'toggleSidebar') {
        e.preventDefault();
        toggleSidebar();
        return;
      }
      if (isCommandPaletteOpen) return;
      if (shortcut === 'newChat') {
        e.preventDefault();
        goto(NEW_CHAT_URL);
      } else if (shortcut === 'newPatient') {
        e.preventDefault();
        goto('/patients/new');
      } else if (shortcut === 'newSession') {
        e.preventDefault();
        const patientId = patientIdFromPath(currentPath);
        if (patientId) goto(`/patients/${patientId}/sessions/new`);
      }
    };

    window.addEventListener('keydown', handleGlobalKeydown);

    return () => {
      window.removeEventListener('keydown', handleGlobalKeydown);
      media?.removeEventListener('change', onMediaChange);
    };
  });
</script>

{#if showLayout}
  <a
    href="#main-content"
    class="sr-only z-50 rounded-control bg-surface-overlay px-3 py-2 text-body text-fg shadow-popover focus:not-sr-only focus:fixed focus:left-3 focus:top-3"
  >
    {$t('nav.skipToContent')}
  </a>
  <div class="flex h-screen bg-surface">
    <Sidebar
      collapsed={$sidebarCollapsed}
      wide={isWide}
      overlayOpen={isOverlayOpen}
      onToggle={toggleSidebar}
      onOverlayClose={() => (isOverlayOpen = false)}
      onOpenPalette={() => (isCommandPaletteOpen = true)}
    />
    <main id="main-content" tabindex="-1" class="min-w-0 flex-1 overflow-auto">
      {@render children()}
    </main>
  </div>
  <Toast />
  <CommandPalette
    bind:isOpen={isCommandPaletteOpen}
    onClose={() => (isCommandPaletteOpen = false)}
  />
{:else}
  {@render children()}
{/if}
