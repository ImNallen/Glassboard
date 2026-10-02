import { FADE_MS, type Shape } from './shapes';

const HISTORY_LIMIT = 100;

function sameShapes(left: Shape[], right: Shape[]): boolean {
  return left.length === right.length && left.every((shape, index) => shape.id === right[index].id);
}

export class DrawingHistory {
  shapes: Shape[] = [];
  private past: Shape[][] = [];
  private future: Shape[][] = [];
  private scheduleDirty = true;
  private nextExpiry = Infinity;
  private nextFade = Infinity;

  get canUndo() { return this.past.length > 0; }
  get canRedo() { return this.future.length > 0; }

  private commit(next: Shape[]) {
    this.past.push(this.shapes);
    if (this.past.length > HISTORY_LIMIT) this.past.shift();
    this.shapes = next;
    this.future = [];
    this.scheduleDirty = true;
  }

  add(shape: Shape, now = Date.now()) {
    const completed = {
      ...shape,
      points: shape.points.map(point => ({ ...point })),
      expiresAt: shape.fadeSeconds ? now + shape.fadeSeconds * 1000 : undefined,
    };
    this.commit([...this.shapes, completed]);
  }

  expire(now: number): boolean {
    this.updateSchedule();
    // Preserve the arrays (and their render cache) until an expiry is actually due.
    if (now < this.nextExpiry) return false;
    const previousCount = this.shapes.length;
    const couldUndo = this.canUndo;
    const couldRedo = this.canRedo;
    const alive = (shapes: Shape[]) => shapes.filter(shape =>
      shape.expiresAt === undefined || shape.expiresAt > now,
    );
    this.shapes = alive(this.shapes);

    // Remove expired shapes from every snapshot so undo/redo cannot resurrect them.
    // Collapse redundant steps while preserving edits to permanent drawings.
    const prune = (stack: Shape[][]) => {
      let previous = this.shapes;
      const result: Shape[][] = [];
      for (let index = stack.length - 1; index >= 0; index--) {
        const next = alive(stack[index]);
        if (!sameShapes(next, previous)) {
          result.push(next);
          previous = next;
        }
      }
      return result.reverse();
    };
    this.past = prune(this.past);
    this.future = prune(this.future);
    this.scheduleDirty = true;
    return previousCount !== this.shapes.length || couldUndo !== this.canUndo || couldRedo !== this.canRedo;
  }

  private updateSchedule() {
    if (!this.scheduleDirty) return;
    this.scheduleDirty = false;
    this.nextExpiry = Infinity;
    this.nextFade = Infinity;
    for (const shape of this.shapes) {
      if (shape.expiresAt !== undefined) {
        this.nextFade = Math.min(this.nextFade, shape.expiresAt - FADE_MS);
        this.nextExpiry = Math.min(this.nextExpiry, shape.expiresAt);
      }
    }
    // Invisible history only needs an update at expiry, not during the fade.
    for (const stack of [this.past, this.future]) {
      for (const shapes of stack) {
        for (const shape of shapes) {
          if (shape.expiresAt !== undefined) this.nextExpiry = Math.min(this.nextExpiry, shape.expiresAt);
        }
      }
    }
  }

  nextFadeUpdate(now: number): number | undefined {
    this.updateSchedule();
    const next = Math.min(this.nextExpiry, Math.max(now, this.nextFade));
    return Number.isFinite(next) ? next : undefined;
  }

  /** A new annotation session discards drawings and both history branches. */
  reset() {
    this.shapes = [];
    this.past = [];
    this.future = [];
    this.scheduleDirty = true;
  }

  clear() {
    if (this.shapes.length) this.commit([]);
  }

  /** Remove several shapes as a single undo step. */
  removeAll(ids: Iterable<string>): boolean {
    const removed = new Set(ids);
    const next = this.shapes.filter(shape => !removed.has(shape.id));
    if (next.length === this.shapes.length) return false;
    this.commit(next);
    return true;
  }

  undo() {
    const previous = this.past.pop();
    if (!previous) return;
    this.future.push(this.shapes);
    this.shapes = previous;
    this.scheduleDirty = true;
  }

  redo() {
    const next = this.future.pop();
    if (!next) return;
    this.past.push(this.shapes);
    this.shapes = next;
    this.scheduleDirty = true;
  }
}
