"use client";

import { useActionState } from "react";
import { deleteProduct } from "../actions";
import { ListReturnField } from "@/components/list-select/list-position";

/** Deletes a product after a confirm. Saved hampers keep their own copy of its line. */
export function DeleteProductButton({ id, label }: { id: string; label: string }) {
  const [state, action, pending] = useActionState(deleteProduct, {});

  return (
    <form
      action={action}
      onSubmit={(e) => {
        if (!confirm(`Move ${label} to the Bin? It will stay in the Bin for 30 days and can be restored anytime.`)) e.preventDefault();
      }}
      className="inline"
    >
      <input type="hidden" name="id" value={id} />
      <ListReturnField kind="products" />
      <button
        type="submit"
        disabled={pending}
        className="font-medium text-red-700 hover:underline disabled:opacity-50"
      >
        {pending ? "Moving to Bin…" : "Delete"}
      </button>
      {state.error && (
        <span role="alert" className="mt-1 block max-w-60 whitespace-normal text-left text-xs text-red-700">
          {state.error}
        </span>
      )}
    </form>
  );
}
