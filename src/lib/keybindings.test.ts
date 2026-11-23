import test from "node:test";
import assert from "node:assert/strict";
import { actionFor, assign, comboFromKey, comboFromMouse, DEFAULT_BINDINGS } from "./keybindings.ts";

const key = (k: string, mods: Partial<Record<"ctrlKey" | "metaKey" | "altKey" | "shiftKey", boolean>> = {}) =>
  comboFromKey({ key: k, ctrlKey: false, metaKey: false, altKey: false, shiftKey: false, ...mods });

test("key presses become combos; ⌘ counts as Ctrl; modifiers alone aren't combos", () => {
  assert.equal(key("z", { ctrlKey: true }), "Ctrl+Z");
  assert.equal(key("z", { metaKey: true }), "Ctrl+Z");
  assert.equal(key("Z", { ctrlKey: true, shiftKey: true }), "Ctrl+Shift+Z");
  assert.equal(key("?", { shiftKey: true }), "Shift+?");
  assert.equal(key("Shift", { shiftKey: true }), null);
  assert.equal(comboFromMouse(3), "Mouse4");
  assert.equal(comboFromMouse(0), null); // left click stays a click
});

test("defaults find their actions", () => {
  assert.equal(actionFor(DEFAULT_BINDINGS, "Ctrl+Z"), "undo");
  assert.equal(actionFor(DEFAULT_BINDINGS, "Ctrl+Shift+Z"), "redo");
  assert.equal(actionFor(DEFAULT_BINDINGS, "P"), "products");
  assert.equal(actionFor(DEFAULT_BINDINGS, "X"), null);
});

test("a combo used elsewhere moves to the new slot, like a game", () => {
  const { bindings, movedFrom } = assign(DEFAULT_BINDINGS, "clients", 0, "P");
  assert.equal(movedFrom, "products");
  assert.deepEqual(bindings.products, ["", ""]);
  assert.equal(actionFor(bindings, "P"), "clients");
  assert.deepEqual(DEFAULT_BINDINGS.products, ["P", ""]); // the defaults are untouched

  const same = assign(DEFAULT_BINDINGS, "redo", 0, "Ctrl+Shift+Z"); // from its own alternate
  assert.equal(same.movedFrom, null);
  assert.deepEqual(same.bindings.redo, ["Ctrl+Shift+Z", ""]);
});
