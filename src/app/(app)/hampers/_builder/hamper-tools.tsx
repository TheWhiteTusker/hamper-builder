"use client";

import { useActionState } from "react";
import Link from "@/components/link";
import { duplicateHamper, deleteHamper } from "../actions";
import type { Hamper } from "@/lib/types";

/** Image, quote, duplicate and bin: kept outside the builder form so these never submit the hamper. */
export function HamperTools({ hamper }: { hamper: Hamper }) {
  const [dupState, dupAction, duplicating] = useActionState(duplicateHamper, {});
  const [delState, delAction, deleting] = useActionState(deleteHamper, {});
  const error = dupState.error || delState.error;

  return (
    <>
      {error && (
        <p role="alert" className="mt-4 text-sm text-red-700">
          {error}
        </p>
      )}
      <div className="mt-4 flex flex-wrap items-center gap-2">
        {hamper.image_url && (
          // eslint-disable-next-line @next/next/no-img-element
          <img
            src={hamper.image_url}
            alt={`${hamper.name} image`}
            className="h-16 w-16 rounded-md border border-(--color-line) object-cover"
          />
        )}
        {/* No prefetch: the image editor (and its canvas library) loads only when opened. */}
        <Link href={`/hampers/${encodeURIComponent(hamper.code)}/image`} prefetch={false} className="btn-secondary">
          {hamper.image_url ? "Edit image" : "Design image"}
        </Link>
        <Link href={`/quotes/new?hamper=${encodeURIComponent(hamper.code)}`} className="btn-secondary">
          Add to a quote
        </Link>

        <form action={dupAction}>
          <input type="hidden" name="code" value={hamper.code} />
          <button type="submit" className="btn-secondary" disabled={duplicating}>
            {duplicating ? "Duplicating…" : "Duplicate"}
          </button>
        </form>

        <form
          action={delAction}
          className="ml-auto"
          onSubmit={(e) => {
            if (!confirm(`Move ${hamper.name} to the Bin? It will stay in the Bin for 30 days and can be restored anytime.`)) {
              e.preventDefault();
            }
          }}
        >
          <input type="hidden" name="id" value={hamper.id} />
          <button type="submit" className="btn-danger" disabled={deleting}>
            {deleting ? "Moving to Bin…" : "Move to Bin"}
          </button>
        </form>
      </div>
    </>
  );
}
