/**
 * Whether `target` is in a form field or editable content, where keys and copies belong to the text instead of the drawing.
 * A text node stands for its element, and the window or document for the focused element.
 */
export function isEditableTarget(target: EventTarget | null): boolean {
  const element = target instanceof Element ? target : target instanceof Text ? target.parentElement : document.activeElement;
  return Boolean(element?.closest('input, textarea, select, [contenteditable]:not([contenteditable="false"])'));
}

/** Keep webview document selection from covering the annotation canvas. */
export function protectSelection(doc: Document = document) {

  function clearSelection() {
    if (isEditableTarget(doc.activeElement)) return;
    const selection = doc.getSelection();
    // Removing an existing range can itself fire selectionchange.
    if (selection && selection.rangeCount > 0) selection.removeAllRanges();
  }

  function keydown(event: KeyboardEvent) {
    if (isEditableTarget(event.target)) return;
    if ((event.metaKey || event.ctrlKey) && !event.shiftKey && !event.altKey && event.key.toLowerCase() === 'a') {
      event.preventDefault();
      clearSelection();
    }
  }

  function selectstart(event: Event) {
    if (!isEditableTarget(event.target)) event.preventDefault();
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
