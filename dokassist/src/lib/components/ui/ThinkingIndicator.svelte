<script lang="ts">
  import { t } from '$lib/translations';
  import {
    chatActivityLabel,
    ELAPSED_AFTER_SECONDS,
    formatElapsed,
    type ChatActivityStage,
  } from '$lib/chat-activity';

  let {
    stage = 'thinking',
    startedAt,
    toolName = null,
    label: labelOverride = undefined,
    class: className = '',
  }: {
    stage?: ChatActivityStage;
    startedAt: number;
    toolName?: string | null;
    label?: string;
    class?: string;
  } = $props();

  let now = $state(Date.now());

  $effect(() => {
    void startedAt;
    now = Date.now();
    const id = setInterval(() => {
      now = Date.now();
    }, 250);
    return () => clearInterval(id);
  });

  let elapsedSeconds = $derived(Math.max(0, Math.floor((now - startedAt) / 1000)));
  let label = $derived(labelOverride ?? chatActivityLabel(stage, elapsedSeconds, $t, toolName));
  let showElapsed = $derived(elapsedSeconds >= ELAPSED_AFTER_SECONDS);
</script>

<span class="inline-flex items-center gap-2 text-body text-fg-muted {className}">
  <span class="thinking-dots" aria-hidden="true">
    <span class="thinking-dot"></span>
    <span class="thinking-dot"></span>
    <span class="thinking-dot"></span>
  </span>
  <!-- Timer ticks every second; keep it out of the live region so SRs do not re-announce. -->
  <span role="status" aria-live="polite">{label}</span>
  {#if showElapsed}
    <span class="text-caption text-fg-subtle" data-numeric>{formatElapsed(elapsedSeconds)}</span>
  {/if}
</span>
