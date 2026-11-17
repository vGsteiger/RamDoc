<script lang="ts">
  import type { Snippet } from 'svelte';
  import { ChevronDown } from 'lucide-svelte';

  let {
    value = $bindable(''),
    invalid = false,
    controlSize = 'md',
    class: className = '',
    children,
    ...rest
  }: {
    value?: string | number | null;
    invalid?: boolean;
    /** 28 / 32 / 36 px. Not `size`: that is a native input/select attribute. */
    controlSize?: 'sm' | 'md' | 'lg';
    class?: string;
    children?: Snippet;
    [key: string]: unknown;
  } = $props();

  const heights = { sm: 'h-control-sm', md: 'h-control', lg: 'h-control-lg' };

  let classes = $derived(
    [
      heights[controlSize],
      'w-full appearance-none rounded-control border bg-surface-raised py-0 pl-2.5 pr-8',
      'text-body text-fg transition-colors duration-150 ease-standard',
      'focus:outline-none focus-visible:outline-none',
      'disabled:cursor-not-allowed disabled:bg-surface-sunken disabled:text-fg-disabled',
      invalid
        ? 'border-danger focus:border-danger focus:ring-2 focus:ring-danger/25'
        : 'border-line focus:border-focus focus:ring-2 focus:ring-focus/25',
      className,
    ].join(' ')
  );
</script>

<div class="relative">
  <select bind:value class={classes} aria-invalid={invalid || undefined} {...rest}>
    {@render children?.()}
  </select>
  <ChevronDown
    size={14}
    class="pointer-events-none absolute right-2.5 top-1/2 -translate-y-1/2 text-fg-subtle"
    aria-hidden="true"
  />
</div>
