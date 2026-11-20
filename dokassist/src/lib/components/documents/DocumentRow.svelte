<script lang="ts">
  import type { ComponentType } from 'svelte';
  import { goto } from '$app/navigation';
  import { ClipboardList, ExternalLink, Mail, Pencil, ScrollText, Trash2 } from 'lucide-svelte';
  import { t } from '$lib/translations';
  import { reportTypeLabel } from '$lib/translations/labels';
  import { fmt } from '$lib/format';
  import { Badge, ListRow, Menu, type MenuItem } from '$lib/components/ui';
  import { documentHref, parseTimestamp, type DocumentItem } from './documents';
  import { documentTitle } from './titles';

  /*
   * A report, letter or email in the unified Documents list. The whole row
   * opens the document; Delete lives in the ⋯ menu. Files use FileCard.
   */
  type Item = Exclude<DocumentItem, { kind: 'file' }>;

  let {
    item,
    patientId,
    onDelete = undefined,
  }: {
    item: Item;
    patientId: string;
    onDelete?: (item: Item) => void;
  } = $props();

  const icons: Record<Item['kind'], ComponentType> = {
    report: ClipboardList as unknown as ComponentType,
    letter: ScrollText as unknown as ComponentType,
    email: Mail as unknown as ComponentType,
  };

  const statusTone = { draft: 'warning', final: 'info', sent: 'success' } as const;

  let Icon = $derived(icons[item.kind]);
  let href = $derived(documentHref(patientId, item) ?? undefined);
  let date = $derived(parseTimestamp(item.date));
  let title = $derived(documentTitle(item, $t, $reportTypeLabel));

  let secondary = $derived.by(() => {
    const kind = $t(`documents.kinds.${item.kind}`);
    switch (item.kind) {
      case 'report':
        return item.record.model_name ? `${kind} · ${item.record.model_name}` : kind;
      case 'letter': {
        const type = $t(`letters.types.${item.record.letter_type}`);
        const to = item.record.recipient_name
          ? ` · ${$t('documents.to').replace('{recipient}', item.record.recipient_name)}`
          : '';
        return `${kind} · ${type}${to}`;
      }
      case 'email':
        return `${kind} · ${$t('documents.to').replace('{recipient}', item.record.recipient_email)}`;
    }
  });

  // Sent emails were only ever deletable as drafts; keep that rule.
  let deletable = $derived(item.kind !== 'email' || item.status === 'draft');
  let editable = $derived(item.kind === 'email' && item.status === 'draft');

  let items = $derived<MenuItem[]>([
    {
      label: editable ? $t('documents.edit') : $t('documents.open'),
      icon: editable ? Pencil : ExternalLink,
      onSelect: () => {
        if (href) void goto(href);
      },
    },
    ...(deletable && onDelete
      ? [
          {
            label: $t('documents.delete'),
            icon: Trash2,
            danger: true,
            onSelect: () => onDelete?.(item),
          },
        ]
      : []),
  ]);
</script>

<ListRow {title} {secondary} {href} data-document-kind={item.kind}>
  {#snippet leading()}
    <Icon size={16} aria-hidden="true" />
  {/snippet}
  {#snippet metaSnippet()}
    <span class="flex items-center gap-2">
      {#if item.status}
        <Badge tone={statusTone[item.status]}>{$t(`documents.status.${item.status}`)}</Badge>
      {/if}
      <time
        class="whitespace-nowrap text-caption text-fg-subtle"
        datetime={date?.toISOString()}
        title={$fmt.dateTime(date)}>{$fmt.date(date)}</time
      >
    </span>
  {/snippet}
  {#snippet trailing()}
    <Menu {items} size="sm" label={$t('common.moreActions')} />
  {/snippet}
</ListRow>
