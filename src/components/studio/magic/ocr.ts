import type { LineBox } from "@/lib/magic-layers/text-style";

export type OcrLine = { text: string; box: LineBox; rowHeight: number };

/**
 * Lines of text OCR is confident about. Tesseract (and its English data) is
 * fetched from its CDN on first use and cached by the browser.
 *
 * Low-confidence lines are left alone: on photos they're mostly texture read
 * as letters, and they stay part of the background where they're harmless.
 */
export async function readLines(image: HTMLCanvasElement, onProgress: (p: number) => void): Promise<OcrLine[]> {
  const { createWorker, OEM } = await import("tesseract.js");
  const worker = await createWorker("eng", OEM.LSTM_ONLY, {
    logger: (m) => m.status === "recognizing text" && onProgress(m.progress),
  });
  try {
    const { data } = await worker.recognize(image, {}, { blocks: true });
    return (data.blocks ?? [])
      .flatMap((b) => b.paragraphs.flatMap((p) => p.lines))
      .filter((l) => l.confidence >= 70 && l.text.trim().length >= 2)
      .map((l) => ({
        text: l.text.trim(),
        box: l.bbox,
        rowHeight: l.rowAttributes?.rowHeight || l.bbox.y1 - l.bbox.y0,
      }));
  } finally {
    await worker.terminate();
  }
}
