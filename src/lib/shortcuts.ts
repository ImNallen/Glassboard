import type { Tool } from './drawing';

export function eraseShortcut(event: Pick<KeyboardEvent, 'key' | 'metaKey' | 'ctrlKey' | 'altKey' | 'shiftKey' | 'repeat'>): boolean {
  return event.key.toLowerCase() === 'x' && !event.metaKey && !event.ctrlKey && !event.altKey && !event.shiftKey && !event.repeat;
}

export function toolShortcut(event: Pick<KeyboardEvent, 'key' | 'metaKey' | 'ctrlKey' | 'altKey' | 'shiftKey'>): Tool | undefined {
  if (!(event.metaKey || event.ctrlKey) || event.altKey || event.shiftKey) return;
  return ({ '1': 'arrow', '2': 'rectangle', '3': 'ellipse', '4': 'highlighter' } as Record<string, Tool>)[event.key];
}
