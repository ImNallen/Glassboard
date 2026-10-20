import { ArrowUpRight, Circle, Eraser, Highlighter, ListOrdered, Pencil, Square, Type } from '@lucide/svelte';
import type { Tool } from './drawing';

export const TOOL_ICONS: Record<Tool, typeof Pencil> = { arrow: ArrowUpRight, pen: Pencil, rectangle: Square, ellipse: Circle, step: ListOrdered, eraser: Eraser, text: Type, highlighter: Highlighter };
