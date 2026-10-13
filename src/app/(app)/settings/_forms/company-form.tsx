"use client";

import { useActionState } from "react";
import { round2 } from "@/lib/pricing";
import type { CompanySettings } from "@/lib/types";
import { saveCompany } from "../actions";
import { Status, keepValues } from "./status";

export function CompanyForm({
  company,
  gstRate,
}: {
  company: CompanySettings;
  gstRate: number;
}) {
  const [state, action, pending] = useActionState(saveCompany, {});

  return (
    <form onSubmit={keepValues(action)} className="card p-4">
      <h2 className="text-sm font-semibold">Company details</h2>
      <p className="mt-0.5 text-xs text-[var(--color-muted)]">
        Printed at the top of every quotation and proforma invoice.
      </p>

      <div className="mt-4 grid gap-3 sm:grid-cols-2">
        <div>
          <label className="label" htmlFor="company-name">
            Company name
          </label>
          <input id="company-name" name="name" defaultValue={company.name} className="input mt-1" required />
        </div>

        <div>
          <label className="label" htmlFor="company-legal-name">
            Legal name (on invoices)
          </label>
          <input
            id="company-legal-name"
            name="legal_name"
            defaultValue={company.legal_name}
            placeholder="e.g. Palm Length LLP"
            className="input mt-1"
          />
        </div>

        <div className="sm:col-span-2">
          <label className="label" htmlFor="company-address">
            Address
          </label>
          <textarea
            id="company-address"
            name="address"
            rows={2}
            defaultValue={company.address}
            className="input mt-1"
          />
        </div>

        <div>
          <label className="label" htmlFor="company-gstin">
            GSTIN
          </label>
          <input id="company-gstin" name="gstin" defaultValue={company.gstin} className="input mt-1 font-mono" />
        </div>

        <div>
          <label className="label" htmlFor="company-phone">
            Phone
          </label>
          <input id="company-phone" name="phone" defaultValue={company.phone} className="input mt-1" />
        </div>

        <div>
          <label className="label" htmlFor="company-email">
            Email
          </label>
          <input id="company-email" name="email" defaultValue={company.email} className="input mt-1" />
        </div>

        <div>
          <label className="label" htmlFor="company-website">
            Website
          </label>
          <input id="company-website" name="website" defaultValue={company.website} className="input mt-1" />
        </div>

        <div>
          <label className="label" htmlFor="gst_rate">
            Default GST %
          </label>
          <input
            id="gst_rate"
            name="gst_rate"
            inputMode="decimal"
            defaultValue={String(round2(gstRate * 100))}
            className="input input-num mt-1"
          />
        </div>

        <div className="sm:col-span-2 border-t border-[var(--color-line)] pt-3">
          <h3 className="text-xs font-semibold">Bank details</h3>
          <p className="mt-0.5 text-xs text-[var(--color-muted)]">Printed on proforma invoices.</p>
        </div>

        <div>
          <label className="label" htmlFor="bank-account-name">
            Account name
          </label>
          <input
            id="bank-account-name"
            name="bank_account_name"
            defaultValue={company.bank_account_name}
            className="input mt-1"
          />
        </div>

        <div>
          <label className="label" htmlFor="bank-name">
            Bank
          </label>
          <input id="bank-name" name="bank_name" defaultValue={company.bank_name} className="input mt-1" />
        </div>

        <div>
          <label className="label" htmlFor="bank-account-no">
            Account number
          </label>
          <input
            id="bank-account-no"
            name="bank_account_no"
            defaultValue={company.bank_account_no}
            className="input mt-1 font-mono"
          />
        </div>

        <div>
          <label className="label" htmlFor="bank-ifsc">
            IFSC code
          </label>
          <input
            id="bank-ifsc"
            name="bank_ifsc"
            defaultValue={company.bank_ifsc}
            className="input mt-1 font-mono uppercase"
          />
        </div>
      </div>

      <div className="mt-4 flex items-center gap-3">
        <button type="submit" className="btn-primary" disabled={pending}>
          {pending ? "Saving…" : "Save"}
        </button>
        <Status state={state} />
      </div>
    </form>
  );
}
