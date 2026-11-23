"use client";

/** A labelled read-only figure in the pricing strip. */
export function Cell({
  label,
  value,
  tone,
  strong,
}: {
  label: string;
  value: string;
  tone?: "good" | "bad";
  strong?: boolean;
}) {
  return (
    <div>
      <div className="label">{label}</div>
      <div
        className={[
          "mt-1 tabular-nums",
          strong ? "text-base font-semibold" : "text-sm font-medium",
          tone === "bad" ? "text-red-700" : tone === "good" ? "text-green-800" : "",
        ].join(" ")}
      >
        {value}
      </div>
    </div>
  );
}

export function NumField({
  id,
  label,
  value,
  onChange,
  disabled,
  hint,
}: {
  id: string;
  label: string;
  value: string;
  onChange: (v: string) => void;
  disabled?: boolean;
  hint?: string;
}) {
  return (
    <div className="w-30">
      <label className="label" htmlFor={id} title={hint}>
        {label}
      </label>
      <input
        id={id}
        inputMode="decimal"
        className="input input-num mt-1"
        value={value}
        onChange={(e) => onChange(e.target.value)}
        disabled={disabled}
      />
    </div>
  );
}

export function RowButton({
  label,
  onClick,
  disabled,
  danger,
  children,
}: {
  label: string;
  onClick: () => void;
  disabled?: boolean;
  danger?: boolean;
  children: React.ReactNode;
}) {
  return (
    <button
      type="button"
      aria-label={label}
      title={label}
      onClick={onClick}
      disabled={disabled}
      className={`rounded px-1.5 py-0.5 text-(--color-muted) disabled:opacity-30 ${
        danger ? "hover:bg-red-50 hover:text-red-700" : "hover:bg-paper"
      }`}
    >
      {children}
    </button>
  );
}
