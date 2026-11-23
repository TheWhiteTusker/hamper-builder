import { useEffect, useState, type RefObject } from "react";
import { useRouter } from "next/navigation";
import { useUnsavedWarning } from "@/components/unsaved-warning";
import type Konva from "konva";
import type { ActionResult } from "../editor";
import type { CanvasDoc } from "./use-canvas-doc";
import type { StageView } from "./use-stage-view";

export type Toast = { kind: "ok" | "error"; text: string } | null;

/** Save, download and page switching, plus the toast they report through. */
export function useStageSave({
  doc,
  view,
  pageRef,
  onSave,
  saveImage,
  savedMessage,
  downloadName,
}: {
  doc: CanvasDoc;
  view: StageView;
  pageRef: RefObject<Konva.Group | null>;
  onSave: (formData: FormData) => Promise<ActionResult>;
  saveImage: boolean;
  savedMessage: string;
  downloadName: string;
}) {
  const router = useRouter();
  const [saving, setSaving] = useState(false);
  const [toast, setToast] = useState<Toast>(null);
  const { canvas, dirty } = doc;

  async function renderPng(): Promise<Blob> {
    await document.fonts.ready;
    const page = pageRef.current;
    if (!page) throw new Error("not ready");
    const { ox, oy, zoom } = view;
    const out = page.toCanvas({ x: ox, y: oy, width: canvas.width * zoom, height: canvas.height * zoom, pixelRatio: 1 / zoom });
    return new Promise((resolve, reject) => {
      try {
        out.toBlob((b) => (b ? resolve(b) : reject(new Error("empty"))), "image/png");
      } catch (e) {
        reject(e);
      }
    });
  }

  /** Saves the design (and its PNG, when asked for). Resolves false if it didn't save. */
  async function save(): Promise<boolean> {
    if (saving) return false;
    setSaving(true);
    doc.setEditingId(null);
    try {
      const fd = new FormData();
      fd.set("canvas", JSON.stringify(canvas));
      if (saveImage) {
        try {
          fd.set("png", new File([await renderPng()], "image.png", { type: "image/png" }));
        } catch {
          setToast({ kind: "error", text: "Export blocked by an image from another website. Upload that image to the product instead." });
          return false;
        }
      }
      const res = await onSave(fd);
      if (res.error) {
        setToast({ kind: "error", text: res.error });
        return false;
      }
      doc.setSavedCanvas(canvas);
      setToast({ kind: "ok", text: savedMessage });
      return true;
    } finally {
      setSaving(false);
    }
  }

  /** Switch to another page of the deck, saving this one first. */
  async function goTo(href: string) {
    if (dirty && !(await save())) return;
    router.push(href);
  }

  async function download() {
    try {
      const url = URL.createObjectURL(await renderPng());
      const a = document.createElement("a");
      a.href = url;
      a.download = `${downloadName}.png`;
      a.click();
      URL.revokeObjectURL(url);
    } catch {
      setToast({ kind: "error", text: "Could not export the image." });
    }
  }

  useEffect(() => {
    if (!toast) return;
    const t = setTimeout(() => setToast(null), toast.kind === "error" ? 7000 : 3500);
    return () => clearTimeout(t);
  }, [toast]);

  useUnsavedWarning(dirty);

  // The editor is full-screen; stop the page behind it scrolling.
  useEffect(() => {
    const prev = document.body.style.overflow;
    document.body.style.overflow = "hidden";
    return () => {
      document.body.style.overflow = prev;
    };
  }, []);

  return { saving, toast, setToast, save, goTo, download };
}
