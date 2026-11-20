<script lang="ts">
  import type { Snippet } from 'svelte';
  import { Check } from 'lucide-svelte';
  import { BrandMark } from '$lib/components/ui';
  import { t } from '$lib/translations';

  /*
   * Shared frame for the four onboarding steps: brand mark, a labelled step
   * indicator, the step title, the step body in a card, and a footer row for
   * Back / Skip / Continue. Visual only; each step keeps its own logic.
   */
  let {
    step,
    title,
    subtitle = undefined,
    children,
    actions,
  }: {
    /** 1-based current step. */
    step: 1 | 2 | 3 | 4;
    title: string;
    subtitle?: string;
    children?: Snippet;
    /** Footer buttons; the last one is the step's single primary action. */
    actions?: Snippet;
  } = $props();

  const steps = ['practice', 'model', 'import', 'overview'] as const;
</script>

<div class="flex min-h-screen justify-center bg-surface px-4 py-10 text-fg sm:px-8">
  <div class="w-full max-w-form">
    <div class="flex items-center gap-2">
      <BrandMark size={24} />
      <span class="text-heading text-fg">RamDoc</span>
    </div>

    <nav class="mt-8" aria-label={$t('onboarding.stepsLabel')}>
      <ol class="flex flex-wrap items-center gap-x-3 gap-y-2">
        {#each steps as key, i (key)}
          {@const n = i + 1}
          {@const done = n < step}
          {@const current = n === step}
          <li class="flex items-center gap-2" aria-current={current ? 'step' : undefined}>
            <span
              class="flex size-5 shrink-0 items-center justify-center rounded-full border text-caption font-medium {current
                ? 'border-accent bg-accent text-on-accent'
                : done
                  ? 'border-line-strong text-fg'
                  : 'border-line text-fg-subtle'}"
              aria-hidden="true"
            >
              {#if done}<Check size={12} />{:else}{n}{/if}
            </span>
            <span class="text-label {current ? 'font-medium text-fg' : 'text-fg-muted'}">
              {$t(`onboarding.steps.${key}`)}
            </span>
            {#if done}<span class="sr-only">{$t('onboarding.stepDone')}</span>{/if}
            {#if n < steps.length}
              <span class="ml-1 hidden h-px w-6 bg-line sm:block" aria-hidden="true"></span>
            {/if}
          </li>
        {/each}
      </ol>
    </nav>

    <header class="mt-8">
      <p class="text-caption text-fg-subtle">
        {$t('onboarding.stepOf')
          .replace('{current}', String(step))
          .replace('{total}', String(steps.length))}
      </p>
      <h1 class="mt-1 text-display text-fg">{title}</h1>
      {#if subtitle}
        <p class="mt-2 text-body text-fg-muted">{subtitle}</p>
      {/if}
    </header>

    <div class="mt-6">
      {@render children?.()}
    </div>

    {#if actions}
      <div class="mt-8 flex flex-wrap items-center justify-between gap-3">
        {@render actions()}
      </div>
    {/if}
  </div>
</div>
