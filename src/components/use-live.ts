"use client";

import { useEffect, useEffectEvent, useState } from "react";
import { applyRowChange, onLiveChange, type LiveChange } from "@/lib/live-refresh";

/** Run `handler` for every live change (see components/live-refresh.tsx). */
export function useLiveChange(handler: (c: LiveChange) => void) {
  // Always the latest handler, without re-subscribing when props change.
  const onChange = useEffectEvent(handler);
  useEffect(() => onLiveChange((c) => onChange(c)), []);
}

/**
 * Server rows, patched in place the moment `table` changes (~200 ms), so the
 * page doesn't wait ~1.5 s for the background refresh. That refresh then
 * replaces them, bringing new rows, filters and joined columns.
 *
 * ponytail: a refresh that started before a patch can land after it and
 * briefly show the older row; the next refresh fixes it. Compare updated_at
 * if that's ever seen.
 */
export function useLiveRows<T extends { id: string }>(
  serverRows: T[],
  table: string,
  merge: (cur: T, row: Record<string, unknown>) => T,
) {
  const [rows, setRows] = useState(serverRows);
  const [prev, setPrev] = useState(serverRows);
  if (serverRows !== prev) {
    setPrev(serverRows);
    setRows(serverRows);
  }
  useLiveChange((c) => {
    if (c.table === table) setRows((list) => applyRowChange(list, c, merge));
  });
  return rows;
}
