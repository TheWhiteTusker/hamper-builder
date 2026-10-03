"use client";

import { useRef } from "react";
import Image from "next/image";
import { thumbProps, viewProps } from "@/lib/product-images";

/**
 * A thumbnail that opens the full image in a modal. Native <dialog> gives
 * Esc-to-close and focus trapping; clicking the backdrop closes it too.
 */
export function ImagePreview({
  src,
  alt,
  className,
  sizes,
}: {
  src: string;
  alt: string;
  className?: string;
  sizes: string;
}) {
  const ref = useRef<HTMLDialogElement>(null);

  return (
    <>
      <button
        type="button"
        onClick={() => ref.current?.showModal()}
        title={`Preview ${alt}`}
        className={`relative block cursor-zoom-in overflow-hidden ${className ?? ""}`}
      >
        <Image {...thumbProps(src)} alt={alt} fill sizes={sizes} className="object-cover" unoptimized />
      </button>

      <dialog
        ref={ref}
        onClick={(e) => e.target === ref.current && ref.current.close()}
        className="m-auto max-h-[96vh] max-w-[96vw] rounded-xl bg-white p-2 shadow-2xl backdrop:bg-black/70"
      >
        {/* Fixed large box; object-contain scales small photos up and big ones down without cropping. */}
        {/* eslint-disable-next-line @next/next/no-img-element -- any host, no optimisation needed */}
        <img {...viewProps(src)} alt={alt} loading="lazy" className="block h-[86vh] w-[94vw] max-w-[1400px] object-contain" />
        <div className="flex items-center justify-between gap-3 px-1 pt-2 text-xs">
          <span className="truncate text-[var(--color-muted)]">{alt}</span>
          <form method="dialog">
            <button className="btn-secondary py-1 text-xs">Close</button>
          </form>
        </div>
      </dialog>
    </>
  );
}
