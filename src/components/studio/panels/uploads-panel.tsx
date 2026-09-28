"use client";

import { Loader2, Upload } from "lucide-react";
import { DRAG_MIME, type Editor } from "../editor";
import { cx, panelTitle } from "../studio-ui";

export function UploadsPanel({ ed }: { ed: Editor }) {
  return (
    <div className="space-y-3">
      <label
        className={cx(
          "flex w-full cursor-pointer items-center justify-center gap-2 rounded-md bg-(--st-accent) px-3 py-2.5 text-[13px] font-medium text-(--st-on-accent) hover:bg-(--st-accent-strong)",
          ed.uploading && "pointer-events-none opacity-60",
        )}
      >
        {ed.uploading ? <Loader2 className="h-4 w-4 animate-spin" /> : <Upload className="h-4 w-4" />}
        {ed.uploading ? "Uploading…" : "Upload images"}
        <input
          type="file"
          accept="image/*"
          multiple
          className="hidden"
          onChange={(e) => {
            const files = Array.from(e.target.files ?? []);
            e.target.value = "";
            ed.uploadImages(files);
          }}
        />
      </label>
      <p className="text-[12px] text-(--st-muted)">
        Pick several at once (hold Ctrl or Shift in the file dialog). Each image is added to the page as its own layer.
      </p>

      {ed.uploads.length > 0 && (
        <>
          <div className={panelTitle}>Uploaded this session</div>
          <div className="grid grid-cols-2 gap-2">
            {ed.uploads.map((u) => (
              <button
                key={u.url}
                type="button"
                title={`Add ${u.name} again`}
                draggable
                onDragStart={(e) => {
                  e.dataTransfer.setData(DRAG_MIME, JSON.stringify({ product: null, url: u.url, name: u.name }));
                  e.dataTransfer.effectAllowed = "copy";
                }}
                onClick={() => ed.addImage(u.url, u.name)}
                className="checkerboard aspect-square overflow-hidden rounded-md border border-(--st-line) hover:border-(--st-accent)"
              >
                {/* eslint-disable-next-line @next/next/no-img-element */}
                <img src={u.url} alt="" className="h-full w-full object-contain" draggable={false} />
              </button>
            ))}
          </div>
        </>
      )}
    </div>
  );
}
