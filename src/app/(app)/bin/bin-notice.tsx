export type Notice = { text: string; error?: boolean };

/** The result of the last Bin action, until dismissed. */
export function BinNotice({ notice, onDismiss }: { notice: Notice; onDismiss: () => void }) {
  return (
    <div
      role="status"
      className={`flex items-center justify-between rounded-lg border px-4 py-3 text-sm transition-all ${
        notice.error
          ? "border-red-200 bg-red-50 text-red-800"
          : "border-line bg-paper text-(--color-ink)"
      }`}
    >
      <div className="flex items-center gap-2">
        <span
          className={`inline-block h-2 w-2 rounded-full ${
            notice.error ? "bg-red-500" : "bg-(--color-brand)"
          }`}
        />
        <span>{notice.text}</span>
      </div>
      <button
        type="button"
        onClick={onDismiss}
        className="text-xs font-semibold uppercase hover:underline"
      >
        Dismiss
      </button>
    </div>
  );
}
