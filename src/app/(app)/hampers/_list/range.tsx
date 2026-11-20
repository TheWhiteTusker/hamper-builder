/** A from-to pair on one numeric column; either end may be left blank. */
export function Range({
  name,
  label,
  min,
  max,
}: {
  name: string;
  label: string;
  min: string;
  max: string;
}) {
  return (
    <div className="flex items-center gap-1">
      <input
        name={`${name}_min`}
        aria-label={`Minimum ${label}`}
        inputMode="decimal"
        defaultValue={min}
        placeholder="min"
        className="input input-num"
      />
      <span className="text-(--color-muted)">–</span>
      <input
        name={`${name}_max`}
        aria-label={`Maximum ${label}`}
        inputMode="decimal"
        defaultValue={max}
        placeholder="max"
        className="input input-num"
      />
    </div>
  );
}
