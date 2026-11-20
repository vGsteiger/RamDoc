<script lang="ts">
  import { t } from '$lib/translations';
  import { CheckCircle2, XCircle, X } from 'lucide-svelte';
  import { toasts, removeToast } from '$lib/stores/toast';
  import { IconButton } from '$lib/components/ui';
</script>

<!-- Top-right, clear of the sidebar and of the chat composer. An overlay surface with a hairline and one status icon: the icon shape
     and the text carry the state, colour only reinforces it. -->
<div
  class="pointer-events-none fixed right-4 top-4 z-50 flex w-[min(24rem,calc(100vw-2rem))] flex-col gap-2"
  aria-live="polite"
  aria-atomic="false"
>
  {#each $toasts as toast (toast.id)}
    <div
      role={toast.type === 'error' ? 'alert' : 'status'}
      data-toast={toast.type}
      class="pointer-events-auto flex items-start gap-2.5 rounded-card border border-line bg-surface-overlay py-2 pl-3 pr-1.5 text-body text-fg shadow-popover"
    >
      {#if toast.type === 'success'}
        <CheckCircle2 size={16} class="mt-1.5 shrink-0 text-success-fg" aria-hidden="true" />
      {:else}
        <XCircle size={16} class="mt-1.5 shrink-0 text-danger-fg" aria-hidden="true" />
      {/if}
      <span class="min-w-0 flex-1 py-1 [overflow-wrap:anywhere]">{toast.message}</span>
      <IconButton size="sm" label={$t('common.dismiss')} onclick={() => removeToast(toast.id)}>
        <X size={14} aria-hidden="true" />
      </IconButton>
    </div>
  {/each}
</div>
