// SPDX-License-Identifier: AGPL-3.0-or-later
import {
  type Action,
  type FolioIndex,
  type FolioFrame,
  type Layer,
  type Rect,
  SUPPORTED_FOLIO,
  SUPPORTED_IR,
} from "./folio.ts";
import { CONTROLS_STYLE, Controls } from "./controls.ts";
import { thumbHashToDataURL } from "thumbhash";
import { decoded, type HoldKind, play, renderFrame } from "./lite.ts";
import { Runtime, type Step } from "./runtime.ts";

export const TAG_NAME = "folioscope-player";

// Keeps the module importable during SSR (VitePress build, the Vite plugin), where HTMLElement is undefined.
const BaseElement: typeof HTMLElement =
  globalThis.HTMLElement ?? (class {} as unknown as typeof HTMLElement);

const STYLE = `
:host { display: block; position: relative; contain: content; }
:host([hidden]) { display: none; }
.viewport { position: relative; width: 100%; overflow: hidden; aspect-ratio: var(--w) / var(--h); }
.stage { position: absolute; top: 0; left: 0; width: calc(var(--w) * 1px); height: calc(var(--h) * 1px); transform-origin: 0 0; transform: scale(var(--scale, 1)); }
.frame { position: absolute; inset: 0; overflow: hidden; background: transparent; }
.frame img, .poster { position: absolute; display: block; user-select: none; pointer-events: none; }
.poster { inset: 0; width: 100%; height: 100%; object-fit: contain; }
.hotspot { position: absolute; margin: 0; padding: 0; border: 0; background: none; cursor: pointer; }
.hotspot:focus-visible { outline: 2px solid #2563eb; outline-offset: 2px; }
`;

/**
 * `<folioscope-player src="app.folio">`: plays a baked prototype. Raw design files need the full
 * player, which comes later.
 */
export class FolioscopePlayer extends BaseElement {
  static readonly observedAttributes: readonly string[] = ["src", "flow", "page", "controls"];
  // `poster`, `fit` and `preload` are read when needed, so changing them needs no callback.

  private readonly root: ShadowRoot;
  private readonly viewport: HTMLElement;
  private readonly stage: HTMLElement;
  private runtime?: Runtime;
  private index?: FolioIndex;
  private base = "";
  private density = 1;
  private shown?: HTMLElement;
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
    if (!src) return;
    const attempt = ++this.loading;
    this.showPoster();
    try {
      if (/\.(fig|sketch|xd|penpot)$/i.test(src)) {
        throw new Error(
          `${src}: raw design files need the full player, not available yet; bake it with \`folioscope bake\``,
        );
      }
      this.base = src.endsWith("index.json")
        ? src.slice(0, -"index.json".length)
        : src.endsWith("/")
          ? src
          : `${src}/`;
      const response = await fetch(`${this.base}index.json`);
      if (!response.ok) throw new Error(`${this.base}index.json: HTTP ${response.status}`);
      const index = (await response.json()) as FolioIndex;
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
    const { frame } = step;
    this.viewport.style.setProperty("--w", String(frame.bounds.width));
    this.viewport.style.setProperty("--h", String(frame.bounds.height));
    this.fit();
    const incoming = renderFrame(frame, (layer) => this.layerUrl(layer), {
      action: (action) => {
        void this.show(this.runtime?.run(action));
        if (action.type === "openUrl")
          window.open(action.url, action.newTab ? "_blank" : "_self", "noopener");
      },
      hold: (action, kind, bounds) => this.hold(action, kind, bounds),
    });
    await decoded(incoming);
    if (seq !== this.showing) return;
    // Clicking during a transition jumps it to its end.
    for (const animation of this.stage.getAnimations({ subtree: true })) animation.finish();
    const outgoing = this.shown;
    // Focus would be lost with the outgoing screen: move it to the new one, announced by name.
    const hadFocus = this.root.activeElement !== null;
    this.shown = incoming;
    this.stage.append(incoming);
    this.root.querySelector(".poster")?.remove();
    await play(outgoing, incoming, step.transition, step.reverse ?? false);
    if (seq !== this.showing) return;
    for (const el of this.stage.querySelectorAll(".frame")) if (el !== incoming) el.remove();
    if (hadFocus) incoming.focus({ preventScroll: true });
    this.controls.update(this.runtime?.canGoBack ?? false);
    this.emit("folioscope:navigate", { frame: frame.id, name: frame.name });
    this.schedule(frame);
    this.preload(frame);
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
    return (
      x >= bounds.x && x < bounds.x + bounds.width && y >= bounds.y && y < bounds.y + bounds.height
    );
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
    for (const target of targets) {
      for (const layer of target.layers) new Image().src = this.layerUrl(layer);
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

  private fit(): void {
    const width = Number(this.viewport.style.getPropertyValue("--w")) || 1;
    const fit = this.getAttribute("fit") ?? "contain";
    const scale = fit === "none" ? 1 : this.viewport.clientWidth / width;
    this.viewport.style.setProperty("--scale", String(scale || 1));
  }

  private layerUrl(layer: Layer): string {
    return `${this.base}layers/${layer.src}@${this.density}x.${layer.format}`;
  }

  private clearTimers(): void {
    this.timers.forEach(clearTimeout);
    this.timers = [];
  }

  private emit(type: string, detail: object): void {
    this.dispatchEvent(new CustomEvent(type, { detail, bubbles: true, composed: true }));
  }
}
