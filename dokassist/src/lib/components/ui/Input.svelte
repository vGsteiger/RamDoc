<script lang="ts">
  let {
    value = $bindable(''),
    type = 'text',
    invalid = false,
    class: className = '',
    ...rest
  }: {
    value?: string | number | null;
    type?: string;
    invalid?: boolean;
    class?: string;
    [key: string]: unknown;
  } = $props();

  let classes = $derived(
    [
      'h-8 w-full rounded-control border bg-surface-raised px-2.5 text-body text-fg',
      'transition-colors duration-150 ease-standard',
      'focus:outline-none focus-visible:outline-none',
      'disabled:cursor-not-allowed disabled:bg-surface-sunken disabled:text-fg-disabled',
      invalid
        ? 'border-danger focus:border-danger focus:ring-2 focus:ring-danger/25'
        : 'border-line focus:border-accent focus:ring-2 focus:ring-accent/25',
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
