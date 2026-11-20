"use client";

import { useState } from "react";
import type { ProductColor } from "@/lib/product-code";
import type { ProductImage } from "@/lib/types";
import { ImageCard } from "./product-images/image-card";
import { SaveFirst } from "./product-images/save-first";
import { UploadBar } from "./product-images/upload-bar";
import { useImageActions } from "./product-images/use-image-actions";

const tabClass = (active: boolean) =>
  `rounded-md px-3 py-1.5 text-xs font-semibold transition-all ${
    active ? "bg-(--color-brand) text-white shadow-xs" : "text-(--color-muted) hover:bg-slate-100"
  }`;

const isOfColor = (img: ProductImage, col: ProductColor) =>
  img.color?.toLowerCase() === col.name.toLowerCase() || img.color_code?.toUpperCase() === col.code;
const isGeneral = (img: ProductImage) => !img.color && !img.color_code;

type Props = {
  productId?: string | null;
  initialImages?: ProductImage[];
  productName?: string;
  currentColor?: string | null;
  /** The finishes this product comes in; one photo tab each. */
  colors: ProductColor[];
  /** Told about every upload, delete or re-tag, so a parent can mirror the list. */
  onImagesChange?: (images: ProductImage[]) => void;
  /** False where photos are uploaded elsewhere (the calculator's colour rows); the gallery then only manages them. */
  canUpload?: boolean;
};

export function ProductImagesManager(props: Props) {
  return props.productId ? <Manager {...props} productId={props.productId} /> : <SaveFirst />;
}

function Manager({
  productId,
  initialImages = [],
  productName,
  currentColor,
  colors,
  onImagesChange,
  canUpload = true,
}: Props & { productId: string }) {
  const act = useImageActions(productId, initialImages, onImagesChange);
  const { images, feedback } = act;
  const [activeTab, setActiveTab] = useState("all");
  const [uploadColor, setUploadColor] = useState(currentColor || colors[0]?.name || "General");
  const [isPrimaryUpload, setIsPrimaryUpload] = useState(images.length === 0);
  const [showUrlInput, setShowUrlInput] = useState(false);

  const tab = activeTab.toLowerCase();
  const filteredImages = images.filter((img) =>
    activeTab === "all"
      ? true
      : activeTab === "general"
        ? isGeneral(img)
        : img.color?.toLowerCase() === tab || img.color_code?.toLowerCase() === tab,
  );
  const countGeneral = images.filter(isGeneral).length;

  return (
    <div className="space-y-4">
      <div className="flex flex-wrap items-center justify-between gap-3 border-b border-(--color-border) pb-3">
        <div>
          <h3 className="text-sm font-bold text-(--color-ink)">Product Images & Color Variations</h3>
          <p className="text-xs text-(--color-muted) mt-0.5">
            {canUpload
              ? "Pick a color tab, then upload its photos. Multiple images supported per color."
              : "Set the cover, change a photo's color or remove it. Add photos in the color rows above."}
          </p>
        </div>
        {canUpload && (
          <button
            type="button"
            onClick={() => setShowUrlInput(!showUrlInput)}
            className="text-xs text-(--color-brand) hover:underline font-medium"
          >
            {showUrlInput ? "Hide URL Option" : "+ Add via URL"}
          </button>
        )}
      </div>

      {/* Filter Tabs */}
      <div className="flex flex-wrap items-center gap-1 border-b border-slate-200 pb-2">
        <button type="button" onClick={() => setActiveTab("all")} className={tabClass(activeTab === "all")}>
          All Photos ({images.length})
        </button>
        {colors.map((col) => (
          <button
            key={col.name}
            type="button"
            onClick={() => {
              setActiveTab(col.name);
              setUploadColor(col.name);
            }}
            className={`flex items-center gap-1.5 ${tabClass(tab === col.name.toLowerCase())}`}
          >
            <span className="h-2 w-2 rounded-full border border-black/20" style={{ backgroundColor: col.hex }} />
            {col.name} ({images.filter((i) => isOfColor(i, col)).length})
          </button>
        ))}
        {countGeneral > 0 && (
          <button type="button" onClick={() => setActiveTab("general")} className={tabClass(activeTab === "general")}>
            General ({countGeneral})
          </button>
        )}
      </div>

      {canUpload && (
        <UploadBar
          colors={colors}
          uploadColor={uploadColor}
          onUploadColor={setUploadColor}
          isPrimary={isPrimaryUpload}
          onPrimary={setIsPrimaryUpload}
          showUrl={showUrlInput}
          isPending={act.isPending}
          onFiles={(files) => act.upload(files, uploadColor, isPrimaryUpload)}
          onUrl={(url, done) =>
            act.addUrl(url, uploadColor, isPrimaryUpload, () => {
              done();
              setShowUrlInput(false);
            })
          }
        />
      )}

      {feedback.error && (
        <div className="rounded-lg bg-red-50 p-2.5 text-xs font-medium text-red-700 border border-red-200">
          {feedback.error}
        </div>
      )}
      {feedback.success && (
        <div className="rounded-lg bg-emerald-50 p-2.5 text-xs font-medium text-emerald-800 border border-emerald-200">
          {feedback.success}
        </div>
      )}

      {filteredImages.length === 0 ? (
        <div className="rounded-lg border border-dashed border-slate-200 p-8 text-center text-xs text-(--color-muted)">
          No images uploaded for {activeTab === "all" ? "this product" : `${activeTab} finish`} yet.
        </div>
      ) : (
        <div className="grid grid-cols-2 gap-3 sm:grid-cols-3 md:grid-cols-4">
          {filteredImages.map((img) => (
            <ImageCard
              key={img.id}
              img={img}
              colors={colors}
              color={colors.find((c) => isOfColor(img, c))}
              productName={productName}
              onDelete={() => act.remove(img.id)}
              onColor={(c) => act.changeColor(img.id, c)}
              onPrimary={() => act.setPrimary(img.id)}
            />
          ))}
        </div>
      )}
    </div>
  );
}
