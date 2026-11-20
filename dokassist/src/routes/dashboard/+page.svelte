<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';

  // D3: the dashboard folded into the chat home ("Today" on a new chat).
  // The route stays so old links and bookmarks still land somewhere useful.
  onMount(() => {
    void goto('/chat', { replaceState: true });
  });
</script>
