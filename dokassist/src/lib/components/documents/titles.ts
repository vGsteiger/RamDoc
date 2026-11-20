import type { DocumentItem } from './documents';

type Translate = (key: string) => string;

/**
 * The human name of a document, used as the row title and in delete
 * confirmations. Report types are persisted in German and only their label is
 * translated, so the caller passes the `reportTypeLabel` store value.
 */
export function documentTitle(
  item: DocumentItem,
  t: Translate,
  reportTypeLabel: (value: string) => string
): string {
  switch (item.kind) {
    case 'file':
      return item.record.filename;
    case 'report':
      return reportTypeLabel(item.record.report_type);
    case 'letter':
      return item.record.subject.trim() || t(`letters.types.${item.record.letter_type}`);
    case 'email':
      return item.record.subject.trim() || t('email.noSubject');
  }
}
