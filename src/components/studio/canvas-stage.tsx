"use client";

import { useMemo, useRef, useState } from "react";
import type Konva from "konva";
import { moveLayers, reorderLayer, type HamperCanvas } from "@/lib/hamper-canvas";
import { DRAG_MIME, type ActionResult, type Editor, type UploadTicket, type PickerProduct } from "./editor";
import { ContextToolbar } from "./context-toolbar";
import { LayersPanel, TransformPanel } from "./layers-panel";
import { Rail, SidePanel, type SideTab } from "./side-panels";
import { cx } from "./studio-ui";
import { BoardOverlays } from "./stage/board-overlays";
import { PageStrip } from "./stage/page-strip";
import { StageCanvas } from "./stage/stage-canvas";
import { StageFooter } from "./stage/stage-footer";
import { StageMenus } from "./stage/stage-menus";
import { StageTopbar } from "./stage/stage-topbar";
import { useArrange } from "./stage/use-arrange";
import { documentColorsOf, useCanvasDoc } from "./stage/use-canvas-doc";
import { useDrawTool, type Tool } from "./stage/use-draw-tool";
import { useFontTick } from "./stage/use-font-tick";
import { useLayerEvents } from "./stage/use-layer-events";
import { useStageImages } from "./stage/use-stage-images";
import { useStageKeys } from "./stage/use-stage-keys";
import { useStageSave } from "./stage/use-stage-save";
import { useStageView } from "./stage/use-stage-view";

export default function CanvasStage({
  title,
  subtitle,
  backHref,
  downloadName,
  initial,
  products,
  hamperProductIds,
  onSave,
  onUpload,
  saveImage = true,
  savedMessage = "Saved.",
  pages,
  pageId,
}: {
  title: string;
  subtitle?: string;
  backHref: string;
  downloadName: string;
  initial: HamperCanvas;
  products: PickerProduct[];
  /** Products offered first in the Products panel ("In this hamper"). */
  hamperProductIds: string[];
  /** Server action; receives `canvas` (JSON) and, with saveImage, `png`. */
  onSave: (formData: FormData) => Promise<ActionResult>;
  /** Server action; receives `file`, returns its public `url`. */
  onUpload: (ext: string) => Promise<UploadTicket>;
  saveImage?: boolean;
  savedMessage?: string;
  /** Sibling pages (presentation slides) shown as a strip under the page. */
  pages?: { id: string; href: string; canvas: HamperCanvas }[];
  pageId?: string;
}) {
  const doc = useCanvasDoc(initial);
  const { canvas, setLayers } = doc;
  const [tab, setTab] = useState<SideTab | null>("products");
  const [spaceDown, setSpaceDown] = useState(false);
  const fontTick = useFontTick(canvas);
  const [activeTool, setActiveTool] = useState<Tool>("select");

  const stageRef = useRef<Konva.Stage>(null);
  const pageRef = useRef<Konva.Group>(null);
  const trRef = useRef<Konva.Transformer>(null);
  const boardRef = useRef<HTMLDivElement>(null);
  const nodes = useRef(new Map<string, Konva.Node>());

  const view = useStageView(boardRef, canvas.width, canvas.height);
  const saver = useStageSave({ doc, view, pageRef, onSave, saveImage, savedMessage, downloadName });
  const images = useStageImages(doc, onUpload, saver.setToast);
  const arrange = useArrange(doc, nodes, pageRef);

  const documentColors = useMemo(() => documentColorsOf(canvas), [canvas]);
  const events = useLayerEvents({
    doc, nodes, pageRef, trRef, boardRef, zoom: view.zoom, spaceDown, selecting: activeTool === "select", fontTick,
  });
  const draw = useDrawTool({ pageRef, setGuides: events.setGuides, add: doc.add, activeTool, setActiveTool });

  const ed: Editor = {
    ...doc,
    documentColors,
    ...arrange,
    move: (ids, m) => setLayers((ls) => moveLayers(ls, ids, m)),
    reorder: (id, to) => setLayers((ls) => reorderLayer(ls, id, to)),
    ...images,
    startSwap: (id) => {
      images.setSwapId(id);
      if (id) setTab("products");
    },
    activeTool,
    setActiveTool,
    snapRightAngles: draw.snapRightAngles,
    setSnapRightAngles: draw.setSnapRightAngles,
  };

  useStageKeys({ doc, view, save: saver.save, move: ed.move, swapId: images.swapId, setSwapId: images.setSwapId, setSpaceDown });

  const resize = (width: number, height: number) => {
    doc.change((c) => ({ ...c, width, height }));
    view.zoomFit();
  };

  return (
    <div className="studio fixed inset-0 z-[60] flex flex-col bg-[var(--st-bg)]">
      <StageTopbar
        title={title}
        subtitle={subtitle}
        backHref={backHref}
        doc={doc}
        saving={saver.saving}
        onResize={resize}
        onDownload={saver.download}
        onSave={saver.save}
      />

      <div className="flex min-h-0 flex-1">
        <Rail tab={tab} onTab={setTab} />
        {tab && <SidePanel tab={tab} onClose={() => setTab(null)} ed={ed} products={products} hamperProductIds={hamperProductIds} />}

        <div className="flex min-w-0 flex-1 flex-col">
          <ContextToolbar ed={ed} />

          {/* Pasteboard */}
          <div
            ref={boardRef}
            className={cx(
              "relative min-h-0 flex-1 overflow-hidden",
              spaceDown ? "cursor-grab" : activeTool !== "select" ? "cursor-crosshair" : undefined,
            )}
            onPointerDown={(e) => (spaceDown || e.button === 1) && view.startPan(e)}
            onDragOver={(e) => {
              if (e.dataTransfer.types.includes(DRAG_MIME)) {
                e.preventDefault();
                e.dataTransfer.dropEffect = "copy";
              }
            }}
            onDrop={(e) => {
              const raw = e.dataTransfer.getData(DRAG_MIME);
              if (!raw) return;
              e.preventDefault();
              const { product, url, name } = JSON.parse(raw) as { product: PickerProduct | null; url: string; name?: string };
              stageRef.current?.setPointersPositions(e.nativeEvent);
              const at = pageRef.current?.getRelativePointerPosition() ?? undefined;
              images.setSwapId(null);
              if (product) images.placeImage(product, url, at);
              else images.addImage(url, name ?? "Image", at);
            }}
          >
            {view.board.w > 0 && (
              <StageCanvas
                doc={doc}
                view={view}
                events={events}
                draw={draw}
                stageRef={stageRef}
                pageRef={pageRef}
                trRef={trRef}
                spaceDown={spaceDown}
                fontTick={fontTick}
              />
            )}
            <BoardOverlays doc={doc} view={view} marquee={events.marquee} swapping={!!images.swapId} toast={saver.toast} />
          </div>

          {pages && pages.length > 0 && (
            <PageStrip pages={pages} pageId={pageId} current={doc.savedCanvas} dirty={doc.dirty} onGo={saver.goTo} />
          )}

          <StageFooter
            view={view}
            width={canvas.width}
            height={canvas.height}
            layerCount={canvas.layers.length}
            onResize={resize}
          />
        </div>

        <aside className="hidden w-[280px] shrink-0 flex-col border-l border-[var(--st-line)] bg-[var(--st-panel)] lg:flex">
          <LayersPanel ed={ed} />
          <TransformPanel ed={ed} />
        </aside>
      </div>

      <StageMenus doc={doc} ed={ed} zoomFit={view.zoomFit} recenter={view.recenter} />
    </div>
  );
}
