<script lang="ts">
  import { toasts, removeToast } from '$lib/stores/toast';
</script>

<div class="fixed top-4 right-4 z-50 flex flex-col gap-2 pointer-events-none" aria-live="polite" aria-atomic="false">
  {#each $toasts as toast (toast.id)}
    <div
      role="status"
      class="flex items-center gap-3 px-4 py-3 rounded-lg shadow-lg pointer-events-auto text-sm font-medium min-w-64 max-w-sm transition-all {toast.type === 'success'
        ? 'bg-green-600 text-white'
        : 'bg-red-600 text-white'}"
    >
      <span class="flex-1">{toast.message}</span>
      <button
        onclick={() => removeToast(toast.id)}
        class="text-white/70 hover:text-white text-lg leading-none"
        aria-label="Dismiss notification"
      >×</button>
    </div>
  {/each}
</div>
