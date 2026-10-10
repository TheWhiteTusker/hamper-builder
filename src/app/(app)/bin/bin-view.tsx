"use client";

import { Table, TableBody, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { useState, useTransition, useMemo } from "react";
import type { BinItem, BinCounts, BinItemType } from "./bin-types";
import { BinItemRow } from "./bin-item-row";
import { BinHeader } from "./bin-header";
import { BinToolbar } from "./bin-toolbar";
import { emptyBinAction } from "./bin-delete";
import { purgeExpiredAction } from "./bin-purge";

export function BinView({
  initialItems,
  counts,
  canManage,
  isAdmin,
}: {
  initialItems: BinItem[];
  counts: BinCounts;
  canManage: boolean;
  isAdmin: boolean;
}) {
  const [selectedTab, setSelectedTab] = useState<"all" | BinItemType>("all");
  const [searchQuery, setSearchQuery] = useState("");
  const [notification, setNotification] = useState<{
    text: string;
    error?: boolean;
  } | null>(null);

  const [isPending, startTransition] = useTransition();

  const filteredItems = useMemo(() => {
    return initialItems.filter((item) => {
      if (selectedTab !== "all" && item.type !== selectedTab) return false;
      if (searchQuery.trim()) {
        const q = searchQuery.toLowerCase();
        const matchesTitle = item.title.toLowerCase().includes(q);
        const matchesCode = (item.code || "").toLowerCase().includes(q);
        const matchesSubtitle = (item.subtitle || "").toLowerCase().includes(q);
        if (!matchesTitle && !matchesCode && !matchesSubtitle) return false;
      }
      return true;
    });
  }, [initialItems, selectedTab, searchQuery]);

  function handleEmptyBin() {
    const scopeLabel =
      selectedTab === "all"
        ? "all items"
        : selectedTab === "hamper"
          ? "all deleted hampers"
          : selectedTab === "product"
            ? "all deleted products"
            : "all deleted photos";

    if (
      !confirm(
        `Permanently delete ${scopeLabel} from the Bin? This will delete all records and stored images permanently and cannot be undone.`,
      )
    ) {
      return;
    }

    startTransition(async () => {
      const res = await emptyBinAction(selectedTab);
      if (res.error) {
        setNotification({ text: res.error, error: true });
      } else {
        setNotification({ text: res.message || "Bin emptied successfully." });
      }
    });
  }

  function handlePurgeExpired() {
    startTransition(async () => {
      const res = await purgeExpiredAction();
      if (res.error) {
        setNotification({ text: res.error, error: true });
      } else {
        setNotification({ text: res.message || "Expired items purged." });
      }
    });
  }

  return (
    <div className="space-y-4">
      {/* Informational notification or feedback toast */}
      {notification && (
        <div
          role="status"
          className={`flex items-center justify-between rounded-lg border px-4 py-3 text-sm transition-all ${
            notification.error
              ? "border-red-200 bg-red-50 text-red-800"
              : "border-line bg-paper text-(--color-ink)"
          }`}
        >
          <div className="flex items-center gap-2">
            <span
              className={`inline-block h-2 w-2 rounded-full ${
                notification.error ? "bg-red-500" : "bg-(--color-brand)"
              }`}
            />
            <span>{notification.text}</span>
          </div>
          <button
            type="button"
            onClick={() => setNotification(null)}
            className="text-xs font-semibold uppercase hover:underline"
          >
            Dismiss
          </button>
        </div>
      )}

      {/* Overview & explanation banner */}
      <BinHeader
        counts={counts}
        selectedTab={selectedTab}
        canManage={canManage}
        isAdmin={isAdmin}
        isPending={isPending}
        onEmptyBin={handleEmptyBin}
        onPurgeExpired={handlePurgeExpired}
      />

      {/* Tabs and Search Bar */}
      <BinToolbar
        counts={counts}
        selectedTab={selectedTab}
        onTabChange={setSelectedTab}
        searchQuery={searchQuery}
        onSearchChange={setSearchQuery}
      />

      {/* Items Table */}
      {filteredItems.length > 0 ? (
        <div className="card overflow-x-auto shadow-2xs">
          <Table className="min-w-212.5">
            <TableHeader>
              <TableRow>
                <TableHead className="w-14"></TableHead>
                <TableHead className="w-24">Type</TableHead>
                <TableHead className="w-32">Code</TableHead>
                <TableHead>Item Details</TableHead>
                <TableHead>Deleted On</TableHead>
                <TableHead>Retention</TableHead>
                <TableHead className="text-right">Actions</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {filteredItems.map((item) => (
                <BinItemRow
                  key={`${item.type}-${item.id}`}
                  item={item}
                  canManage={canManage}
                  isAdmin={isAdmin}
                  onNotify={setNotification}
                />
              ))}
            </TableBody>
          </Table>
        </div>
      ) : (
        <div className="card flex flex-col items-center justify-center p-12 text-center">
          <div className="mb-3 flex h-12 w-12 items-center justify-center rounded-full bg-paper border border-line text-xl text-(--color-muted)">
            🗑️
          </div>
          <h3 className="font-display text-base font-semibold text-(--color-ink)">
            {searchQuery
              ? "No matching items found"
              : selectedTab === "all"
                ? "The Bin is empty"
                : `No deleted ${selectedTab}s in the Bin`}
          </h3>
          <p className="mt-1 max-w-sm text-xs text-(--color-muted)">
            {searchQuery
              ? "Try adjusting your search query or switching tabs."
              : "When hampers, products, or photos are deleted, they will sit here for 30 days before permanent deletion."}
          </p>
        </div>
      )}
    </div>
  );
}
