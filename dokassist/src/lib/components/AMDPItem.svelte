<script lang="ts">
  import { t } from '$lib/translations';
  import type { AMDPItem } from '$lib/amdp';

  /* One AMDP item scored 0–3. The selected score is ink-filled (a selection, like a segment). */
  interface Props {
    item: AMDPItem;
    onScoreChange: (code: string, score: 0 | 1 | 2 | 3) => void;
  }

  let { item, onScoreChange }: Props = $props();

  const scores = [
    { value: 0, titleKey: 'amdp.scoreNone' },
    { value: 1, titleKey: 'amdp.scoreMild' },
    { value: 2, titleKey: 'amdp.scoreModerate' },
    { value: 3, titleKey: 'amdp.scoreSevere' },
  ] as const;

  function handleScoreClick(score: 0 | 1 | 2 | 3) {
    onScoreChange(item.code, score);
  }
</script>

<div
  class="flex items-center justify-between gap-3 rounded-control px-2 py-1 hover:bg-surface-hover"
>
  <span class="min-w-0 flex-1 text-body text-fg">
    <span class="mr-2 font-mono text-caption text-fg-subtle">{item.code}</span>
    {item.label}
  </span>
  <div class="flex shrink-0 gap-1" role="group" aria-label={item.label}>
    {#each scores as { value, titleKey } (value)}
      <button
        type="button"
        class="size-control-sm rounded-control border text-label font-medium transition-colors duration-150 ease-standard {item.score ===
        value
          ? 'border-accent bg-accent text-on-accent'
          : 'border-line bg-surface-raised text-fg-muted hover:bg-surface-hover'}"
        aria-pressed={item.score === value}
        onclick={() => handleScoreClick(value)}
        title={$t(titleKey)}
        aria-label="{value} {$t(titleKey)}"
      >
        {value}
      </button>
    {/each}
  </div>
</div>
