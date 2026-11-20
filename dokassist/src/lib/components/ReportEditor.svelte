<script lang="ts">
  import { t } from '$lib/translations';
  import { Markdown, SegmentedControl, Textarea } from '$lib/components/ui';

  /* Plain report editor (Edit/Preview) without dictation or suggestions.
   * EnhancedReportEditor is the full version used by the report page. */
  let {
    content = $bindable(''),
    readonly = false,
  }: {
    content?: string;
    readonly?: boolean;
  } = $props();

  let mode = $state<'edit' | 'preview'>('edit');

  let modeOptions = $derived([
    { value: 'edit', label: $t('reports.edit') },
    { value: 'preview', label: $t('reports.preview') },
  ]);
</script>

<div class="space-y-3">
  <SegmentedControl
    options={modeOptions}
    bind:value={mode}
    label={$t('reports.editorMode')}
    size="sm"
  />

  {#if mode === 'preview'}
    <div
      class="min-h-80 rounded-card border border-line bg-surface-raised px-5 py-4 sm:px-8 sm:py-6"
    >
      {#if content}
        <Markdown source={content} headingOffset={1} />
      {:else}
        <p class="text-body text-fg-subtle">{$t('reports.noPreviewContent')}</p>
      {/if}
    </div>
  {:else}
    <Textarea
      bind:value={content}
      {readonly}
      rows={22}
      class="resize-y px-4 py-3 leading-relaxed"
      placeholder={$t('reports.contentPlaceholder')}
      aria-label={$t('reports.edit')}
    />
  {/if}
</div>
