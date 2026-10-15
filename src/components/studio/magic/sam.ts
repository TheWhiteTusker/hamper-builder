import type { SamModel } from "@huggingface/transformers";
import type { LowMask } from "@/lib/magic-layers/masks";

/** SlimSAM: Segment Anything distilled to about 14 MB (Apache-2.0). */
const MODEL = "Xenova/slimsam-77-uniform";
/** Prompt points per side; 16×16 finds most separate things on a page. */
const GRID = 16;
/** Points decoded per run: each returns 3×256×256 floats, so keep batches modest. */
const BATCH = 16;

/**
 * transformers.js comes from its CDN at run time instead of the bundle:
 * bundling it ships onnxruntime's 27 MB wasm as a static asset, over Workers'
 * 25 MiB per-file limit. Keep the version in step with the devDependency,
 * which is only there for the types.
 */
const TRANSFORMERS = "https://cdn.jsdelivr.net/npm/@huggingface/transformers@4.3.1";

/**
 * The model and the ONNX runtime are large CDN files that sometimes fail on the
 * first try (a cold CDN cache answers 503 without CORS, seen as "Failed to fetch").
 * Retry a few times, and name the file if it still fails.
 */
async function retryingFetch(input: string | URL, init?: RequestInit): Promise<Response> {
  for (let attempt = 1; ; attempt++) {
    try {
      const res = await fetch(input, init);
      if (res.status < 500 || attempt === 3) return res;
    } catch (err) {
      if (attempt === 3) throw new Error(`${err instanceof Error ? err.message : err} (${String(input).split("?")[0]})`);
    }
    await new Promise((r) => setTimeout(r, attempt * 1500));
  }
}

async function loadModel() {
  const tf: typeof import("@huggingface/transformers") = await import(
    /* webpackIgnore: true */ /* turbopackIgnore: true */ TRANSFORMERS
  );
  tf.env.fetch = retryingFetch;
  const open = (device: "webgpu" | "wasm") =>
    Promise.all([tf.SamModel.from_pretrained(MODEL, { device }) as Promise<SamModel>, tf.AutoProcessor.from_pretrained(MODEL)]);
  const [model, processor] =
    "gpu" in navigator ? await open("webgpu").catch(() => open("wasm")) : await open("wasm");
  return { tf, model, processor };
}

let loading: ReturnType<typeof loadModel> | null = null;

/** Downloaded once per page (then served from the browser cache); a failed load can be retried. */
const load = () =>
  (loading ??= loadModel().catch((err) => {
    loading = null;
    throw err;
  }));

/**
 * Every object SAM can find from a grid of prompt points, one candidate mask
 * per point, as low-res logits. `scale` is mask cells per source pixel.
 */
export async function findObjects(
  image: HTMLCanvasElement,
  onProgress: (p: number) => void,
): Promise<{ masks: LowMask[]; scale: number }> {
  const { tf, model, processor } = await load();
  const inputs = await processor(tf.RawImage.fromCanvas(image));
  const embeddings = await model.get_image_embeddings(inputs);

  // The model sees the image resized so its long side is 1024, padded to a square;
  // masks come back at a quarter of that, so only the top-left rw/4 × rh/4 is image.
  const [rh, rw] = inputs.reshaped_input_sizes[0] as [number, number];
  const w = Math.ceil(rw / 4);
  const h = Math.ceil(rh / 4);
  const points: number[][] = [];
  for (let j = 0; j < GRID; j++) {
    for (let i = 0; i < GRID; i++) points.push([((i + 0.5) / GRID) * rw, ((j + 0.5) / GRID) * rh]);
  }

  const masks: LowMask[] = [];
  for (let s = 0; s < points.length; s += BATCH) {
    const batch = points.slice(s, s + BATCH);
    const input_points = new tf.Tensor("float32", Float32Array.from(batch.flat()), [1, batch.length, 1, 2]);
    const { iou_scores, pred_masks } = await model({ ...embeddings, input_points });
    const scores = iou_scores.data as Float32Array;
    const logits = pred_masks.data as Float32Array;
    const [, , choices, mh, mw] = pred_masks.dims as number[];
    for (let b = 0; b < batch.length; b++) {
      // SAM offers three readings per point (part, object, whole); keep the one it rates best.
      let best = 0;
      for (let c = 1; c < choices; c++) if (scores[b * choices + c] > scores[b * choices + best]) best = c;
      const base = (b * choices + best) * mh * mw;
      const out = new Float32Array(w * h);
      for (let y = 0; y < h; y++) out.set(logits.subarray(base + y * mw, base + y * mw + w), y * w);
      masks.push({ logits: out, w, h, score: scores[b * choices + best] });
    }
    onProgress((s + batch.length) / points.length);
  }
  return { masks, scale: rw / 4 / image.width };
}
