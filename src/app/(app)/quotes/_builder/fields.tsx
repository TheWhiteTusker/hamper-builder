export function Field({
  label,
  htmlFor,
  className,
  children,
}: {
  label: string;
  htmlFor: string;
  className?: string;
  children: React.ReactNode;
}) {
  return (
    <div className={className}>
      <label className="label" htmlFor={htmlFor}>
        {label}
      </label>
      {children}
    </div>
  );
}

export function Row({ label, value, strong }: { label: string; value: string; strong?: boolean }) {
  return (
    <div className="flex items-baseline justify-between gap-4">
      <dt className="text-(--color-muted)">{label}</dt>
      <dd className={`tabular-nums ${strong ? "text-base font-semibold" : "font-medium"}`}>{value || "—"}</dd>
    </div>
  );
}

/** A labelled input bound to one field. `className` is added to the input. */
export function TextField({
  id,
  label,
  value,
  onChange,
  disabled,
  className = "",
  wrapClassName,
  ...rest
}: {
  id: string;
  label: string;
  value: string;
  onChange: (value: string) => void;
  disabled: boolean;
  className?: string;
  wrapClassName?: string;
} & Omit<React.InputHTMLAttributes<HTMLInputElement>, "onChange" | "value">) {
  return (
    <Field label={label} htmlFor={id} className={wrapClassName}>
      <input
        id={id}
        className={`input mt-1 ${className}`.trim()}
        value={value}
        onChange={(e) => onChange(e.target.value)}
        disabled={disabled}
        {...rest}
      />
    </Field>
  );
}
