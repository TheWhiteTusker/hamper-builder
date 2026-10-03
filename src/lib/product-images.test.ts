import { test } from "node:test";
import assert from "node:assert/strict";
import {
  getImagesForColor,
  getPrimaryImage,
  groupImagesByColor,
  thumbPath,
  thumbUrl,
  viewPath,
  viewUrl,
} from "./product-images.ts";
import type { ProductImage } from "./types.ts";

const mockImages: ProductImage[] = [
  {
    id: "img-1",
    product_id: "prod-1",
    url: "https://example.com/walnut-front.jpg",
    color: "Walnut",
    color_code: "WL",
    is_primary: true,
    sort_order: 1,
    created_at: new Date().toISOString(),
    updated_at: new Date().toISOString(),
  },
  {
    id: "img-2",
    product_id: "prod-1",
    url: "https://example.com/walnut-back.jpg",
    color: "Walnut",
    color_code: "WL",
    is_primary: false,
    sort_order: 2,
    created_at: new Date().toISOString(),
    updated_at: new Date().toISOString(),
  },
  {
    id: "img-3",
    product_id: "prod-1",
    url: "https://example.com/natural-front.jpg",
    color: "Natural",
    color_code: "NT",
    is_primary: true,
    sort_order: 3,
    created_at: new Date().toISOString(),
    updated_at: new Date().toISOString(),
  },
  {
    id: "img-4",
    product_id: "prod-1",
    url: "https://example.com/black-front.jpg",
    color: "Black",
    color_code: "BL",
    is_primary: true,
    sort_order: 4,
    created_at: new Date().toISOString(),
    updated_at: new Date().toISOString(),
  },
  {
    id: "img-5",
    product_id: "prod-1",
    url: "https://example.com/dimensions-diagram.jpg",
    color: null,
    color_code: null,
    is_primary: false,
    sort_order: 5,
    created_at: new Date().toISOString(),
    updated_at: new Date().toISOString(),
  },
];

test("getImagesForColor filters correctly by color name and code", () => {
  const walnutImgs = getImagesForColor(mockImages, "Walnut", false);
  assert.equal(walnutImgs.length, 2);
  assert.equal(walnutImgs[0].id, "img-1");
  assert.equal(walnutImgs[1].id, "img-2");

  const naturalImgs = getImagesForColor(mockImages, "NT", false);
  assert.equal(naturalImgs.length, 1);
  assert.equal(naturalImgs[0].id, "img-3");

  const blackImgs = getImagesForColor(mockImages, "Black", false);
  assert.equal(blackImgs.length, 1);
  assert.equal(blackImgs[0].id, "img-4");
});

test("getPrimaryImage returns the primary image for a specific color", () => {
  const primaryNatural = getPrimaryImage(mockImages, "Natural");
  assert.equal(primaryNatural?.id, "img-3");

  const primaryWalnut = getPrimaryImage(mockImages, "WL");
  assert.equal(primaryWalnut?.id, "img-1");

  const primaryBlack = getPrimaryImage(mockImages, "BL");
  assert.equal(primaryBlack?.id, "img-4");
});

test("groupImagesByColor groups images correctly into color buckets", () => {
  const grouped = groupImagesByColor(mockImages);
  assert.equal(grouped.Walnut.length, 2);
  assert.equal(grouped.Natural.length, 1);
  assert.equal(grouped.Black.length, 1);
  assert.equal(grouped.General.length, 1);
});

test("thumbUrl points our product photos at their small copy and leaves others alone", () => {
  const base = "https://x.supabase.co/storage/v1/object/public/product-images";
  assert.equal(thumbUrl(`${base}/products/p1/asset-1-ab.png`), `${base}/products/p1/asset-1-ab.thumb.webp`);
  assert.equal(thumbPath("products/p1/asset-1-ab.webp"), "products/p1/asset-1-ab.thumb.webp");
  assert.equal(viewPath("products/p1/asset-1-ab.png"), "products/p1/asset-1-ab.view.webp");
  assert.equal(viewUrl(`${base}/products/p1/asset-1-ab.png`), `${base}/products/p1/asset-1-ab.view.webp`);
  assert.equal(viewUrl(`${base}/hampers/h1/a.png`), `${base}/hampers/h1/a.png`);
  assert.equal(thumbUrl(`${base}/hampers/h1/a.png`), `${base}/hampers/h1/a.png`);
  assert.equal(thumbUrl("https://example.com/walnut.jpg"), "https://example.com/walnut.jpg");
  assert.equal(thumbUrl("blob:http://localhost/123"), "blob:http://localhost/123");
});
