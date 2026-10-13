"use client";

import { useState, useTransition, useEffect } from "react";
import { Plus, Check, AlertCircle } from "lucide-react";
import type { Settings } from "@/lib/types";
import { markOtherPagesStale } from "@/components/stale-pages";
import { addPickListItem, removePickListItem } from "./actions";
import { DefaultForm } from "./_forms/setting-forms";
import { AddOptionDialog } from "./_picklists/add-option-dialog";
import { PICK_LIST_CONFIGS, type PickListKey } from "./_picklists/configs";
import { ListSelector } from "./_picklists/list-selector";
import { OptionList } from "./_picklists/option-list";
import { OptionSearch } from "./_picklists/option-search";

export function PickListManager({ settings }: { settings: Settings }) {
  const [lists, setLists] = useState<Record<PickListKey, string[]>>(
    () => Object.fromEntries(PICK_LIST_CONFIGS.map((c) => [c.key, settings[c.key] ?? []])) as Record<PickListKey, string[]>,
  );
  const [selectedKey, setSelectedKey] = useState<PickListKey>("hamper_statuses");
  const [search, setSearch] = useState("");
  // The add dialog, and what it opens pre-filled with.
  const [adding, setAdding] = useState<string | null>(null);
  const [statusMessage, setStatusMessage] = useState<{ type: "success" | "error"; text: string } | null>(null);
  const [isPending, startTransition] = useTransition();

  const activeConfig = PICK_LIST_CONFIGS.find((c) => c.key === selectedKey) ?? PICK_LIST_CONFIGS[0];
  const activeItems = lists[selectedKey] ?? [];
  const q = search.trim().toLowerCase();
  const filteredItems = q ? activeItems.filter((item) => item.toLowerCase().includes(q)) : activeItems;
  const hasExactMatch = activeItems.some((item) => item.toLowerCase() === q);
  const counts = Object.fromEntries(PICK_LIST_CONFIGS.map((c) => [c.key, (lists[c.key] ?? []).length])) as Record<PickListKey, number>;

  // Auto-dismiss status messages after 4 seconds
  useEffect(() => {
    if (!statusMessage) return;
    const timer = setTimeout(() => setStatusMessage(null), 4000);
    return () => clearTimeout(timer);
  }, [statusMessage]);

  /** Apply `next` at once, save, and roll back if the save fails. */
  function saveOptimistic(next: string[], save: () => Promise<{ error?: string }>, success: string) {
    const key = selectedKey;
    const previous = [...activeItems];
    startTransition(async () => {
      setLists((prev) => ({ ...prev, [key]: next }));
      const res = await save();
      if (res.error) {
        setLists((prev) => ({ ...prev, [key]: previous }));
        setStatusMessage({ type: "error", text: res.error });
      } else {
        markOtherPagesStale();
        setStatusMessage({ type: "success", text: success });
      }
    });
  }

  function handleAdd(value: string) {
    setAdding(null);
    setSearch(""); // so the newly added item is in view
    saveOptimistic([...activeItems, value], () => addPickListItem(selectedKey, value), `Added "${value}" to ${activeConfig.label}.`);
  }

  function handleRemove(item: string) {
    saveOptimistic(
      activeItems.filter((i) => i !== item),
      () => removePickListItem(selectedKey, item),
      `Removed "${item}" from ${activeConfig.label}.`,
    );
  }

  const openAdd = (prefill?: string) => setAdding((prefill ?? search).trim());

  return (
    <div className="space-y-4">
      <div className="grid gap-3 sm:grid-cols-2 lg:grid-cols-3">
        <div className="sm:col-span-2">
          <label className="label" id="picklist-label">
            Select Pick-list
          </label>
          <div className="mt-1 flex items-center gap-2">
            <ListSelector
              active={activeConfig}
              counts={counts}
              onSelect={(key) => {
                setSelectedKey(key);
                setSearch("");
              }}
            />
            <button
              type="button"
              onClick={() => openAdd()}
              className="btn-primary shrink-0 gap-1"
              title={`Add new item to ${activeConfig.label}`}
            >
              <Plus className="h-4 w-4" />
              <span>Add new</span>
            </button>
          </div>
          {activeConfig.hint && <p className="mt-1 text-xs text-[var(--color-muted)]">{activeConfig.hint}</p>}
        </div>
      </div>

      <OptionSearch
        value={search}
        onChange={setSearch}
        listLabel={activeConfig.label}
        hasExactMatch={hasExactMatch}
        onAdd={openAdd}
      />

      {statusMessage && (
        <div
          role="status"
          className={`flex items-center gap-2 rounded-lg px-3.5 py-2 text-xs font-medium transition-all ${
            statusMessage.type === "success"
              ? "bg-emerald-50 text-emerald-800 border border-emerald-200"
              : "bg-red-50 text-red-800 border border-red-200"
          }`}
        >
          {statusMessage.type === "success" ? (
            <Check className="h-4 w-4 shrink-0 text-emerald-600" />
          ) : (
            <AlertCircle className="h-4 w-4 shrink-0 text-red-600" />
          )}
          <span>{statusMessage.text}</span>
        </div>
      )}

      <OptionList
        items={filteredItems}
        total={activeItems.length}
        query={search.trim()}
        listLabel={activeConfig.label}
        isPending={isPending}
        onAdd={openAdd}
        onRemove={handleRemove}
      />

      {adding !== null && (
        <AddOptionDialog
          listLabel={activeConfig.label}
          initial={adding}
          existing={activeItems}
          isPending={isPending}
          onAdd={handleAdd}
          onClose={() => setAdding(null)}
        />
      )}

      {/* Here, not on the page, so an option added above is offered straight away. */}
      <div className="mt-6 border-t border-line pt-4">
        <h3 className="text-xs font-semibold uppercase tracking-wider text-(--color-muted)">
          Default Selections
        </h3>
        <div className="mt-3 grid gap-3 md:grid-cols-3">
          <DefaultForm
            settingKey="default_detail_mode"
            label="Default contents setting"
            value={settings.default_detail_mode}
            options={lists.detail_modes}
          />
          <DefaultForm
            settingKey="default_packaging_treatment"
            label="Default packaging treatment"
            value={settings.default_packaging_treatment}
            options={lists.packaging_treatments}
          />
          <DefaultForm
            settingKey="default_validity"
            label="Default validity"
            value={settings.default_validity}
            options={lists.validity_options}
          />
        </div>
      </div>
    </div>
  );
}
