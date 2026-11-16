<script lang="ts">
  /**
   * Microphone button for local dictation. Click to start recording; click
   * again to stop, transcribe on-device and hand the text to `onTranscript`.
   * The speech model loads in the background while the user speaks.
   */
  import { onDestroy } from 'svelte';
  import { listen, type UnlistenFn } from '@tauri-apps/api/event';
  import { Mic, Square, X } from 'lucide-svelte';
  import {
    cancelDictation,
    getDictationStatus,
    startDictation,
    stopDictation,
    DICTATION_EVENTS,
  } from '$lib/api';
  import { addToast } from '$lib/stores/toast';
  import { errorText } from '$lib/translations/labels';
  import { t } from '$lib/translations';
  import Spinner from './ui/Spinner.svelte';

  let {
    onTranscript,
    disabled = false,
    class: className = '',
  }: {
    onTranscript: (text: string) => void;
    disabled?: boolean;
    class?: string;
  } = $props();

  type Phase = 'idle' | 'starting' | 'recording' | 'transcribing';
  let phase = $state<Phase>('idle');
  let dictationId: number | null = null;
  let level = $state(0);
  let unlistenLevel: UnlistenFn | null = null;

  function fail(error: unknown) {
    addToast($t('dictation.failed').replace('{error}', $errorText(error)), 'error');
  }

  async function start() {
    phase = 'starting';
    try {
      const status = await getDictationStatus();
      if (!status.worker_available) {
        addToast($t('dictation.workerMissing'), 'error');
        phase = 'idle';
        return;
      }
      if (!status.ready) {
        addToast($t('dictation.notReady'), 'error');
        phase = 'idle';
        return;
      }
      unlistenLevel = await listen<{ id: number; level: number }>(DICTATION_EVENTS.level, (e) => {
        if (e.payload.id === dictationId) level = e.payload.level;
      });
      dictationId = await startDictation();
      phase = 'recording';
    } catch (error) {
      cleanup();
      fail(error);
    }
  }

  async function stop() {
    if (dictationId === null) return;
    const id = dictationId;
    phase = 'transcribing';
    level = 0;
    try {
      const result = await stopDictation(id);
      const text = result.text.trim();
      if (text) {
        onTranscript(text);
      } else {
        addToast($t('dictation.empty'), 'error');
      }
      if (result.truncated) addToast($t('dictation.truncated'), 'error');
    } catch (error) {
      fail(error);
    } finally {
      cleanup();
    }
  }

  async function cancel() {
    const id = dictationId;
    cleanup();
    if (id !== null) await cancelDictation(id).catch(() => {});
  }

  function cleanup() {
    unlistenLevel?.();
    unlistenLevel = null;
    dictationId = null;
    level = 0;
    phase = 'idle';
  }

  function toggle() {
    if (phase === 'idle') start();
    else if (phase === 'recording') stop();
  }

  onDestroy(() => {
    // Navigating away mid-recording must close the microphone.
    if (dictationId !== null) cancelDictation(dictationId).catch(() => {});
    unlistenLevel?.();
  });

  // Ring grows with the input level so the user can see the mic is live.
  let ringScale = $derived(1 + Math.min(level * 3, 1) * 0.35);
</script>

<span class="inline-flex items-center gap-1 {className}">
  <button
    type="button"
    onclick={toggle}
    disabled={disabled || phase === 'starting' || phase === 'transcribing'}
    aria-label={phase === 'recording' ? $t('dictation.stop') : $t('dictation.start')}
    title={phase === 'recording'
      ? $t('dictation.stop')
      : phase === 'transcribing'
        ? $t('dictation.transcribing')
        : $t('dictation.start')}
    aria-pressed={phase === 'recording'}
    class="relative inline-flex h-8 w-8 items-center justify-center rounded-full transition-colors duration-150 ease-standard disabled:opacity-50 disabled:cursor-not-allowed {phase ===
    'recording'
      ? 'bg-danger text-on-danger'
      : 'text-fg-subtle hover:bg-surface-hover hover:text-fg'}"
  >
    {#if phase === 'recording'}
      <span
        class="absolute inset-0 rounded-full bg-danger/30 transition-transform duration-100"
        style="transform: scale({ringScale})"
        aria-hidden="true"
      ></span>
      <Square size={14} class="relative" />
    {:else if phase === 'starting' || phase === 'transcribing'}
      <Spinner size={14} />
    {:else}
      <Mic size={16} />
    {/if}
  </button>

  {#if phase === 'recording'}
    <button
      type="button"
      onclick={cancel}
      aria-label={$t('dictation.cancel')}
      title={$t('dictation.cancel')}
      class="inline-flex h-6 w-6 items-center justify-center rounded-full text-fg-subtle hover:bg-surface-hover hover:text-fg"
    >
      <X size={12} />
    </button>
    <span class="text-caption text-danger-fg" aria-live="polite">{$t('dictation.recording')}</span>
  {:else if phase === 'transcribing'}
    <span class="text-caption text-fg-muted" aria-live="polite">{$t('dictation.transcribing')}</span
    >
  {/if}
</span>
