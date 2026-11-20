<script lang="ts">
  import { sessionTypeLabel } from '$lib/translations/labels';
  import { t } from '$lib/translations';
  import type { Session } from '$lib/api';
  import { ListRow } from '$lib/components/ui';
  import { fmt } from '$lib/format';

  /* One session in a patient's session list: type, first line of the notes, date and duration. */
  interface Props {
    session: Session;
    href?: string;
    onclick?: () => void;
  }

  let { session, href = undefined, onclick = undefined }: Props = $props();

  let snippet = $derived(session.notes?.trim().split('\n')[0] ?? '');
  let meta = $derived(
    [
      $fmt.date(session.session_date),
      session.duration_minutes ? $fmt.duration(session.duration_minutes) : '',
    ]
      .filter(Boolean)
      .join(' · ')
  );
</script>

<ListRow {href} {onclick} {meta}>
  <span class="shrink-0 text-body text-fg">{$sessionTypeLabel(session.session_type)}</span>
  {#if snippet}
    <span class="min-w-0 truncate text-label text-fg-muted">{snippet}</span>
  {:else}
    <span class="min-w-0 truncate text-label text-fg-subtle">{$t('sessions.noNotes')}</span>
  {/if}
</ListRow>
