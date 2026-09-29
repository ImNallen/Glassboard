/** Keep webview document selection from covering the annotation canvas. */
export function protectSelection(doc: Document = document) {
  const isEditable = (target: EventTarget | null) => target instanceof HTMLElement
    && (target.isContentEditable || Boolean(target.closest('input, textarea, select')));

  function clearSelection() {
    if (isEditable(doc.activeElement)) return;
    const selection = doc.getSelection();
    // Removing an existing range can itself fire selectionchange.
    if (selection && selection.rangeCount > 0) selection.removeAllRanges();
  }

  function keydown(event: KeyboardEvent) {
    if (isEditable(event.target)) return;
    if ((event.metaKey || event.ctrlKey) && !event.shiftKey && !event.altKey && event.key.toLowerCase() === 'a') {
      event.preventDefault();
      clearSelection();
    }
  }

  function selectstart(event: Event) {
    if (!isEditable(event.target)) event.preventDefault();
  }

  doc.addEventListener('keydown', keydown, true);
  doc.addEventListener('selectstart', selectstart);
  // macOS's native Edit > Select All can bypass the DOM keydown handler.
  doc.addEventListener('selectionchange', clearSelection);
  clearSelection();

  return () => {
    doc.removeEventListener('keydown', keydown, true);
    doc.removeEventListener('selectstart', selectstart);
    doc.removeEventListener('selectionchange', clearSelection);
  };
}
