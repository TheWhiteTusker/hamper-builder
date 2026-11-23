import { test } from "node:test";
import assert from "node:assert/strict";
import { applyRowChange, emitLiveChange, isOthersChange, onLiveChange, type LiveChange } from "./live-refresh.ts";

const ME = "me";
const change = (table: string, eventType: LiveChange["eventType"], row: Record<string, unknown>): LiveChange =>
  eventType === "DELETE" ? { table, eventType, new: {}, old: row } : { table, eventType, new: row, old: {} };

test("isOthersChange skips your own writes and refreshes for everyone else's", () => {
  assert.equal(isOthersChange(change("products", "UPDATE", { id: "1", updated_by: "other" }), ME), true);
  assert.equal(isOthersChange(change("products", "UPDATE", { id: "1", updated_by: ME }), ME), false);
  assert.equal(isOthersChange(change("clients", "INSERT", { id: "1", updated_by: ME }), ME), false);
  assert.equal(isOthersChange(change("clients", "INSERT", { id: "1", updated_by: "other" }), ME), true);
  // Cron and service-role writes have no user.
  assert.equal(isOthersChange(change("products", "UPDATE", { id: "1", updated_by: null }), ME), true);
});

test("isOthersChange: child-table deletes ride on the parent's update, other deletes refresh", () => {
  for (const t of ["hamper_items", "quote_items", "presentation_slides"]) {
    assert.equal(isOthersChange(change(t, "DELETE", { id: "1" }), ME), false);
  }
  for (const t of ["products", "clients"]) {
    assert.equal(isOthersChange(change(t, "DELETE", { id: "1" }), ME), true);
  }
});

type R = { id: string; name: string; total?: number };
const rows: R[] = [
  { id: "a", name: "A", total: 1 },
  { id: "b", name: "B", total: 2 },
];
const merge = (cur: R, row: Record<string, unknown>) => ({ ...cur, name: row.name as string });

test("applyRowChange replaces a listed row and keeps the others identical", () => {
  const next = applyRowChange(rows, change("products", "UPDATE", { id: "b", name: "B2" }), merge);
  assert.deepEqual(next[1], { id: "b", name: "B2", total: 2 });
  assert.equal(next[0], rows[0]);
  assert.notEqual(next, rows);
});

test("applyRowChange leaves rows it doesn't show to the refresh", () => {
  assert.equal(applyRowChange(rows, change("products", "UPDATE", { id: "z", name: "Z" }), merge), rows);
  assert.equal(applyRowChange(rows, change("products", "INSERT", { id: "c", name: "C" }), merge), rows);
});

test("applyRowChange drops deleted and binned rows", () => {
  assert.deepEqual(applyRowChange(rows, change("products", "DELETE", { id: "a" }), merge).map((r) => r.id), ["b"]);
  const binned = change("products", "UPDATE", { id: "b", name: "B", deleted_at: "2026-11-23T10:00:00Z" });
  assert.deepEqual(applyRowChange(rows, binned, merge).map((r) => r.id), ["a"]);
});

test("onLiveChange hears changes until unsubscribed", () => {
  const heard: string[] = [];
  const off = onLiveChange((c) => heard.push(c.table));
  emitLiveChange(change("products", "UPDATE", { id: "1" }));
  off();
  emitLiveChange(change("clients", "UPDATE", { id: "1" }));
  assert.deepEqual(heard, ["products"]);
});
