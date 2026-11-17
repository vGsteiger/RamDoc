/**
 * RamDoc UI primitives.
 *
 * These are the only place raw visual decisions live. Views compose them and
 * use the semantic token utilities from app.css (bg-surface, text-fg-muted,
 * border-line, …) — never raw palette utilities such as bg-gray-100.
 *
 * See docs/design-language.md.
 */
export { default as Alert } from './Alert.svelte';
export { default as Badge } from './Badge.svelte';
export { default as BrandMark } from './BrandMark.svelte';
export { default as Button } from './Button.svelte';
export { default as Card } from './Card.svelte';
export { default as ConfirmDialog } from './ConfirmDialog.svelte';
export { default as Dialog } from './Dialog.svelte';
export { default as EmptyState } from './EmptyState.svelte';
export { default as Field } from './Field.svelte';
export { default as HighlightedSnippet } from './HighlightedSnippet.svelte';
export { default as IconButton } from './IconButton.svelte';
export { default as Input } from './Input.svelte';
export { default as Kbd } from './Kbd.svelte';
export { default as ListRow } from './ListRow.svelte';
export { default as ListSkeleton } from './ListSkeleton.svelte';
export { default as Markdown } from './Markdown.svelte';
export { default as Menu } from './Menu.svelte';
export { default as PageHeader } from './PageHeader.svelte';
export { default as PageShell } from './PageShell.svelte';
export { default as ProgressBar } from './ProgressBar.svelte';
export { default as SectionHeader } from './SectionHeader.svelte';
export { default as SegmentedControl } from './SegmentedControl.svelte';
export { default as Select } from './Select.svelte';
export { default as SettingsRow } from './SettingsRow.svelte';
export { default as Skeleton } from './Skeleton.svelte';
export { default as Spinner } from './Spinner.svelte';
export { default as StatusIndicator } from './StatusIndicator.svelte';
export { default as Tabs } from './Tabs.svelte';
export { default as Textarea } from './Textarea.svelte';
export { default as ThinkingIndicator } from './ThinkingIndicator.svelte';
export { default as Tooltip } from './Tooltip.svelte';

export type { MenuItem } from './Menu.svelte';
export type { SegmentOption } from './SegmentedControl.svelte';
export type { TabItem } from './Tabs.svelte';
export { parseMarkdown, safeHref } from './markdown';
export { parseMarkedSnippet, segmentsFromRanges, findMatchRanges } from './highlight';
