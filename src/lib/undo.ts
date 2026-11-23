// One undo/redo history per tab, newest last: ticks, review marks and moves
// between screens (components/undo-redo.tsx). In memory, so a reload starts
// it fresh.

type Step = { undo: () => void; redo: () => void };

const past: Step[] = [];
const future: Step[] = [];
const MAX = 100;

/** A new change: undoable, and it ends any redo trail. */
export function record(step: Step) {
  past.push(step);
  if (past.length > MAX) past.shift();
  future.length = 0;
}

/** Reverses the latest change; false when there's none. */
export function undo(): boolean {
  const step = past.pop();
  if (!step) return false;
  step.undo();
  future.push(step);
  return true;
}

/** Re-applies the latest undone change; false when there's none. */
export function redo(): boolean {
  const step = future.pop();
  if (!step) return false;
  step.redo();
  past.push(step);
  return true;
}
