"use client";

import { useActionState, useState, useTransition } from "react";
import Link from "@/components/link";
import { saveClient, deleteClient, lookupGstin } from "./actions";
import type { Client } from "@/lib/types";

export function ClientForm({ client, canDelete }: { client?: Client; canDelete: boolean }) {
  const [state, action, pending] = useActionState(saveClient, {});
  const [delState, delAction, deleting] = useActionState(deleteClient, {});

  const [gstin, setGstin] = useState(client?.gstin ?? "");
  const [name, setName] = useState(client?.name ?? "");
  const [address, setAddress] = useState(client?.billing_address ?? "");
  const [lookupError, setLookupError] = useState("");
  const [fetching, startFetch] = useTransition();

  function fetchDetails() {
    startFetch(async () => {
      const result = await lookupGstin(gstin);
      setLookupError(result.error ? String(result.error) : "");
      if (result.details) {
        setName(result.details.name);
        setAddress(result.details.billing_address);
      }
    });
  }

  return (
    <div className="card p-4">
      <h2 className="text-sm font-semibold">{client ? "Edit client" : "New client"}</h2>

      <form action={action} className="mt-3 space-y-3">
        {client && <input type="hidden" name="id" value={client.id} />}

        <div>
          <label className="label" htmlFor="client-gstin">
            GST number
          </label>
          <div className="mt-1 flex gap-2">
            <input
              id="client-gstin"
              name="gstin"
              value={gstin}
              onChange={(e) => setGstin(e.target.value.toUpperCase())}
              maxLength={15}
              placeholder="29AAECB4326R1Z5"
              className="input font-mono uppercase"
            />
            <button
              type="button"
              onClick={fetchDetails}
              disabled={fetching || gstin.length !== 15}
              className="btn-secondary"
            >
              {fetching ? "Fetching…" : "Fetch details"}
            </button>
          </div>
          {lookupError && <p className="mt-1 text-xs text-amber-700">{lookupError}</p>}
        </div>

        <div>
          <label className="label" htmlFor="client-name">
            Company name *
          </label>
          <input
            id="client-name"
            name="name"
            required
            value={name}
            onChange={(e) => setName(e.target.value)}
            className="input mt-1"
          />
        </div>

        <div>
          <label className="label" htmlFor="client-address">
            Billing address
          </label>
          <textarea
            id="client-address"
            name="billing_address"
            rows={3}
            value={address}
            onChange={(e) => setAddress(e.target.value)}
            className="input mt-1"
          />
        </div>

        <div className="grid gap-3 sm:grid-cols-2">
          <div>
            <label className="label" htmlFor="client-contact">
              Contact person
            </label>
            <input
              id="client-contact"
              name="contact_person"
              defaultValue={client?.contact_person ?? ""}
              className="input mt-1"
            />
          </div>
          <div>
            <label className="label" htmlFor="client-phone">
              Phone
            </label>
            <input
              id="client-phone"
              name="phone"
              defaultValue={client?.phone ?? ""}
              className="input mt-1"
            />
          </div>
          <div className="sm:col-span-2">
            <label className="label" htmlFor="client-email">
              Email
            </label>
            <input
              id="client-email"
              name="email"
              type="email"
              defaultValue={client?.email ?? ""}
              className="input mt-1"
            />
          </div>
        </div>

        {state.error && (
          <p role="alert" className="text-sm text-red-700">
            {state.error}
          </p>
        )}

        <div className="flex items-center gap-2">
          <button type="submit" className="btn-primary" disabled={pending}>
            {pending ? "Saving…" : client ? "Save changes" : "Add client"}
          </button>
          {client && (
            <Link href="/clients" className="btn-secondary">
              Cancel
            </Link>
          )}
        </div>
      </form>

      {client && canDelete && (
        <form action={delAction} className="mt-4 border-t border-[var(--color-line)] pt-3">
          <input type="hidden" name="id" value={client.id} />
          {delState.error && (
            <p role="alert" className="mb-2 text-sm text-red-700">
              {delState.error}
            </p>
          )}
          <button type="submit" className="btn-danger" disabled={deleting}>
            {deleting ? "Deleting…" : "Delete client"}
          </button>
        </form>
      )}
    </div>
  );
}
