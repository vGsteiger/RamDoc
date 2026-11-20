<script lang="ts">
  import type { Snippet } from 'svelte';

  /*
   * One settings row: label and description on the left, the control on the
   * right. The control column never shrinks or wraps its labels, and the row
   * stacks (control below the text) when the row itself is narrower than
   * 560px — a container query, so it responds to the pane, not the window.
   *
   * The control snippet receives the ids to wire up accessibility:
   *
   *   <SettingsRow label={$t('settings.theme')} description={…}>
   *     {#snippet control({ labelId, descriptionId })}
   *       <SegmentedControl aria-labelledby={labelId} aria-describedby={descriptionId} … />
   *     {/snippet}
   *   </SettingsRow>
   *
   * or pass `for` to render a real <label for> pointing at a form control id.
   */
  let {
    label,
    description = undefined,
    for: forId = undefined,
    class: className = '',
    control,
    children,
  }: {
    label: string;
    description?: string;
    /** Id of the form control; renders the label as <label for>. */
    for?: string;
    class?: string;
    control?: Snippet<[{ labelId: string; descriptionId: string | undefined }]>;
    /** Extra content under the description (status lines, warnings). */
    children?: Snippet;
  } = $props();

  const uid = $props.id();
  const labelId = `${uid}-label`;
  let descriptionId = $derived(description ? `${uid}-description` : undefined);
</script>

<div class="@container border-b border-line-subtle last:border-b-0 {className}">
  <div
    class="flex flex-col gap-2 py-3 @min-[560px]:flex-row @min-[560px]:items-center @min-[560px]:justify-between @min-[560px]:gap-6"
  >
    <div class="min-w-0 flex-1">
      {#if forId}
        <label id={labelId} for={forId} class="block text-body font-medium text-fg">{label}</label>
      {:else}
        <p id={labelId} class="text-body font-medium text-fg">{label}</p>
      {/if}
      {#if description}
        <p id={descriptionId} class="mt-0.5 text-label text-fg-muted">{description}</p>
      {/if}
      {@render children?.()}
    </div>
    {#if control}
      <div class="flex min-w-0 shrink-0 flex-wrap items-center gap-2 whitespace-nowrap">
        {@render control({ labelId, descriptionId })}
      </div>
    {/if}
  </div>
</div>
