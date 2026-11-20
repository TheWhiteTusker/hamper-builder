/** Shown instead of the photo manager until the product has been saved. */
export function SaveFirst() {
  return (
    <div className="rounded-xl border border-dashed border-slate-300 bg-slate-50/50 p-6 text-center">
      <div className="mx-auto flex h-10 w-10 items-center justify-center rounded-full bg-slate-100 text-slate-400">
        <svg className="h-5 w-5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M4 16l4.586-4.586a2 2 0 012.828 0L16 16m-2-2l1.586-1.586a2 2 0 012.828 0L20 14m-6-6h.01M6 20h12a2 2 0 002-2V6a2 2 0 00-2-2H6a2 2 0 00-2 2v12a2 2 0 002 2z" />
        </svg>
      </div>
      <h4 className="mt-2 text-sm font-semibold text-(--color-ink)">Photos & Color Finishes</h4>
      <p className="mt-1 text-xs text-(--color-muted)">
        Save this product first to upload photos for each of its color finishes.
      </p>
    </div>
  );
}
