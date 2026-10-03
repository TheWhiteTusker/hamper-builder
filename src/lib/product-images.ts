import type { SyntheticEvent } from "react";
import type { ProductImage } from "./types.ts";
import { COLOR_TO_CODE, CODE_TO_COLOR, colorCode } from "./product-code.ts";

/**
 * Filter images for a specific color finish (e.g. "Walnut", "Natural", "Black" or "WL", "NT", "BL").
 * If includeGeneral is true (default), images with null/empty color are also included as fallback.
 */
export function getImagesForColor(
  images: ProductImage[] = [],
  colorOrCode?: string | null,
  includeGeneral: boolean = true,
): ProductImage[] {
  if (!images || images.length === 0) return [];
  if (!colorOrCode) return images;

  const key = colorOrCode.trim().toLowerCase();
  const normalizedCode = COLOR_TO_CODE[key] || colorCode(colorOrCode);
  const normalizedName = (CODE_TO_COLOR as Record<string, string>)[normalizedCode] || colorOrCode;

  const filtered = images.filter((img) => {
    if (!img.color && !img.color_code) return includeGeneral;
    const imgCode = img.color_code?.toUpperCase() || (img.color ? colorCode(img.color) : "");
    const imgName = img.color || (img.color_code ? (CODE_TO_COLOR as Record<string, string>)[img.color_code] : "");

    return (
      (normalizedCode && imgCode === normalizedCode) ||
      (imgName && imgName.toLowerCase() === normalizedName.toLowerCase())
    );
  });

  return filtered.length > 0 ? filtered : includeGeneral ? images : [];
}

/**
 * Get the best primary image URL for a product, prioritizing a requested color variant.
 */
export function getPrimaryImage(
  images: ProductImage[] = [],
  preferredColorOrCode?: string | null,
): ProductImage | null {
  if (!images || images.length === 0) return null;

  if (preferredColorOrCode) {
    const colorImages = getImagesForColor(images, preferredColorOrCode, false);
    if (colorImages.length > 0) {
      const primaryInColor = colorImages.find((img) => img.is_primary);
      if (primaryInColor) return primaryInColor;
      return colorImages[0];
    }
  }

  // Fallback to overall primary image
  const globalPrimary = images.find((img) => img.is_primary);
  if (globalPrimary) return globalPrimary;

  // Fallback to first image
  return images[0];
}

/**
 * Group images by color finish: "Walnut", "Natural", "Black", and "General"
 */
export function groupImagesByColor(
  images: ProductImage[] = [],
): Record<string, ProductImage[]> {
  const groups: Record<string, ProductImage[]> = {
    Walnut: [],
    Natural: [],
    Black: [],
    General: [],
  };

  for (const img of images) {
    const rawColor = (img.color || "").trim().toLowerCase();
    const code = img.color_code?.toUpperCase() || COLOR_TO_CODE[rawColor];

    if (code === "WL" || rawColor === "walnut") {
      groups.Walnut.push(img);
    } else if (code === "NT" || rawColor === "natural") {
      groups.Natural.push(img);
    } else if (code === "BL" || rawColor === "black") {
      groups.Black.push(img);
    } else {
      groups.General.push(img);
    }
  }

  return groups;
}

const PRODUCT_PHOTOS = "/product-images/products/";

/*
 * Each product photo is stored three ways. The original (usually PNG) is what
 * presentations and the studio place; the WebP copies are only for viewing:
 *   products/x/asset-1.png        original
 *   products/x/asset-1.view.webp  full-size view, 2000px
 *   products/x/asset-1.thumb.webp lists and galleries, 480px
 */
const copyPath = (path: string, kind: "thumb" | "view") => path.replace(/\.[^./]+$/, "") + `.${kind}.webp`;
export const thumbPath = (path: string) => copyPath(path, "thumb");
export const viewPath = (path: string) => copyPath(path, "view");

/** The WebP copy of one of our product photos; any other URL (external, hamper, blob) is unchanged. */
export const thumbUrl = (url: string) => (url.includes(PRODUCT_PHOTOS) ? thumbPath(url) : url);
export const viewUrl = (url: string) => (url.includes(PRODUCT_PHOTOS) ? viewPath(url) : url);

/** <img>/<Image> props showing `src`, falling back to the original if that copy is missing. */
function withFallback(src: string, url: string) {
  return {
    src,
    onError: (e: SyntheticEvent<HTMLImageElement>) => {
      if (e.currentTarget.src !== url) e.currentTarget.src = url;
    },
  };
}
export const thumbProps = (url: string) => withFallback(thumbUrl(url), url);
export const viewProps = (url: string) => withFallback(viewUrl(url), url);
