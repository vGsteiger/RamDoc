<script lang="ts">
  let {
    value = $bindable(''),
    type = 'text',
    invalid = false,
    controlSize = 'md',
    class: className = '',
    ...rest
  }: {
    value?: string | number | null;
    type?: string;
    invalid?: boolean;
    /** 28 / 32 / 36 px. Not `size`: that is a native input/select attribute. */
    controlSize?: 'sm' | 'md' | 'lg';
    class?: string;
    [key: string]: unknown;
  } = $props();

  const heights = { sm: 'h-control-sm', md: 'h-control', lg: 'h-control-lg' };

  let classes = $derived(
    [
      heights[controlSize],
      'w-full rounded-control border border-transparent bg-fill px-3 text-body text-fg',
      'transition-colors duration-150 ease-standard',
      'focus:outline-none focus-visible:outline-none',
      'disabled:cursor-not-allowed disabled:bg-surface-sunken disabled:text-fg-disabled',
      invalid
        ? 'border-danger focus:border-danger focus:ring-2 focus:ring-danger/25'
        : 'focus:bg-surface-raised focus:ring-2 focus:ring-focus/30',
      className,
    ].join(' ')
  );
</script>

<!-- `type` cannot be spread dynamically onto a bound input in Svelte, so the
     common types are branched explicitly. -->
{#if type === 'number'}
  <input type="number" bind:value class={classes} aria-invalid={invalid || undefined} {...rest} />
{:else if type === 'date'}
  <input type="date" bind:value class={classes} aria-invalid={invalid || undefined} {...rest} />
{:else if type === 'password'}
  <input type="password" bind:value class={classes} aria-invalid={invalid || undefined} {...rest} />
{:else if type === 'email'}
  <input type="email" bind:value class={classes} aria-invalid={invalid || undefined} {...rest} />
{:else if type === 'search'}
  <input type="search" bind:value class={classes} aria-invalid={invalid || undefined} {...rest} />
{:else if type === 'tel'}
  <input type="tel" bind:value class={classes} aria-invalid={invalid || undefined} {...rest} />
{:else if type === 'url'}
  <input type="url" bind:value class={classes} aria-invalid={invalid || undefined} {...rest} />
{:else}
  <input type="text" bind:value class={classes} aria-invalid={invalid || undefined} {...rest} />
{/if}
