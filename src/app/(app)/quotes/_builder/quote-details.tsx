"use client";

import Link from "@/components/link";
import type { Client, Quote, Settings } from "@/lib/types";
import { Field, TextField } from "./fields";
import { GstinField } from "./gstin-field";
import type { QuoteFields, SetField } from "./quote-state";

/** Document type, date, status and the client's details. */
export function QuoteDetails({
  f,
  set,
  clients,
  settings,
  canEdit,
  canChangeStatus,
  onPickClient,
  onDocType,
}: {
  f: QuoteFields;
  set: SetField;
  clients: Client[];
  settings: Settings;
  canEdit: boolean;
  canChangeStatus: boolean;
  onPickClient: (clientId: string) => void;
  onDocType: (next: Quote["doc_type"]) => void;
}) {
  const off = !canEdit;
  const text = (id: keyof QuoteFields, label: string, extra: Partial<React.ComponentProps<typeof TextField>> = {}) => (
    <TextField id={id} label={label} value={f[id]} onChange={(v) => set(id, v)} disabled={off} {...extra} />
  );

  return (
    <section className="card p-4">
      {canEdit && (
        <div className="mb-4 flex flex-wrap items-end gap-2 border-b border-[var(--color-line)] pb-4">
          <Field label="Saved client" htmlFor="savedClient" className="min-w-[280px]">
            <select id="savedClient" className="select mt-1" value="" onChange={(e) => onPickClient(e.target.value)}>
              <option value="">{clients.length ? "Fill details from a saved client…" : "No saved clients yet"}</option>
              {clients.map((c) => (
                <option key={c.id} value={c.id}>
                  {c.name}
                  {c.gstin ? ` — ${c.gstin}` : ""}
                </option>
              ))}
            </select>
          </Field>
          <Link href="/clients" className="btn-secondary">
            Manage clients
          </Link>
        </div>
      )}

      <div className="grid gap-4 md:grid-cols-3">
        <Field label="Document type" htmlFor="docType">
          <select
            id="docType"
            className="select mt-1"
            value={f.docType}
            onChange={(e) => onDocType(e.target.value as Quote["doc_type"])}
            disabled={off}
          >
            <option value="quotation">Quotation</option>
            <option value="proforma_invoice">Proforma Invoice</option>
          </select>
        </Field>

        {text("docDate", "Date", { type: "date" })}

        <Field label="Status" htmlFor="status">
          <select
            id="status"
            className="select mt-1"
            value={f.status}
            onChange={(e) => set("status", e.target.value)}
            disabled={off || !canChangeStatus}
          >
            {Array.from(new Set([...settings.quote_statuses, f.status])).map((s) => (
              <option key={s} value={s}>
                {s}
              </option>
            ))}
          </select>
          {!canChangeStatus && <p className="mt-1 text-xs text-[var(--color-muted)]">Only a manager can change status.</p>}
        </Field>

        {text("clientName", "Client / company", { required: true })}
        {text("contactPerson", "Contact person")}
        {text("occasion", "Occasion / project")}
        {text("email", "Email", { type: "email" })}
        {text("phone", "Phone")}
        <GstinField
          value={f.gstin}
          canEdit={canEdit}
          onChange={(v) => set("gstin", v)}
          onFound={(d) => {
            if (d.name) set("clientName", d.name);
            if (d.billing_address) set("billingAddress", d.billing_address);
          }}
        />

        <Field label="Billing address" htmlFor="billingAddress" className="md:col-span-2">
          <textarea
            id="billingAddress"
            rows={2}
            className="input mt-1"
            value={f.billingAddress}
            onChange={(e) => set("billingAddress", e.target.value)}
            disabled={off}
          />
        </Field>

        <div className="grid gap-4">
          <Field label="Validity" htmlFor="validity">
            <input
              id="validity"
              list="validity-options"
              className="input mt-1"
              value={f.validity}
              onChange={(e) => set("validity", e.target.value)}
              disabled={off}
            />
            <datalist id="validity-options">
              {settings.validity_options.map((v) => (
                <option key={v} value={v} />
              ))}
            </datalist>
          </Field>
          {text("followUp", "Follow up on", { type: "date" })}
        </div>
      </div>
    </section>
  );
}
