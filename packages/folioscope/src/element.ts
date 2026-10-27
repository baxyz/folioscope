// SPDX-License-Identifier: AGPL-3.0-or-later
import {
  type Action,
  type FolioIndex,
  type FolioFrame,
  type Layer,
  type OverlaySettings,
  type Point,
  type Rect,
  SUPPORTED_FOLIO,
  SUPPORTED_IR,
} from "./folio.ts";
import { CONTROLS_STYLE, Controls } from "./controls.ts";
import { contains, fitContain } from "helpers4/geometry";
import { thumbHashToDataURL } from "thumbhash";
import {
  decoded,
  type HoldKind,
  play,
  playOverlay,
  playSmart,
  renderFrame,
  scrollFrame,
} from "./lite.ts";
import { type OpenOverlay, Runtime, type Step } from "./runtime.ts";

export const TAG_NAME = "folioscope-player";

/** A raw design file, played by the full player when no `.folio` sits next to it. */
const RAW = /\.(fig|sketch|xd|penpot)$/i;

const TYPES: Record<string, string> = {
  ".json": "application/json",
  ".webp": "image/webp",
  ".avif": "image/avif",
};

// Keeps the module importable during SSR (VitePress build, the Vite plugin), where HTMLElement is undefined.
const BaseElement: typeof HTMLElement =
  globalThis.HTMLElement ?? (class {} as unknown as typeof HTMLElement);

const STYLE = `
:host { display: block; position: relative; contain: content; }
:host([hidden]) { display: none; }
.viewport { position: relative; width: 100%; overflow: hidden; aspect-ratio: var(--w) / var(--h); }
.stage { position: absolute; top: 0; left: 0; width: calc(var(--w) * 1px); height: calc(var(--h) * 1px); transform-origin: 0 0; transform: translate(var(--x, 0px), var(--y, 0px)) scale(var(--scale, 1)); }
:host(:fullscreen) { display: flex; flex-direction: column; background: #111; }
:host(:fullscreen) .viewport { flex: 1; min-height: 0; aspect-ratio: auto; }
.frame { position: absolute; inset: 0; overflow: hidden; background: transparent; }
.frame img, .poster { position: absolute; display: block; user-select: none; pointer-events: none; }
.poster { inset: 0; width: 100%; height: 100%; object-fit: contain; }
.overlay, .backdrop, .sheet { position: absolute; inset: 0; }
.scroll { position: absolute; inset: 0; overscroll-behavior: contain; scrollbar-width: none; }
.scroll::-webkit-scrollbar { display: none; }
.content { position: relative; }
.sheet { pointer-events: none; }
.sheet > .frame { pointer-events: auto; }
.component { position: absolute; }
.smart { position: absolute; inset: 0; overflow: hidden; }
.smart img { position: absolute; display: block; transform-origin: 0 0; pointer-events: none; }
.hotspot { position: absolute; margin: 0; padding: 0; border: 0; background: none; cursor: pointer; }
.hotspot:focus-visible { outline: 2px solid #2563eb; outline-offset: 2px; }
:host([hotspots="always"]) .hotspot { background: rgb(37 99 235 / 0.15); box-shadow: inset 0 0 0 2px rgb(37 99 235 / 0.6); }
.viewport.flash .hotspot { animation: folioscope-flash 0.7s ease-out; }
@keyframes folioscope-flash { from { background: rgb(37 99 235 / 0.35); box-shadow: inset 0 0 0 2px rgb(37 99 235 / 0.8); } }
`;

/**
 * `<folioscope-player src="app.folio">`: plays a baked prototype. Raw design files need the full
 * player, which comes later.
 */
export class FolioscopePlayer extends BaseElement {
  static readonly observedAttributes: readonly string[] = ["src", "flow", "page", "controls"];
  // `poster`, `fit`, `preload` and `hotspots` are read when needed, so changing them needs no
  // callback.

  private readonly root: ShadowRoot;
  private readonly viewport: HTMLElement;
  private readonly stage: HTMLElement;
  private runtime?: Runtime;
  private index?: FolioIndex;
  private base = "";
  /** The `src` loaded or loading. */
  private loadedSrc?: string;
  /** Files baked in the browser, as `blob:` URLs by path, when `src` is a raw design file. */
  private readonly blobs = new Map<string, string>();
  private density = 1;
  private shown?: HTMLElement;
  /** The frame `shown` draws, for smart animate. */
  private shownFrame?: FolioFrame;
  private timers: ReturnType<typeof setTimeout>[] = [];
  private loading = 0;
  private preloadedPage?: FolioIndex["pages"][number];
  private showing = 0;
  private resize?: ResizeObserver;
  private readonly controls: Controls;
  /** The hovers and presses holding a frame, innermost last, with the hotspot that started each. */
  private holds: { kind: HoldKind; bounds: Rect }[] = [];
  private readonly releasePress = () => this.release((hold) => hold.kind === "press");

  constructor() {
    super();
    this.root = this.attachShadow({ mode: "open" });
    this.root.innerHTML = `<style>${STYLE}${CONTROLS_STYLE}</style><div class="viewport" part="viewport"><div class="stage"></div></div>`;
    this.viewport = this.root.querySelector(".viewport")!;
    this.stage = this.root.querySelector(".stage")!;
    this.controls = new Controls({
      restart: () => this.restart(),
      back: () => this.back(),
      page: (i) =>
        this.setAttribute("page", String(this.index?.pages.indexOf(this.playablePages()[i]!) ?? 0)),
      flow: (i) => this.setAttribute("flow", String(i)),
      fullscreen: () => void this.requestFullscreen?.().catch(() => undefined),
    });
    this.controls.element.hidden = !this.hasAttribute("controls");
    this.root.append(this.controls.element);
    // `hotspots="on-miss"` (the default): a click beside every hotspot shows where they are.
    this.viewport.addEventListener("click", (event) => {
      const missed = !event
        .composedPath()
        .some((el) => el instanceof HTMLElement && el.classList.contains("hotspot"));
      if (missed && (this.getAttribute("hotspots") ?? "on-miss") === "on-miss") {
        this.viewport.classList.remove("flash");
        void this.viewport.offsetWidth; // restarts the animation
        this.viewport.classList.add("flash");
      }
    });
    this.viewport.addEventListener("animationend", () => this.viewport.classList.remove("flash"));
    this.addEventListener("fullscreenchange", () => this.fit());
    // A hover lasts while the pointer stays over the hotspot that started it.
    this.viewport.addEventListener("pointermove", (event) =>
      this.release((hold) => hold.kind === "hover" && !this.under(event, hold.bounds)),
    );
    this.viewport.addEventListener("pointerleave", () =>
      this.release((hold) => hold.kind === "hover"),
    );
    this.addEventListener("keydown", (event) => {
      if (event.key === "Escape" && this.runtime?.canGoBack) {
        event.preventDefault();
        void this.show(this.runtime.back());
      }
    });
  }

  get src(): string {
    return this.getAttribute("src") ?? "";
  }

  set src(value: string) {
    this.setAttribute("src", value);
  }

  /** The frame on screen. */
  get frame(): FolioFrame | undefined {
    return this.runtime?.frame;
  }

  connectedCallback(): void {
    // A press ends wherever the button is released, even outside the player.
    globalThis.addEventListener("pointerup", this.releasePress);
    globalThis.addEventListener("pointercancel", this.releasePress);
    this.resize = new ResizeObserver(() => this.fit());
    this.resize.observe(this.viewport);
    if (!this.index) void this.load();
  }

  disconnectedCallback(): void {
    globalThis.removeEventListener("pointerup", this.releasePress);
    globalThis.removeEventListener("pointercancel", this.releasePress);
    this.resize?.disconnect();
    this.clearTimers();
  }

  attributeChangedCallback(name: string, previous: string | null, value: string | null): void {
    if (previous === value || !this.isConnected) return;
    if (name === "controls") this.controls.element.hidden = value === null;
    else if (name === "src") void this.load();
    else if (this.index) this.startPage();
  }

  /** Back to the start of the flow. */
  restart(): void {
    if (this.runtime) void this.show(this.runtime.restart());
  }

  back(): void {
    if (this.runtime) void this.show(this.runtime.back());
  }

  goTo(frameId: string): void {
    if (this.runtime) void this.show(this.runtime.goTo(frameId));
  }

  private async load(): Promise<void> {
    const src = this.src;
    // Upgrading an element already in the page calls both attributeChangedCallback and
    // connectedCallback: one load per source.
    if (!src || src === this.loadedSrc) return;
    this.loadedSrc = src;
    const attempt = ++this.loading;
    this.showPoster();
    try {
      const index = RAW.test(src) ? await this.loadRaw(src, attempt) : await this.loadFolio(src);
      if (!index) return;
      if (index.folio > SUPPORTED_FOLIO || index.irVersion > SUPPORTED_IR) {
        throw new Error(
          `${src} was baked by a newer folioscope (folio v${index.folio}, IR v${index.irVersion})`,
        );
      }
      if (attempt !== this.loading) return;
      this.index = index;
      const dpr = globalThis.devicePixelRatio ?? 1;
      this.density =
        [...index.densities].sort((a, b) => a - b).find((d) => d >= dpr) ??
        Math.max(...index.densities);
      this.startPage();
      this.emit("folioscope:load", { pages: index.pages.length });
    } catch (error) {
      this.emit("folioscope:error", { error });
    }
  }

  /** A `.folio` folder (or its `index.json`). */
  private async loadFolio(src: string): Promise<FolioIndex> {
    this.forgetBlobs();
    this.base = src.endsWith("index.json")
      ? src.slice(0, -"index.json".length)
      : src.endsWith("/")
        ? src
        : `${src}/`;
    const response = await fetch(`${this.base}index.json`);
    if (!response.ok) throw new Error(`${this.base}index.json: HTTP ${response.status}`);
    return (await response.json()) as FolioIndex;
  }

  /**
   * A raw design file: its baked `.folio` next to it when there is one (`app.fig` →
   * `app.folio/`), else baked here by the full player, loaded on demand.
   */
  private async loadRaw(src: string, attempt: number): Promise<FolioIndex | undefined> {
    const sibling = src.replace(RAW, ".folio/");
    const found = await fetch(`${sibling}index.json`).catch(() => undefined);
    if (found?.ok) {
      this.forgetBlobs();
      this.base = sibling;
      return (await found.json()) as FolioIndex;
    }
    const { bakeInBrowser } = await import("./full/index.ts");
    const baked = await bakeInBrowser(new URL(src, document.baseURI).href, {
      density: (globalThis.devicePixelRatio ?? 1) > 1 ? 2 : 1,
      onProgress: (stage) => this.emit("folioscope:progress", { stage }),
    });
    if (attempt !== this.loading) return undefined;
    for (const font of baked.missingFonts) this.emit("folioscope:font-missing", { font });
    this.forgetBlobs();
    this.base = "";
    for (const [path, bytes] of baked.files) {
      const type = TYPES[path.slice(path.lastIndexOf("."))] ?? "application/octet-stream";
      this.blobs.set(
        path,
        URL.createObjectURL(new Blob([bytes as Uint8Array<ArrayBuffer>], { type })),
      );
    }
    const index = baked.files.get("index.json");
    if (!index) throw new Error(`${src}: the bake made no index.json`);
    return JSON.parse(new TextDecoder().decode(index)) as FolioIndex;
  }

  /** Releases the files of a previous in-browser bake. */
  private forgetBlobs(): void {
    for (const url of this.blobs.values()) URL.revokeObjectURL(url);
    this.blobs.clear();
  }

  private startPage(): void {
    const index = this.index!;
    const pageAttr = this.getAttribute("page");
    const page =
      (pageAttr && /^\d+$/.test(pageAttr)
        ? index.pages[Number(pageAttr)]
        : index.pages.find((p) => p.name === pageAttr)) ??
      index.pages.find((p) => p.flows.length > 0) ??
      index.pages[0];
    if (!page) return;
    const flowAttr = this.getAttribute("flow") ?? undefined;
    this.runtime = new Runtime(page, flowAttr);
    const flowIndex =
      flowAttr !== undefined && /^\d+$/.test(flowAttr)
        ? Number(flowAttr)
        : Math.max(
            0,
            page.flows.findIndex((f) => f.name === flowAttr),
          );
    this.controls.load(index, page, flowIndex);
    this.showPoster(this.runtime.frame.placeholder);
    void this.show(this.runtime.restart());
  }

  private async show(step: Step | undefined): Promise<void> {
    if (!step) return;
    // A newer navigation supersedes this one; the runtime already holds the latest state.
    const seq = ++this.showing;
    this.clearTimers();
    // Clicking during a transition jumps it to its end.
    for (const animation of this.stage.getAnimations({ subtree: true })) animation.finish();
    const hadFocus = this.root.activeElement !== null;
    const shown =
      step.layer === "overlay" && this.shown
        ? await this.showOverlay(step, seq)
        : await this.showScreen(step, seq);
    if (!shown || seq !== this.showing) return;
    // Focus would be lost with the outgoing screen: move it to the new one, announced by name.
    if (hadFocus) shown.focus({ preventScroll: true });
    this.controls.update(this.runtime?.canGoBack ?? false);
    const top = step.overlays.at(-1)?.frame ?? step.frame;
    this.emit("folioscope:navigate", { frame: top.id, name: top.name });
    this.schedule(top);
    this.preload(top);
  }

  /** Replaces the screen and its overlays; returns the frame on top. */
  private async showScreen(step: Step, seq: number): Promise<HTMLElement | undefined> {
    const { frame } = step;
    this.viewport.style.setProperty("--w", String(frame.bounds.width));
    this.viewport.style.setProperty("--h", String(frame.bounds.height));
    this.fit();
    const incoming = this.render(frame);
    const overlays = step.overlays.map((overlay) => this.renderOverlay(overlay, frame));
    await Promise.all([incoming, ...overlays].map(decoded));
    if (seq !== this.showing) return undefined;
    const outgoing = this.shown;
    this.shown = incoming;
    for (const el of this.stage.querySelectorAll(":scope > .overlay")) el.remove();
    this.stage.append(incoming, ...overlays);
    this.root.querySelector(".poster")?.remove();
    const before = this.shownFrame;
    this.shownFrame = frame;
    if (step.transition?.kind.type === "smartAnimate" && before?.parts && frame.parts) {
      await playSmart(this.stage, before, frame, step.transition, (layer) => this.layerUrl(layer));
    } else {
      await play(outgoing, incoming, step.transition, step.reverse ?? false);
    }
    if (seq !== this.showing) return undefined;
    for (const el of this.stage.querySelectorAll(":scope > .frame"))
      if (el !== incoming) el.remove();
    return overlays.at(-1)?.querySelector<HTMLElement>(".frame") ?? incoming;
  }

  /** Opens, closes or swaps the top overlay, the screen below staying; returns the frame on top. */
  private async showOverlay(step: Step, seq: number): Promise<HTMLElement | undefined> {
    const open = [...this.stage.querySelectorAll<HTMLElement>(":scope > .overlay")];
    const wanted = step.overlays;
    const below = Math.min(open.length, wanted.length) - (open.length === wanted.length ? 1 : 0);
    // Only the top overlay may differ; anything else redraws the whole screen.
    if (open.slice(0, below).some((el, i) => el.dataset.frame !== wanted[i]!.frame.id)) {
      return this.showScreen({ ...step, transition: undefined }, seq);
    }
    const closing = open.length > below ? open.at(-1) : undefined;
    const top = wanted.length > below ? wanted.at(-1)! : undefined;
    const opening = top && this.renderOverlay(top, step.frame);
    if (opening) {
      await decoded(opening);
      if (seq !== this.showing) return undefined;
      this.stage.append(opening);
    }
    const transition = top?.transition ?? step.transition;
    if (transition) {
      await Promise.all([
        opening && playOverlay(opening, transition, false),
        closing && playOverlay(closing, transition, true),
      ]);
    }
    if (seq !== this.showing) return undefined;
    closing?.remove();
    const shown = this.stage.querySelectorAll<HTMLElement>(":scope > .overlay > .sheet > .frame");
    return shown[shown.length - 1] ?? this.shown;
  }

  /** A frame with its hotspots wired to the runtime; `at` places it in the screen's coordinates. */
  private render(frame: FolioFrame, at: Point = { x: 0, y: 0 }): HTMLElement {
    const el = renderFrame(frame, (layer) => this.layerUrl(layer), {
      action: (action, bounds) => {
        if (action.type === "scrollTo") return scrollFrame(el, action);
        const origin = { ...bounds, x: bounds.x + at.x, y: bounds.y + at.y };
        void this.show(this.runtime?.run(action, origin));
        if (action.type === "openUrl")
          window.open(action.url, action.newTab ? "_blank" : "_self", "noopener");
      },
      hold: (action, kind, bounds) => this.hold(action, kind, bounds),
    });
    return el;
  }

  /** An overlay layer: a backdrop over the whole screen, and the overlay frame at its position. */
  private renderOverlay(overlay: OpenOverlay, screen: FolioFrame): HTMLElement {
    const settings = overlay.frame.overlay ?? { position: "center", closeOnClickOutside: false };
    const at = overlayPosition(settings.position, screen.bounds, overlay);
    const layer = document.createElement("div");
    layer.className = "overlay";
    layer.dataset.frame = overlay.frame.id;
    const backdrop = document.createElement("div");
    backdrop.className = "backdrop";
    if (settings.backdrop) backdrop.style.background = settings.backdrop;
    // The screen below does not react while an overlay is open.
    backdrop.addEventListener("click", () => {
      if (settings.closeOnClickOutside) void this.show(this.runtime?.close());
    });
    const sheet = document.createElement("div");
    sheet.className = "sheet";
    const el = this.render(overlay.frame, at);
    Object.assign(el.style, {
      inset: "auto",
      left: `${at.x}px`,
      top: `${at.y}px`,
      width: `${overlay.frame.bounds.width}px`,
      height: `${overlay.frame.bounds.height}px`,
    });
    sheet.append(el);
    layer.append(backdrop, sheet);
    return layer;
  }

  private hold(action: Action, kind: HoldKind, bounds: Rect): void {
    const step = this.runtime?.hold(action);
    if (!step) return;
    this.holds.push({ kind, bounds });
    void this.show(step);
  }

  /** Ends the innermost holds while `ends` says so, then shows the frame they return to. */
  private release(ends: (hold: { kind: HoldKind; bounds: Rect }) => boolean): void {
    const runtime = this.runtime;
    if (!runtime) return;
    // A navigation, back or restart drops every hold in the runtime.
    this.holds.length = Math.min(this.holds.length, runtime.holding);
    let step: Step | undefined;
    while (this.holds.length > 0 && ends(this.holds.at(-1)!)) {
      this.holds.pop();
      step = runtime.release() ?? step;
    }
    if (step) void this.show(step);
  }

  /** Whether a pointer event falls inside `bounds`, in the frame's coordinates. */
  private under(event: PointerEvent, bounds: Rect): boolean {
    const stage = this.stage.getBoundingClientRect();
    const scale = stage.width / (Number(this.viewport.style.getPropertyValue("--w")) || 1);
    const x = (event.clientX - stage.left) / scale;
    const y = (event.clientY - stage.top) / scale;
    return contains(bounds, { x, y });
  }

  /** "After delay" interactions start counting when the frame shows. */
  private schedule(frame: FolioFrame): void {
    for (const hotspot of frame.hotspots) {
      for (const { trigger, action } of hotspot.interactions) {
        if (trigger.type === "afterDelay") {
          this.timers.push(
            setTimeout(() => void this.show(this.runtime?.run(action)), trigger.delayMs),
          );
        }
      }
    }
  }

  /**
   * `preload`: `neighbors` (default) fetches the frames this one links to, `all` every frame of
   * the page once, `start` and `none` nothing beyond the frame on screen.
   */
  private preload(frame: FolioFrame): void {
    const strategy = this.getAttribute("preload") ?? "neighbors";
    const page = this.runtime?.page;
    if (!page || strategy === "none" || strategy === "start") return;
    let targets: FolioFrame[];
    if (strategy === "all") {
      if (this.preloadedPage === page) return;
      this.preloadedPage = page;
      targets = page.frames;
    } else {
      const frames = new Map(page.frames.map((f) => [f.id, f]));
      targets = frame.hotspots
        .flatMap((h) => h.interactions)
        .flatMap(({ action }) => ("to" in action ? [frames.get(action.to)] : []))
        .filter((f) => f !== undefined);
    }
    // Smart animate draws both frames' parts: fetch this one's, and the targets'.
    for (const target of [frame, ...targets]) {
      const parts = (target.parts ?? []).map((part) => part.layer);
      for (const layer of target === frame ? parts : [...target.layers, ...parts]) {
        new Image().src = this.layerUrl(layer);
      }
    }
  }

  private playablePages() {
    return this.index?.pages.filter((p) => p.frames.length > 0) ?? [];
  }

  /** The `poster` image, else the frame's baked placeholder, until the first frame is ready. */
  private showPoster(placeholder?: string): void {
    const poster =
      this.getAttribute("poster") ??
      (placeholder
        ? thumbHashToDataURL(Uint8Array.from(atob(placeholder), (c) => c.charCodeAt(0)))
        : null);
    if (!poster || this.root.querySelector(".poster")) return;
    const img = document.createElement("img");
    img.className = "poster";
    img.alt = "";
    img.src = poster;
    this.viewport.append(img);
  }

  /**
   * Scales the prototype to the element's width. In fullscreen, the screen's height bounds it too,
   * and the prototype is centered.
   */
  private fit(): void {
    const style = this.viewport.style;
    const width = Number(style.getPropertyValue("--w")) || 1;
    const height = Number(style.getPropertyValue("--h")) || 1;
    const { clientWidth, clientHeight } = this.viewport;
    let [scale, x, y] = [1, 0, 0];
    if ((this.getAttribute("fit") ?? "contain") !== "none") {
      if (document.fullscreenElement === this) {
        // As large as the screen allows, centred.
        const placed = fitContain({ width, height }, { width: clientWidth, height: clientHeight });
        [scale, x, y] = [placed.a, placed.e, placed.f];
      } else {
        scale = clientWidth / width;
      }
    }
    style.setProperty("--scale", String(scale || 1));
    style.setProperty("--x", `${x}px`);
    style.setProperty("--y", `${y}px`);
  }

  private layerUrl(layer: Layer): string {
    const path = `layers/${layer.src}@${this.density}x.${layer.format}`;
    return this.blobs.get(path) ?? `${this.base}${path}`;
  }

  private clearTimers(): void {
    this.timers.forEach(clearTimeout);
    this.timers = [];
  }

  private emit(type: string, detail: object): void {
    this.dispatchEvent(new CustomEvent(type, { detail, bubbles: true, composed: true }));
  }
}

/** Top-left corner of an overlay in the screen's coordinates. */
function overlayPosition(
  position: OverlaySettings["position"],
  screen: Rect,
  overlay: OpenOverlay,
): Point {
  const { width, height } = overlay.frame.bounds;
  if (position === "manual") {
    const origin = overlay.origin ?? { x: 0, y: 0 };
    return { x: origin.x + (overlay.offset?.x ?? 0), y: origin.y + (overlay.offset?.y ?? 0) };
  }
  const x = position.endsWith("Left")
    ? 0
    : position.endsWith("Right")
      ? screen.width - width
      : (screen.width - width) / 2;
  const y = position.startsWith("top")
    ? 0
    : position.startsWith("bottom")
      ? screen.height - height
      : (screen.height - height) / 2;
  return { x, y };
}
