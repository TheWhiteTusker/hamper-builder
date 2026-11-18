"use client";

import { useEffect, useRef, useState } from "react";
import { useRouter } from "next/navigation";

/**
 * Leaving the calculator: straight away when nothing has changed since it
 * opened, otherwise Save / Don't save / Cancel first. Covers "← Back", every
 * in-app link (nav, tabs, other colours) and, via the browser's own prompt,
 * reload or closing the tab. `to: null` means back.
 */
export function useLeaveGuard(snapshot: string, fallback: string) {
  const router = useRouter();
  const [initial] = useState(snapshot);
  const dirty = snapshot !== initial;
  const [asking, setAsking] = useState<{ to: string | null } | null>(null);

  useEffect(() => {
    if (!dirty) return;
    const onClick = (e: MouseEvent) => {
      const a = (e.target as Element | null)?.closest?.("a[href]") as HTMLAnchorElement | null;
      if (!a || a.target || a.hasAttribute("download") || e.button !== 0 || e.metaKey || e.ctrlKey || e.shiftKey || e.altKey) return;
      const url = new URL(a.href);
      if (url.origin !== location.origin || url.pathname === location.pathname) return;
      // Capture phase, before the Link's own handler navigates.
      e.preventDefault();
      e.stopPropagation();
      setAsking({ to: url.pathname + url.search + url.hash });
    };
    const onUnload = (e: BeforeUnloadEvent) => e.preventDefault();
    document.addEventListener("click", onClick, true);
    window.addEventListener("beforeunload", onUnload);
    return () => {
      document.removeEventListener("click", onClick, true);
      window.removeEventListener("beforeunload", onUnload);
    };
  }, [dirty]);

  function go(to: string | null) {
    if (to) router.push(to);
    else if (history.length > 1) router.back();
    else router.push(fallback); // opened in a fresh tab: nothing to go back to
  }

  return {
    asking: !!asking,
    back: () => (dirty ? setAsking({ to: null }) : go(null)),
    discard: () => {
      const to = asking?.to ?? null;
      setAsking(null);
      go(to);
    },
    cancel: () => setAsking(null),
  };
}

export function LeaveDialog({
  isPending,
  error,
  onSave,
  onDiscard,
  onCancel,
}: {
  isPending: boolean;
  error?: string;
  onSave: () => void;
  onDiscard: () => void;
  onCancel: () => void;
}) {
  const ref = useRef<HTMLDialogElement>(null);
  useEffect(() => ref.current?.showModal(), []);

  return (
    <dialog
      ref={ref}
      onClose={onCancel}
      aria-labelledby="leave-title"
      className="m-auto w-[min(92vw,26rem)] rounded-xl bg-white p-5 shadow-2xl backdrop:bg-black/50"
    >
      <h2 id="leave-title" className="text-base font-semibold">
        Save changes?
      </h2>
      <p className="mt-2 text-sm text-(--color-muted)">This product has changes that haven&apos;t been saved.</p>
      {error && (
        <p role="alert" className="mt-3 rounded-lg border border-red-200 bg-red-50 p-2.5 text-sm text-red-700">
          {error}
        </p>
      )}
      <div className="mt-5 flex justify-end gap-2">
        <button type="button" onClick={() => ref.current?.close()} className="btn-secondary" disabled={isPending}>
          Cancel
        </button>
        <button type="button" onClick={onDiscard} className="btn-secondary" disabled={isPending}>
          Don&apos;t save
        </button>
        <button type="button" onClick={onSave} className="btn-primary" disabled={isPending}>
          {isPending ? "Saving…" : "Save"}
        </button>
      </div>
    </dialog>
  );
}
