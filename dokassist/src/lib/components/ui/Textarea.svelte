<script lang="ts">
  let {
    value = $bindable(''),
    rows = 4,
    invalid = false,
    class: className = '',
    ...rest
  }: {
    value?: string;
    rows?: number;
    invalid?: boolean;
    class?: string;
    [key: string]: unknown;
  } = $props();

  let classes = $derived(
    [
      'w-full rounded-control border bg-surface-raised px-2.5 py-2 text-body text-fg',
      'transition-colors duration-150 ease-standard',
      'focus:outline-none focus-visible:outline-none',
      'disabled:cursor-not-allowed disabled:bg-surface-sunken disabled:text-fg-disabled',
      invalid
        ? 'border-danger focus:border-danger focus:ring-2 focus:ring-danger/25'
        : 'border-line focus:border-focus focus:ring-2 focus:ring-focus/25',
      className,
    ].join(' ')
  );
</script>

<textarea bind:value {rows} class={classes} aria-invalid={invalid || undefined} {...rest}
></textarea>
