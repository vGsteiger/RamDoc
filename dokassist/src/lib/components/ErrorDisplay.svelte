<script lang="ts">
  import type { AppError } from '$lib/api';
  import { t } from '$lib/translations';
  import { errorMessage } from '$lib/translations/labels';
  import { Alert, Button } from '$lib/components/ui';

  interface Props {
    error: AppError | null;
    showDetails?: boolean;
  }

  let { error, showDetails = false }: Props = $props();

  let expanded = $state(false);

  function copyErrorRef() {
    if (error?.ref) {
      navigator.clipboard.writeText(error.ref);
    }
  }

  function copyFullError() {
    if (error) {
      const text = `Error Code: ${error.code}\nError Reference: ${error.ref}\nMessage: ${error.message}`;
      navigator.clipboard.writeText(text);
    }
  }
</script>

{#if error}
  <Alert tone="danger" title={$errorMessage(error.code, error.message)}>
    <div class="flex flex-wrap items-center justify-between gap-x-4 gap-y-1">
      <div class="flex min-w-0 items-center gap-1 text-caption">
        <span class="truncate font-mono">{error.ref}</span>
        <Button variant="ghost" size="sm" onclick={copyErrorRef} title={$t('errors.copyReference')}>
          {$t('errors.copy')}
        </Button>
      </div>
      {#if showDetails}
        <Button
          variant="ghost"
          size="sm"
          aria-expanded={expanded}
          onclick={() => (expanded = !expanded)}
        >
          {expanded ? $t('errors.hideDetails') : $t('errors.showDetails')}
        </Button>
      {/if}
    </div>

    {#if showDetails && expanded}
      <div class="mt-3 border-t border-danger-line pt-3">
        <dl class="space-y-1 text-caption [overflow-wrap:anywhere]">
          <div class="flex gap-2">
            <dt>{$t('errors.errorCode')}</dt>
            <dd class="font-mono">{error.code}</dd>
          </div>
          <div class="flex gap-2">
            <dt>{$t('errors.technicalMessage')}</dt>
            <dd>{error.message}</dd>
          </div>
          <div class="flex gap-2">
            <dt>{$t('errors.referenceId')}</dt>
            <dd class="font-mono">{error.ref}</dd>
          </div>
        </dl>
        <Button variant="ghost" size="sm" class="-ml-2.5 mt-2" onclick={copyFullError}>
          {$t('errors.copyFullDetails')}
        </Button>
      </div>
    {/if}

    <p class="mt-2 text-caption">
      {$t('errors.supportHint')}
    </p>
  </Alert>
{/if}
