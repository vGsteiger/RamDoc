<script lang="ts">
  import { t } from '$lib/translations';
  import { thinkingEffort } from '$lib/stores/thinking';
  import { Select } from '$lib/components/ui';
  import type { ThinkingEffort } from '$lib/api';

  let { disabled = false }: { disabled?: boolean } = $props();

  const levels: ThinkingEffort[] = ['low', 'medium', 'high', 'extra_high'];
</script>

<label class="inline-flex items-center gap-2">
  <span class="text-caption text-fg-muted whitespace-nowrap">{$t('thinking.label')}</span>
  <Select
    bind:value={$thinkingEffort}
    {disabled}
    class="w-auto min-w-[9.5rem]"
    aria-label={$t('thinking.label')}
  >
    {#each levels as level (level)}
      <option value={level}>{$t(`thinking.${level}`)}</option>
    {/each}
  </Select>
</label>
