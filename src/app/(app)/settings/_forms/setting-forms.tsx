"use client";

import { useActionState } from "react";
import { saveDefault } from "../actions";
import { Status, keepValues } from "./status";

export function TermsForm({
  settingKey,
  title,
  terms,
}: {
  settingKey: string;
  title: string;
  terms: string;
}) {
  const [state, action, pending] = useActionState(saveDefault, {});

  return (
    <form onSubmit={keepValues(action)} className="card p-4">
      <input type="hidden" name="key" value={settingKey} />
      <h2 className="text-sm font-semibold">{title}</h2>
      <p className="mt-0.5 text-xs text-[var(--color-muted)]">
        Copied into each new document, where it can still be edited.
      </p>

      <textarea
        name="value"
        rows={6}
        defaultValue={terms}
        className="input mt-3"
        aria-label={title}
      />

      <div className="mt-3 flex items-center gap-3">
        <button type="submit" className="btn-primary" disabled={pending}>
          {pending ? "Saving…" : "Save"}
        </button>
        <Status state={state} />
      </div>
    </form>
  );
}

export function DefaultForm({
  settingKey,
  label,
  value,
  options,
}: {
  settingKey: string;
  label: string;
  value: string;
  options: string[];
}) {
  const [state, action, pending] = useActionState(saveDefault, {});

  return (
    <form onSubmit={keepValues(action)} className="rounded-md border border-[var(--color-line)] p-3">
      <input type="hidden" name="key" value={settingKey} />

      <label className="label" htmlFor={`default-${settingKey}`}>
        {label}
      </label>

      <select
        id={`default-${settingKey}`}
        name="value"
        defaultValue={value}
        className="select mt-2"
      >
        {Array.from(new Set([...options, value].filter(Boolean))).map((o) => (
          <option key={o} value={o}>
            {o}
          </option>
        ))}
      </select>

      <div className="mt-2 flex items-center gap-3">
        <button type="submit" className="btn-secondary" disabled={pending}>
          {pending ? "Saving…" : "Save"}
        </button>
        <Status state={state} />
      </div>
    </form>
  );
}
