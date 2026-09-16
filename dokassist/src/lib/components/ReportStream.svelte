<script lang="ts">
  import { t } from '$lib/translations';
  import { ThinkingIndicator } from '$lib/components/ui';
  import { THINK_END, THINK_START, chatActivityStage } from '$lib/chat-activity';

  export let content: string = '';
  export let isStreaming: boolean = false;
  export let isSummarizing: boolean = false;

  let thinkContent = '';
  let reportContent = '';
  let activityStartedAt: number | null = null;

  $: {
    if (content.startsWith(THINK_START)) {
      const endIdx = content.indexOf(THINK_END);
      if (endIdx !== -1) {
        thinkContent = content.slice(THINK_START.length, endIdx).trim();
        reportContent = content.slice(endIdx + THINK_END.length).trim();
      } else {
        thinkContent = content.slice(THINK_START.length).trim();
        reportContent = '';
      }
    } else {
      thinkContent = '';
      reportContent = content;
    }
  }

  $: if (isStreaming) {
    if (activityStartedAt == null) activityStartedAt = Date.now();
  } else {
    activityStartedAt = null;
  }

  $: activityStage = chatActivityStage(content);
  $: activityLabel =
    isStreaming && isSummarizing && !thinkContent && !reportContent
      ? $t('reports.activityCompressing')
      : activityStage === 'reasoning'
        ? $t('chat.activity.reasoning')
        : activityStage === 'writing'
          ? $t('reports.activityWriting')
          : $t('chat.activity.thinking');
</script>

<div class="space-y-4">
  {#if thinkContent}
    <div class="bg-surface-hover border border-line rounded-card p-4">
      <p class="text-caption font-medium text-fg-subtle uppercase tracking-wide mb-2">
        {$t('reports.streamThinking')}
      </p>
      <pre
        class="not-prose whitespace-pre-wrap font-sans text-body text-fg-muted italic">{thinkContent}</pre>
      {#if isStreaming && !reportContent && activityStartedAt}
        <div class="mt-2">
          <ThinkingIndicator
            stage="reasoning"
            startedAt={activityStartedAt}
            label={$t('chat.activity.reasoning')}
          />
        </div>
      {/if}
    </div>
  {/if}

  <div class="bg-surface-raised border border-line rounded-card p-6 min-h-[300px] relative">
    {#if isStreaming && reportContent && activityStartedAt}
      <div class="absolute top-4 right-4">
        <ThinkingIndicator
          stage="writing"
          startedAt={activityStartedAt}
          label={$t('reports.activityWriting')}
        />
      </div>
    {/if}

    <div class="prose dark:prose-invert max-w-none">
      {#if reportContent}
        <pre class="not-prose whitespace-pre-wrap font-sans text-fg">{reportContent}</pre>
      {:else if !isStreaming && !thinkContent}
        <p class="text-fg-subtle italic">{$t('reports.streamWillAppear')}</p>
      {:else if isStreaming && activityStartedAt && !thinkContent}
        <ThinkingIndicator
          stage={activityStage}
          startedAt={activityStartedAt}
          label={activityLabel}
        />
      {:else if !isStreaming && thinkContent && !reportContent}
        <p class="text-fg-subtle italic">{$t('reports.streamEmpty')}</p>
      {/if}
    </div>
  </div>
</div>
