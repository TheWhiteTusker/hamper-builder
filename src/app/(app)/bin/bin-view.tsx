"use client";

import { Table, TableBody, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { useState, useTransition, useMemo } from "react";
import Link from "@/components/link";
import { markOtherPagesStale } from "@/components/stale-pages";
import { PageHeader } from "@/components/ui";
import { binCounts, type BinItem, type BinItemType } from "./bin-types";
import { BinNotice, type Notice } from "./bin-notice";
import { BinItemRow } from "./bin-item-row";
import { BinHeader } from "./bin-header";
import { BinToolbar } from "./bin-toolbar";
import { emptyBinAction } from "./bin-delete";
import { purgeExpiredAction } from "./bin-purge";

export function BinView({
  initialItems,
  canManage,
  isAdmin,
}: {
  initialItems: BinItem[];
  canManage: boolean;
  isAdmin: boolean;
}) {
  // Actions drop rows here instead of re-rendering the page on the server.
  const [items, setItems] = useState(initialItems);
  const [prevInitial, setPrevInitial] = useState(initialItems);
  if (initialItems !== prevInitial) {
    setPrevInitial(initialItems);
    setItems(initialItems);
  }
  const counts = binCounts(items);

  /** Remove items for good; a product deleted for good takes its binned photos with it. */
  function purge(gone: (item: BinItem) => boolean) {
    setItems((prev) => {
      const products = new Set(prev.filter((i) => i.type === "product" && gone(i)).map((i) => i.id));
      return prev.filter((i) => !gone(i) && !(i.type === "image" && products.has(i.metadata?.productId ?? "")));
    });
    markOtherPagesStale();
  }

  function handleGone(item: BinItem, permanently: boolean) {
    if (permanently) return purge((i) => i.id === item.id && i.type === item.type);
    setItems((prev) => prev.filter((i) => i !== item));
    markOtherPagesStale();
  }
  const [selectedTab, setSelectedTab] = useState<"all" | BinItemType>("all");
  const [searchQuery, setSearchQuery] = useState("");
  const [notification, setNotification] = useState<Notice | null>(null);

  const [isPending, startTransition] = useTransition();

  const filteredItems = useMemo(() => {
    return items.filter((item) => {
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
  }, [items, selectedTab, searchQuery]);

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
        const emptied = res.emptied ?? [];
        purge((i) => emptied.includes(i.type));
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
        purge((i) => i.daysRemaining === 0);
        setNotification({ text: res.message || "Expired items purged." });
      }
    });
  }

  return (
    <div className="space-y-4">
      <PageHeader title="Recycle Bin" subtitle={`${counts.all} item${counts.all === 1 ? "" : "s"} in bin`}>
        <Link href="/settings" className="btn-secondary">
          Settings
        </Link>
      </PageHeader>

      {/* Informational notification or feedback toast */}
      {notification && <BinNotice notice={notification} onDismiss={() => setNotification(null)} />}

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
                  onGone={handleGone}
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
