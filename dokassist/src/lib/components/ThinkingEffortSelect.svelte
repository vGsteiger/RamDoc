<script lang="ts">
  import { t } from '$lib/translations';
  import { thinkingEffort } from '$lib/stores/thinking';
  import { Select } from '$lib/components/ui';
  import type { ThinkingEffort } from '$lib/api';

  let {
    disabled = false,
    size = 'md',
  }: {
    disabled?: boolean;
    /** `sm` is the quiet 28 px variant used in the chat composer footer. */
    size?: 'sm' | 'md';
  } = $props();

  const levels: ThinkingEffort[] = ['low', 'medium', 'high', 'extra_high'];
</script>

<label class="inline-flex min-w-0 items-center gap-2">
  <span class="whitespace-nowrap text-caption text-fg-muted">{$t('thinking.label')}</span>
  <Select
    bind:value={$thinkingEffort}
    {disabled}
    controlSize={size}
    class="w-auto {size === 'sm' ? 'min-w-[7.5rem]' : 'min-w-[9.5rem]'}"
    aria-label={$t('thinking.label')}
  >
    {#each levels as level (level)}
      <option value={level}>{$t(`thinking.${level}`)}</option>
    {/each}
  </Select>
</label>
