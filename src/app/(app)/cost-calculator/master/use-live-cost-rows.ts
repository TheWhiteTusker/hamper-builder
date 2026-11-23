"use client";

import { useState } from "react";
import { useLiveChange } from "@/components/use-live";
import { applyCostChanges, type CostChange, type CostRows } from "@/lib/cost-tree";
import type { LiveChange } from "@/lib/live-refresh";

const TABLES = {
  cost_categories: "categories",
  cost_subcategories: "subcategories",
  cost_varieties: "varieties",
} as const;

/** A master change as a CostChange: the master shows active rows only. */
function toCostChange(c: LiveChange): CostChange | null {
  const table = TABLES[c.table as keyof typeof TABLES];
  if (!table) return null;
  if (c.eventType === "DELETE") return { table, remove: [c.old.id as string] };
  if (c.new.is_active === false) return { table, remove: [c.new.id as string] };
  return { table, upsert: [c.new] } as CostChange;
}

/**
 * The master's rows. Edits here patch them in place; anyone's change patches
 * them the moment it lands, new rows included; a server re-render (repricing,
 * a refresh) replaces them wholesale.
 */
export function useLiveCostRows(serverRows: CostRows) {
  const [rows, setRows] = useState(serverRows);
  const [prevServerRows, setPrevServerRows] = useState(serverRows);
  if (serverRows !== prevServerRows) {
    setPrevServerRows(serverRows);
    setRows(serverRows);
  }
  useLiveChange((c) => {
    const change = toCostChange(c);
    if (change) setRows((r) => applyCostChanges(r, [change]));
  });
  return [rows, setRows] as const;
}
