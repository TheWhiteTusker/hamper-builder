import { useState } from "react";
import type { Editor, UploadTicket } from "../editor";
import { runMagicLayers, type MagicStep } from "../magic/run-magic-layers";
import type { CanvasDoc } from "./use-canvas-doc";
import type { Toast } from "./use-stage-save";

/** Per computer: the AI models are heavy, so each machine decides for itself. */
const KEY = "studio.magicLayers";

/** Machines with little memory or few cores start with Magic layers off. */
export function looksWeak() {
  const n = navigator as Navigator & { deviceMemory?: number };
  return (n.deviceMemory ?? 8) < 4 || (n.hardwareConcurrency ?? 8) < 4;
}

function readSetting() {
  try {
    const saved = localStorage.getItem(KEY);
    if (saved) return saved === "on";
  } catch {
    // Storage blocked: fall back to the device default.
  }
  return !looksWeak();
}

/** Magic layers: split an image layer into background, objects and editable text. */
export function useMagicLayers(doc: CanvasDoc, onUpload: (ext: string) => Promise<UploadTicket>, setToast: (t: Toast) => void) {
  const [magicEnabled, setEnabled] = useState(readSetting);
  const [magicStep, setMagicStep] = useState<MagicStep | null>(null);

  const setMagicEnabled: Editor["setMagicEnabled"] = (on) => {
    setEnabled(on);
    try {
      localStorage.setItem(KEY, on ? "on" : "off");
    } catch {
      // Not remembered in this browser; it still applies until the page closes.
    }
  };

  const magicSplit: Editor["magicSplit"] = async (id) => {
    const layer = doc.canvas.layers.find((l) => l.id === id);
    if (!magicEnabled || magicStep || layer?.kind !== "image") return;
    let step = "Starting";
    setMagicStep({ label: step });
    try {
      const layers = await runMagicLayers(layer, onUpload, (s) => {
        step = s.label;
        setMagicStep(s);
      });
      if (!layers.length) {
        setToast({ kind: "error", text: "No text or separate objects were found in that image." });
        return;
      }
      // Swap the flat image for its pieces in one step, so a single Undo brings it back.
      doc.change((c) => {
        const at = c.layers.findIndex((l) => l.id === id);
        if (at < 0) return c;
        return { ...c, layers: [...c.layers.slice(0, at), ...layers, ...c.layers.slice(at + 1)] };
      });
      doc.selectMany(layers.map((l) => l.id));
      setToast({ kind: "ok", text: `Split into ${layers.length} layers. Double-click text to edit it; Undo puts the original back.` });
    } catch (err) {
      const why = err instanceof Error ? err.message : String(err ?? "unknown error");
      setToast({ kind: "error", text: `Magic layers failed at "${step}": ${why}. Try again; downloads often succeed on a retry.` });
    } finally {
      setMagicStep(null);
    }
  };

  return { magicEnabled, setMagicEnabled, magicStep, magicSplit };
}
