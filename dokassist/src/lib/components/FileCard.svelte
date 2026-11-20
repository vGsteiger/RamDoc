<script lang="ts">
  import { isAudioMime, type FileRecord } from '$lib/api';
  import { Download, Eye, Trash2 } from 'lucide-svelte';
  import { t } from '$lib/translations';
  import { fmt } from '$lib/format';
  import { ListRow, Menu, type MenuItem } from '$lib/components/ui';
  import { fileExtension, fileIcon, formatFileSize } from '$lib/components/documents/file-meta';
  import { parseTimestamp } from '$lib/components/documents/documents';

  /*
   * One patient file as a dense list row: the row opens the viewer, and
   * Download / Delete live in the ⋯ menu (never hover-only).
   */
  interface Props {
    file: FileRecord;
    onView?: (file: FileRecord) => void;
    onDownload?: (file: FileRecord) => void;
    onDelete?: (file: FileRecord) => void;
    /** Prefix the secondary line with the kind ("File"), for mixed lists. */
    showKind?: boolean;
  }

  let { file, onView, onDownload, onDelete, showKind = false }: Props = $props();

  let Icon = $derived(fileIcon(file.mime_type));
  let created = $derived(parseTimestamp(file.created_at));

  let secondary = $derived(
    [
      showKind
        ? isAudioMime(file.mime_type)
          ? $t('files.recording')
          : $t('documents.kinds.file')
        : null,
      fileExtension(file.filename),
      formatFileSize(file.size_bytes),
    ]
      .filter(Boolean)
      .join(' · ')
  );

  let items = $derived<MenuItem[]>(
    [
      onView && { label: $t('files.view'), icon: Eye, onSelect: () => onView?.(file) },
      onDownload && {
        label: $t('files.download'),
        icon: Download,
        onSelect: () => onDownload?.(file),
      },
      onDelete && {
        label: $t('common.delete'),
        icon: Trash2,
        danger: true,
        onSelect: () => onDelete?.(file),
      },
    ].filter(Boolean) as MenuItem[]
  );
</script>

<ListRow
  title={file.filename}
  {secondary}
  onclick={onView ? () => onView?.(file) : undefined}
  data-document-kind="file"
>
  {#snippet leading()}
    <Icon size={16} aria-hidden="true" />
  {/snippet}
  {#snippet metaSnippet()}
    <time
      class="whitespace-nowrap text-caption text-fg-subtle"
      datetime={created?.toISOString()}
      title={$fmt.dateTime(created)}>{$fmt.date(created)}</time
    >
  {/snippet}
  {#snippet trailing()}
    {#if items.length > 0}
      <Menu {items} size="sm" label={$t('common.moreActions')} />
    {/if}
  {/snippet}
</ListRow>
