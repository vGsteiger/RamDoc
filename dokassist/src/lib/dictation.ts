/**
 * Insert dictated text into a textarea's value at the caret (or replacing
 * the selection), adding a separating space where needed, and move the caret
 * to the end of the inserted text. Returns the new value; assign it to the
 * bound state variable.
 */
export function insertDictation(
  el: HTMLTextAreaElement | null | undefined,
  current: string,
  text: string
): string {
  const value = current ?? '';
  const start = el ? el.selectionStart : value.length;
  const end = el ? el.selectionEnd : value.length;
  const before = value.slice(0, start);
  const after = value.slice(end);

  const needsLeadingSpace = before.length > 0 && !/\s$/.test(before);
  const needsTrailingSpace = after.length > 0 && !/^[\s.,;:!?)]/.test(after);
  const inserted = `${needsLeadingSpace ? ' ' : ''}${text}${needsTrailingSpace ? ' ' : ''}`;
  const next = before + inserted + after;

  if (el) {
    const caret = before.length + inserted.length;
    // Wait for the bound value to reach the DOM before moving the caret.
    queueMicrotask(() => {
      el.focus();
      el.setSelectionRange(caret, caret);
    });
  }
  return next;
}
