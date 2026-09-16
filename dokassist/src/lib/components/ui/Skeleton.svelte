<script lang="ts">
  let {
    class: className = '',
  }: {
    class?: string;
  } = $props();
</script>

<span class="skeleton rounded-control {className}" aria-hidden="true"></span>
