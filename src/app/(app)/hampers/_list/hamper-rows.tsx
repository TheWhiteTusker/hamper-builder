"use client";

import { useLiveRows } from "@/components/use-live";
import { mergeHamper } from "@/lib/live-merge";
import type { HamperSummary } from "@/lib/types";
import { HamperRow } from "./hamper-row";

/** The list's rows, patched the moment someone else edits or bins a hamper. */
export function HamperRows({ rows: serverRows }: { rows: HamperSummary[] }) {
  const rows = useLiveRows(serverRows, "hampers", mergeHamper);
  return rows.map((h, i) => <HamperRow key={h.id} h={h} n={i + 1} />);
}
