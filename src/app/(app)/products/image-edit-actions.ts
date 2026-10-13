"use server";

// No revalidatePath here: the photo manager patches its own list, and
// markOtherPagesStale() refreshes other pages on the next navigation.

import { createClient } from "@/lib/supabase/server";
import { describeError } from "@/lib/forms";
import { colorTag } from "@/lib/product-code";
import type { ProductImage } from "@/lib/types";
import type { ImageActionResult } from "./image-types";

/**
 * Set an image as the primary cover thumbnail for a product
 */
export async function setPrimaryProductImage(
  imageId: string,
  productId: string,
): Promise<ImageActionResult> {
  try {
    const supabase = await createClient();

    // 1. Clear existing primary
    await supabase
      .from("product_images")
      .update({ is_primary: false })
      .eq("product_id", productId);

    // 2. Set this image as primary
    const { data: updated, error } = await supabase
      .from("product_images")
      .update({ is_primary: true })
      .eq("id", imageId)
      .select("*")
      .single<ProductImage>();

    if (error) return { error: describeError(error) };

    // 3. Update products.image_url
    if (updated?.url) {
      await supabase
        .from("products")
        .update({ image_url: updated.url })
        .eq("id", productId);
    }

    return { ok: true, image: updated };
  } catch (err: unknown) {
    return { error: describeError(err) };
  }
}

/**
 * Update the color finish tag of an image
 */
export async function updateProductImageColor(
  imageId: string,
  rawColor: string | null,
): Promise<ImageActionResult> {
  try {
    const supabase = await createClient();
    const { data: updated, error } = await supabase
      .from("product_images")
      .update(colorTag(rawColor))
      .eq("id", imageId)
      .select("*")
      .single<ProductImage>();

    if (error) return { error: describeError(error) };

    return { ok: true, image: updated };
  } catch (err: unknown) {
    return { error: describeError(err) };
  }
}

/**
 * Delete a product image from database and Supabase storage
 */
export async function deleteProductImage(
  imageId: string,
  productId: string,
): Promise<ImageActionResult> {
  try {
    const supabase = await createClient();

    // Fetch image details
    const { data: img } = await supabase
      .from("product_images")
      .select("*")
      .eq("id", imageId)
      .maybeSingle<ProductImage>();

    if (!img) return { error: "Image not found." };

    // Soft delete: move to bin (preserve in storage until permanently deleted)
    const { error: delError } = await supabase
      .from("product_images")
      .update({
        deleted_at: new Date().toISOString(),
        is_primary: false,
      })
      .eq("id", imageId);

    if (delError) return { error: describeError(delError) };

    // If this was primary, update products.image_url to next available active image or null
    let promoted: ProductImage | undefined;
    if (img.is_primary) {
      const { data: nextImg } = await supabase
        .from("product_images")
        .select("*")
        .eq("product_id", productId)
        .is("deleted_at", null)
        .order("sort_order")
        .order("created_at")
        .limit(1)
        .maybeSingle<ProductImage>();

      if (nextImg) {
        await supabase
          .from("product_images")
          .update({ is_primary: true })
          .eq("id", nextImg.id);
        await supabase
          .from("products")
          .update({ image_url: nextImg.url })
          .eq("id", productId);
        promoted = { ...nextImg, is_primary: true };
      } else {
        await supabase
          .from("products")
          .update({ image_url: null })
          .eq("id", productId);
      }
    }

    // The photo that took over as cover, if any, so the caller can show it.
    return { ok: true, image: promoted };
  } catch (err: unknown) {
    return { error: describeError(err) };
  }
}
