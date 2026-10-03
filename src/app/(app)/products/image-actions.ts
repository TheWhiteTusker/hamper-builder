"use server";

import { revalidatePath } from "next/cache";
import { createClient } from "@/lib/supabase/server";
import { describeError } from "@/lib/forms";
import { colorTag } from "@/lib/product-code";
import { thumbPath } from "@/lib/product-images";
import type { ProductImage } from "@/lib/types";
import { signedImageUpload } from "@/lib/signed-upload";
import type { UploadTicket } from "@/components/studio/editor";
import type { ImageActionResult } from "./image-types";

/** Step 1 of a product photo upload: signed URLs the browser sends the photo and its thumbnail to. */
export async function prepareProductImageUpload(
  productId: string,
  ext: string,
): Promise<UploadTicket & { thumbToken?: string }> {
  if (!productId) return { error: "Product ID is missing." };
  const ticket = await signedImageUpload(`products/${productId}`, ext);
  if (!ticket.path) return ticket;
  // No thumbnail ticket is fine: lists fall back to the full photo.
  const supabase = await createClient();
  const { data } = await supabase.storage.from("product-images").createSignedUploadUrl(thumbPath(ticket.path));
  return { ...ticket, thumbToken: data?.token };
}

/** Step 2: record a photo the browser uploaded to `storagePath`. */
export async function saveUploadedProductImage(input: {
  productId: string;
  storagePath: string;
  color?: string | null;
  isPrimary?: boolean;
  caption?: string | null;
}): Promise<ImageActionResult> {
  try {
    const supabase = await createClient();
    const {
      data: { user },
    } = await supabase.auth.getUser();

    if (!user) return { error: "You must be signed in to upload images." };

    const productId = input.productId?.trim();
    const storagePath = input.storagePath;
    const rawColor = input.color?.trim() || null;
    const isPrimary = input.isPrimary === true;
    const caption = input.caption?.trim() || null;

    if (!productId) {
      return { error: "Product ID is missing." };
    }
    if (!storagePath?.startsWith(`products/${productId}/`)) {
      return { error: "Invalid upload path." };
    }

    // Get public URL
    const {
      data: { publicUrl },
    } = supabase.storage.from("product-images").getPublicUrl(storagePath);

    // If marked primary, clear existing primary flags
    if (isPrimary) {
      await supabase
        .from("product_images")
        .update({ is_primary: false })
        .eq("product_id", productId);
    } else {
      // Check if this is the first image for the product
      const { count } = await supabase
        .from("product_images")
        .select("id", { count: "exact", head: true })
        .eq("product_id", productId)
        .is("deleted_at", null);
      if (count === 0) {
        // Automatically make first image primary
        await supabase
          .from("products")
          .update({ image_url: publicUrl })
          .eq("id", productId);
      }
    }

    // Insert record in product_images
    const { data: inserted, error: dbError } = await supabase
      .from("product_images")
      .insert({
        product_id: productId,
        url: publicUrl,
        storage_path: storagePath,
        ...colorTag(rawColor),
        is_primary: isPrimary,
        caption,
      })
      .select("*")
      .single<ProductImage>();

    if (dbError) {
      return { error: `Database save failed: ${describeError(dbError)}` };
    }

    if (isPrimary) {
      await supabase
        .from("products")
        .update({ image_url: publicUrl })
        .eq("id", productId);
    }

    revalidatePath("/products");
    revalidatePath("/hampers");
    revalidatePath("/cost-calculator", "layout");

    return { ok: true, image: inserted };
  } catch (err: unknown) {
    return { error: describeError(err) };
  }
}

/**
 * Add an external image URL for a product
 */
export async function addExternalProductImage(
  productId: string,
  url: string,
  color?: string | null,
  isPrimary: boolean = false,
  caption?: string | null,
): Promise<ImageActionResult> {
  try {
    const supabase = await createClient();
    const cleanUrl = url.trim();
    if (!cleanUrl) return { error: "Image URL is required." };
    if (!productId) return { error: "Product ID is missing." };

    if (isPrimary) {
      await supabase
        .from("product_images")
        .update({ is_primary: false })
        .eq("product_id", productId);
    }

    const { data: inserted, error: dbError } = await supabase
      .from("product_images")
      .insert({
        product_id: productId,
        url: cleanUrl,
        storage_path: null,
        ...colorTag(color),
        is_primary: isPrimary,
        caption: caption || null,
      })
      .select("*")
      .single<ProductImage>();

    if (dbError) return { error: describeError(dbError) };

    if (isPrimary) {
      await supabase
        .from("products")
        .update({ image_url: cleanUrl })
        .eq("id", productId);
    }

    revalidatePath("/products");
    revalidatePath("/hampers");
    return { ok: true, image: inserted };
  } catch (err: unknown) {
    return { error: describeError(err) };
  }
}
