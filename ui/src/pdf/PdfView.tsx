// PDF viewer (pdf.js): continuous scroll, pages rendered as they come into view,
// selectable text, SyncTeX highlight (forward) and click-to-source (inverse).

import { useEffect, useLayoutEffect, useRef, useState } from "react";
import { convertFileSrc } from "@tauri-apps/api/core";
import * as pdfjs from "pdfjs-dist";
import type { PDFDocumentProxy, PDFPageProxy } from "pdfjs-dist";
import workerUrl from "pdfjs-dist/build/pdf.worker.min.mjs?url";
import "./textLayer.css";
import { ExternalLink, Minus, Plus, RotateCw, MoveHorizontal } from "lucide-react";
import { commands } from "../bindings";
import { useApp } from "../store";
import { inverseSearch, report } from "../actions";
import { basename } from "../lib/paths";

pdfjs.GlobalWorkerOptions.workerSrc = workerUrl;

const GAP = 16;
const ZOOMS = [0.5, 0.75, 1, 1.25, 1.5, 1.75, 2, 2.5, 3];

type Size = { w: number; h: number };

async function load(path: string): Promise<PDFDocumentProxy> {
  // Fetch the bytes ourselves so a rebuilt PDF is never served from cache.
  const res = await fetch(convertFileSrc(path), { cache: "no-store" });
  if (!res.ok) throw new Error(`Could not read ${basename(path)} (${res.status})`);
  const data = new Uint8Array(await res.arrayBuffer());
  return pdfjs.getDocument({ data }).promise;
}

export function PdfView({ path, visible }: { path: string; visible: boolean }) {
  const version = useApp((s) => s.pdfVersion[path] ?? 0);
  const highlight = useApp((s) => (s.pdfHighlight?.path === path ? s.pdfHighlight : null));
  const [doc, setDoc] = useState<PDFDocumentProxy | null>(null);
  const [sizes, setSizes] = useState<Size[]>([]);
  const [zoom, setZoom] = useState(1);
  const [page, setPage] = useState(1);
  const [error, setError] = useState<string | null>(null);
  const scroller = useRef<HTMLDivElement>(null);
  const keepScroll = useRef<number | null>(null);

  const current = useRef<PDFDocumentProxy | null>(null);
  useEffect(() => () => void current.current?.loadingTask.destroy(), []);

  useEffect(() => {
    let cancelled = false;
    // Keep the reading position across reloads (rebuilds).
    const el = scroller.current;
    if (el && el.scrollHeight > 0) keepScroll.current = el.scrollTop / el.scrollHeight;
    load(path)
      .then(async (d) => {
        const pages = await Promise.all(Array.from({ length: d.numPages }, (_, i) => d.getPage(i + 1)));
        if (cancelled) return void d.loadingTask.destroy();
        setSizes(
          pages.map((p) => {
            const v = p.getViewport({ scale: 1 });
            return { w: v.width, h: v.height };
          }),
        );
        // The old document goes only once the new one has replaced it on screen.
        const old = current.current;
        current.current = d;
        setDoc(d);
        setError(null);
        if (old) setTimeout(() => void old.loadingTask.destroy(), 1000);
      })
      .catch((e) => !cancelled && setError(String(e)));
    return () => {
      cancelled = true;
    };
  }, [path, version]);

  // First load: fit the page to the pane when it would not fit at 100%.
  const fitted = useRef(false);
  useLayoutEffect(() => {
    const el = scroller.current;
    if (fitted.current || !el || !sizes[0] || el.clientWidth === 0) return;
    fitted.current = true;
    const fit = (el.clientWidth - 2 * GAP) / sizes[0].w;
    if (fit < 1) setZoom(Math.max(0.5, fit));
  }, [sizes, visible]);

  useLayoutEffect(() => {
    const el = scroller.current;
    if (el && keepScroll.current !== null && sizes.length) {
      el.scrollTop = keepScroll.current * el.scrollHeight;
      keepScroll.current = null;
    }
  }, [sizes]);

  const pageTop = (n: number) => GAP + sizes.slice(0, n - 1).reduce((a, s) => a + s.h * zoom + GAP, 0);

  // Forward search: scroll the highlighted box to the upper third of the view.
  useEffect(() => {
    const el = scroller.current;
    if (!highlight || !el || !sizes.length) return;
    const y = pageTop(highlight.rect.page) + (highlight.rect.y ?? 0) * zoom;
    el.scrollTo({ top: Math.max(0, y - el.clientHeight / 3), behavior: "smooth" });
  }, [highlight?.nonce, sizes.length]);

  function onScroll() {
    const el = scroller.current;
    if (!el) return;
    const mid = el.scrollTop + el.clientHeight / 2;
    let n = 1;
    while (n < sizes.length && pageTop(n + 1) <= mid) n++;
    setPage(n);
  }

  function changeZoom(next: number) {
    const el = scroller.current;
    const ratio = el && el.scrollHeight ? el.scrollTop / el.scrollHeight : 0;
    setZoom(Math.min(3, Math.max(0.5, next)));
    requestAnimationFrame(() => el && (el.scrollTop = ratio * el.scrollHeight));
  }

  const step = (dir: 1 | -1) => {
    const i = ZOOMS.findIndex((z) => z >= zoom - 1e-6);
    changeZoom(ZOOMS[Math.min(ZOOMS.length - 1, Math.max(0, (i === -1 ? ZOOMS.length : i) + dir))]);
  };

  const fitWidth = () => {
    const el = scroller.current;
    if (el && sizes[0]) changeZoom((el.clientWidth - 2 * GAP) / sizes[0].w);
  };

  return (
    <div className="pdf-view" style={{ display: visible ? undefined : "none" }}>
      <div className="pane-toolbar">
        <span className="pane-toolbar-title ellipsis" title={path}>
          {basename(path)}
        </span>
        {sizes.length > 0 && (
          <span className="muted nowrap">
            Page {page} / {sizes.length}
          </span>
        )}
        <span className="spacer" />
        <button className="icon-btn" title="Zoom out" onClick={() => step(-1)}>
          <Minus size={14} />
        </button>
        <button className="zoom-label" title="Reset to 100%" onClick={() => changeZoom(1)}>
          {Math.round(zoom * 100)}%
        </button>
        <button className="icon-btn" title="Zoom in" onClick={() => step(1)}>
          <Plus size={14} />
        </button>
        <button className="icon-btn" title="Fit width" onClick={fitWidth}>
          <MoveHorizontal size={14} />
        </button>
        <button className="icon-btn" title="Reload" onClick={() => useApp.setState((s) => ({ pdfVersion: { ...s.pdfVersion, [path]: (s.pdfVersion[path] ?? 0) + 1 } }))}>
          <RotateCw size={14} />
        </button>
        <button className="icon-btn" title="Open in default app" onClick={() => commands.openExternal(path).catch(report)}>
          <ExternalLink size={14} />
        </button>
      </div>
      <div className="pdf-scroll" ref={scroller} onScroll={onScroll}>
        {error && <div className="empty-state">{error}</div>}
        {doc &&
          sizes.map((size, i) => (
            <PdfPage
              key={i}
              doc={doc}
              number={i + 1}
              size={size}
              zoom={zoom}
              root={scroller}
              highlight={highlight?.rect.page === i + 1 ? toRect(highlight.rect) : null}
              onPick={(x, y) => void inverseSearch(path, i + 1, x, y)}
            />
          ))}
      </div>
    </div>
  );
}

type Rect = { x: number; y: number; width: number; height: number };

// Rust floats arrive as `number | null` (NaN is sent as null).
const toRect = (r: { x: number | null; y: number | null; width: number | null; height: number | null }): Rect => ({
  x: r.x ?? 0,
  y: r.y ?? 0,
  width: r.width ?? 0,
  height: r.height ?? 0,
});

function PdfPage(props: {
  doc: PDFDocumentProxy;
  number: number;
  size: Size;
  zoom: number;
  root: React.RefObject<HTMLDivElement | null>;
  highlight: Rect | null;
  onPick: (x: number, y: number) => void;
}) {
  const { doc, number, size, zoom, root, highlight, onPick } = props;
  const box = useRef<HTMLDivElement>(null);
  const canvas = useRef<HTMLCanvasElement>(null);
  const text = useRef<HTMLDivElement>(null);
  const [near, setNear] = useState(false);
  const down = useRef<{ x: number; y: number } | null>(null);

  useEffect(() => {
    const io = new IntersectionObserver(([e]) => setNear(e.isIntersecting), {
      root: root.current,
      rootMargin: "1200px 0px",
    });
    io.observe(box.current!);
    return () => io.disconnect();
  }, [root]);

  useEffect(() => {
    if (!near) return;
    let task: ReturnType<PDFPageProxy["render"]> | null = null;
    let cancelled = false;
    void doc.getPage(number).catch(() => null).then(async (p) => {
      if (!p) return;
      if (cancelled) return;
      const viewport = p.getViewport({ scale: zoom });
      const dpr = window.devicePixelRatio || 1;
      const c = canvas.current!;
      c.width = Math.floor(viewport.width * dpr);
      c.height = Math.floor(viewport.height * dpr);
      task = p.render({ canvas: c, viewport, transform: dpr === 1 ? undefined : [dpr, 0, 0, dpr, 0, 0] });
      await task.promise.catch(() => {});
      if (cancelled || !text.current) return;
      text.current.replaceChildren();
      await new pdfjs.TextLayer({ textContentSource: p.streamTextContent(), container: text.current, viewport })
        .render()
        .catch(() => {});
    });
    return () => {
      cancelled = true;
      task?.cancel();
    };
  }, [near, zoom, doc, number]);

  const style = {
    width: size.w * zoom,
    height: size.h * zoom,
    "--scale-factor": zoom,
    "--total-scale-factor": zoom,
    "--user-unit": 1,
  } as React.CSSProperties;

  return (
    <div
      ref={box}
      className="pdf-page"
      style={style}
      onMouseDown={(e) => (down.current = { x: e.clientX, y: e.clientY })}
      onMouseUp={(e) => {
        const start = down.current;
        down.current = null;
        // A plain click (not a drag or text selection) jumps to the source.
        if (!start || Math.hypot(e.clientX - start.x, e.clientY - start.y) > 4) return;
        if (window.getSelection()?.toString()) return;
        const r = box.current!.getBoundingClientRect();
        onPick((e.clientX - r.left) / zoom, (e.clientY - r.top) / zoom);
      }}
    >
      <canvas ref={canvas} style={{ width: size.w * zoom, height: size.h * zoom }} />
      <div ref={text} className="textLayer" />
      {highlight && (
        <div
          className="synctex-highlight"
          style={{
            left: highlight.x * zoom - 3,
            top: highlight.y * zoom - 2,
            width: highlight.width * zoom + 6,
            height: highlight.height * zoom + 4,
          }}
        />
      )}
      <span className="pdf-page-label">{number}</span>
    </div>
  );
}
