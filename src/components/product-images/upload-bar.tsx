"use client";

import { useState } from "react";
import type { ProductColor } from "@/lib/product-code";

/** Pick the finish to tag, upload files, or (when shown) add a photo by URL. */
export function UploadBar({
  colors,
  uploadColor,
  onUploadColor,
  isPrimary,
  onPrimary,
  showUrl,
  isPending,
  onFiles,
  onUrl,
}: {
  colors: ProductColor[];
  uploadColor: string;
  onUploadColor: (color: string) => void;
  isPrimary: boolean;
  onPrimary: (primary: boolean) => void;
  showUrl: boolean;
  isPending: boolean;
  onFiles: (files: File[]) => void;
  /** Adds the URL; calls `done` once it is saved so the box can clear. */
  onUrl: (url: string, done: () => void) => void;
}) {
  const [url, setUrl] = useState("");

  return (
    <>
      <div className="flex flex-wrap items-center gap-3 rounded-xl bg-slate-50 p-3 border border-slate-200">
        <div className="flex items-center gap-2">
          <label className="text-xs font-semibold text-slate-700">Tag Finish:</label>
          <select
            value={uploadColor}
            onChange={(e) => onUploadColor(e.target.value)}
            className="select text-xs py-1 px-2.5 bg-white min-w-32.5"
          >
            {colors.map((c) => (
              <option key={c.name} value={c.name}>
                {c.name} ({c.code})
              </option>
            ))}
            <option value="General">All Colors / General</option>
          </select>
        </div>

        <label className="flex items-center gap-1.5 text-xs text-slate-700 cursor-pointer">
          <input
            type="checkbox"
            checked={isPrimary}
            onChange={(e) => onPrimary(e.target.checked)}
            className="rounded text-(--color-brand) focus:ring-(--color-brand)"
          />
          Set as Primary Cover
        </label>

        <div className="ml-auto flex items-center gap-2">
          <input
            type="file"
            accept="image/jpeg,image/png,image/webp"
            multiple
            onChange={(e) => {
              const files = Array.from(e.target.files ?? []);
              e.target.value = "";
              onFiles(files);
            }}
            className="hidden"
            id="product-photo-upload"
          />
          <label
            htmlFor="product-photo-upload"
            className={`btn-primary text-xs py-1.5 px-3.5 cursor-pointer ${isPending ? "opacity-50 pointer-events-none" : ""}`}
          >
            {isPending ? "Uploading…" : "+ Upload Photos"}
          </label>
        </div>
      </div>

      {showUrl && (
        <div className="flex items-center gap-2 rounded-lg border border-slate-200 p-2.5 bg-white">
          <input
            value={url}
            onChange={(e) => setUrl(e.target.value)}
            placeholder="Paste public image URL (https://...)"
            className="input text-xs flex-1 py-1.5"
            onKeyDown={(e) => {
              if (e.key === "Enter") {
                e.preventDefault();
                onUrl(url, () => setUrl(""));
              }
            }}
          />
          <button
            type="button"
            onClick={() => onUrl(url, () => setUrl(""))}
            disabled={isPending || !url.trim()}
            className="btn-secondary text-xs py-1.5 px-3 whitespace-nowrap"
          >
            Add URL
          </button>
        </div>
      )}
    </>
  );
}
