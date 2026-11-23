import test from "node:test";
import assert from "node:assert/strict";
import { record, redo, undo } from "./undo.ts";

test("undo reverses newest first, redo re-applies, a new change ends the redo trail", () => {
  const log: string[] = [];
  const step = (name: string) => ({ undo: () => log.push(`undo ${name}`), redo: () => log.push(`redo ${name}`) });

  record(step("tick"));
  record(step("open product"));
  assert.equal(undo(), true);
  assert.equal(undo(), true);
  assert.equal(undo(), false); // nothing left: the app falls back to going back a screen
  assert.equal(redo(), true);
  record(step("mark"));
  assert.equal(redo(), false); // "open product" was dropped by the new change

  assert.deepEqual(log, ["undo open product", "undo tick", "redo tick"]);
});
