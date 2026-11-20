"use client";

import { useMemo, useState } from "react";
import { Check, ChevronDown, Search } from "lucide-react";
import { FONTS } from "../render";
import { Popover, cx, fieldCls } from "./base";

export function FontPicker({ value, onChange }: { value: string; onChange: (family: string) => void }) {
  const [q, setQ] = useState("");
  const list = useMemo(
    () => FONTS.filter((f) => f.family.toLowerCase().includes(q.trim().toLowerCase())),
    [q],
  );
  return (
    <Popover
      title="Font"
      width={240}
      className="w-44 justify-between border border-(--st-line) bg-(--st-panel-2)"
      trigger={
        <>
          <span className="truncate" style={{ fontFamily: value }}>
            {value}
          </span>
          <ChevronDown className="h-3.5 w-3.5 shrink-0 text-(--st-muted)" />
        </>
      }
    >
      {(close) => (
        <>
          <div className="relative mb-2">
            <Search className="pointer-events-none absolute left-2 top-2 h-4 w-4 text-(--st-muted)" />
            <input
              autoFocus
              className={cx(fieldCls, "w-full pl-8")}
              placeholder="Search fonts"
              value={q}
              onChange={(e) => setQ(e.target.value)}
            />
          </div>
          <div className="max-h-72 overflow-y-auto">
            {list.map((f) => (
              <button
                key={f.family}
                type="button"
                onClick={() => {
                  onChange(f.family);
                  close();
                }}
                className={cx(
                  "flex w-full items-center justify-between rounded px-2 py-1.5 text-left text-[15px] hover:bg-(--st-hover)",
                  f.family === value && "text-(--st-accent)",
                )}
                style={{ fontFamily: f.family }}
              >
                {f.family}
                {f.family === value && <Check className="h-4 w-4" />}
              </button>
            ))}
          </div>
        </>
      )}
    </Popover>
  );
}
