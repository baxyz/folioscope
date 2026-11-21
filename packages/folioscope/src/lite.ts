// SPDX-License-Identifier: AGPL-3.0-or-later
import { spring, toCssLinear } from "helpers4/animation";
import { prefersReducedMotion } from "helpers4/browser";
import type {
  Action,
  FolioComponent,
  FolioFrame,
  Hotspot,
  Layer,
  Point,
  Rect,
  Side,
  Transition,
  Trigger,
} from "./folio.ts";

/** A hold lasts while the pointer hovers its hotspot, or until the press is released. */
export type HoldKind = "hover" | "press";

export interface FrameHandlers {
  /** Runs an action: a click, a pointer entering or leaving; `bounds` is the hotspot's. */
  action: (action: Action, bounds: Rect) => void;
  /** Shows an action's frame for as long as the hover or the press lasts. */
  hold: (action: Action, kind: HoldKind, bounds: Rect) => void;
}

/**
 * Builds a frame: its layers as images, its hotspots as transparent elements. Clickable hotspots
 * are focusable buttons; those that only react to the pointer are plain areas.
 */
export function renderFrame(
  frame: FolioFrame,
  layerUrl: (layer: Layer) => string,
  on: FrameHandlers,
): HTMLElement {
  const el = document.createElement("div");
  el.className = "frame";
  el.setAttribute("role", "group");
  el.setAttribute("aria-label", frame.name);
  el.tabIndex = -1;
  // A scrolling frame: background layers, then a scroller holding the content and its hotspots,
  // then fixed layers and hotspots on top. Frames that scroll inside the screen get scrollers of
  // their own, in the content they are part of.
  let content: HTMLElement | undefined;
  const inner = new Map<string, HTMLElement>();
  const scroller = (
    direction: "vertical" | "horizontal" | "both",
    size: { width: number; height: number },
  ) => {
    const box = document.createElement("div");
    box.className = "scroll";
    box.style.overflowX = direction === "vertical" ? "hidden" : "auto";
    box.style.overflowY = direction === "horizontal" ? "hidden" : "auto";
    const area = document.createElement("div");
    area.className = "content";
    area.style.width = `${size.width}px`;
    area.style.height = `${size.height}px`;
    box.append(area);
    return [box, area] as const;
  };
  const innerOf = (id: string) => {
    let area = inner.get(id);
    const spec = frame.scrollers?.find((s) => s.id === id);
    if (!area && spec) {
      const [box, made] = scroller(spec.direction, spec.content);
      place(box, spec.bounds);
      box.classList.add("inner");
      (content ?? el).append(box);
      inner.set(id, made);
      area = made;
    }
    return area;
  };
  for (const layer of frame.layers) {
    const img = document.createElement("img");
    img.src = layerUrl(layer);
    img.alt = "";
    img.draggable = false;
    place(img, layer.bounds);
    let parent: HTMLElement = el;
    if (layer.scroller) {
      parent = innerOf(layer.scroller) ?? el;
    } else if (layer.scrolls && frame.scroll) {
      if (!content) {
        const [box, area] = scroller(frame.scroll.direction, frame.scroll.content);
        content = area;
        el.append(box);
      }
      parent = content;
    }
    parent.append(img);
    if (layer.sticky) stick(img, layer.bounds, parent.parentElement!);
  }
  for (const hotspot of frame.hotspots) {
    const area = renderHotspot(hotspot, on);
    if (!area) continue;
    const parent = hotspot.scroller
      ? innerOf(hotspot.scroller)
      : hotspot.fixed
        ? el
        : (content ?? el);
    (parent ?? el).append(area);
  }
  // Components go last: they sit above the frame's own hotspots, as in the design file.
  for (const component of frame.components ?? []) {
    const zone = renderComponent(component, layerUrl, on);
    (component.fixed ? el : (content ?? el)).append(zone);
  }
  return el;
}

/**
 * Keeps a sticky layer in its scroller's view once the scroll reaches it: it moves with the
 * content, then holds at the top (or left) edge.
 */
function stick(img: HTMLElement, bounds: Rect, box: HTMLElement): void {
  img.style.zIndex = "1";
  const update = () => {
    const dy = Math.max(0, box.scrollTop - bounds.y);
    const dx = box.style.overflowX === "hidden" ? 0 : Math.max(0, box.scrollLeft - bounds.x);
    img.style.transform = dx || dy ? `translate(${dx}px, ${dy}px)` : "";
  };
  box.addEventListener("scroll", update, { passive: true });
}

/**
 * A hotspot as a transparent element: a focusable button when it can be clicked, a plain area
 * when it only reacts to the pointer; `undefined` when nothing in it is for the player.
 */
function renderHotspot(hotspot: Hotspot, on: FrameHandlers): HTMLElement | undefined {
  const by = (type: Trigger["type"]) =>
    hotspot.interactions.filter((i) => i.trigger.type === type).map((i) => i.action);
  const clicks = by("click");
  const drags = by("drag");
  const pointer = POINTER_TRIGGERS.some((type) => by(type).length > 0) || drags.length > 0;
  if (clicks.length === 0 && !pointer) return undefined;
  const area: HTMLElement = document.createElement(clicks.length > 0 ? "button" : "div");
  area.className = "hotspot";
  place(area, hotspot.bounds);
  if (area instanceof HTMLButtonElement) {
    area.type = "button";
    area.setAttribute("aria-label", hotspot.name);
    area.addEventListener("click", (event) => {
      event.stopPropagation();
      for (const action of clicks) on.action(action, hotspot.bounds);
    });
  }
  if (pointer) {
    const run = (actions: Action[]) => {
      for (const action of actions) on.action(action, hotspot.bounds);
    };
    area.addEventListener("pointerenter", (event) => {
      run(by("mouseEnter"));
      // Touch screens have no hover: a tap is a press.
      if (event.pointerType !== "touch") {
        for (const action of by("hover")) on.hold(action, "hover", hotspot.bounds);
      }
    });
    area.addEventListener("pointerleave", () => run(by("mouseLeave")));
    area.addEventListener("pointerdown", () => {
      run(by("mouseDown"));
      for (const action of by("press")) on.hold(action, "press", hotspot.bounds);
    });
    area.addEventListener("pointerup", () => run(by("mouseUp")));
    if (drags.length > 0) {
      // A drag is a press that moves far enough before it ends.
      let start: { x: number; y: number } | undefined;
      area.style.touchAction = "none";
      area.addEventListener("pointerdown", (event) => {
        start = { x: event.clientX, y: event.clientY };
      });
      area.addEventListener("pointermove", (event) => {
        if (!start) return;
        if (Math.hypot(event.clientX - start.x, event.clientY - start.y) >= DRAG_DISTANCE) {
          start = undefined;
          run(drags);
        }
      });
      const end = () => (start = undefined);
      area.addEventListener("pointerup", end);
      area.addEventListener("pointercancel", end);
    }
  }
  return area;
}

/**
 * An interactive component: the image of its current state, with that state's hotspots. A
 * "change to" swaps the state in place; while hovering or pressing, until the hover or the press
 * ends. Other actions go to the frame, their hotspot moved into the frame's coordinates.
 */
function renderComponent(
  component: FolioComponent,
  layerUrl: (layer: Layer) => string,
  on: FrameHandlers,
): HTMLElement {
  const zone = document.createElement("div");
  zone.className = "component";
  zone.dataset.component = component.id;
  place(zone, component.bounds);
  const inFrame = (bounds: Rect): Rect => ({
    ...bounds,
    x: bounds.x + component.bounds.x,
    y: bounds.y + component.bounds.y,
  });
  let current = component.initial;
  const held: string[] = [];
  let timers: ReturnType<typeof setTimeout>[] = [];
  const show = (variant: string, transition?: Transition) => {
    const state = component.states.find((s) => s.variant === variant);
    if (!state) return;
    const previous = component.states.find((s) => s.variant === current);
    current = variant;
    // "After delay" inside a state starts counting when the state shows.
    timers.forEach(clearTimeout);
    timers = state.hotspots.flatMap((h) =>
      h.interactions.flatMap(({ trigger, action }) =>
        trigger.type === "afterDelay"
          ? [
              setTimeout(
                () => zone.isConnected && handlers.action(action, h.bounds),
                trigger.delayMs,
              ),
            ]
          : [],
      ),
    );
    zone.dataset.state = state.name;
    zone.style.width = `${state.bounds.width}px`;
    zone.style.height = `${state.bounds.height}px`;
    const img = document.createElement("img");
    img.src = layerUrl(state.layer);
    img.alt = "";
    img.draggable = false;
    place(img, state.layer.bounds);
    const before = zone.querySelector("img");
    const areas = state.hotspots.flatMap((h) => renderHotspot(h, handlers) ?? []);
    // Components nested in the state play on their own, over it; what else they do goes up.
    const nested = (state.components ?? []).map((c) => renderComponent(c, layerUrl, handlers));
    zone.replaceChildren(...(before ? [before] : []), img, ...nested, ...areas);
    const reduced = prefersReducedMotion();
    if (
      !before ||
      !transition ||
      reduced ||
      transition.durationMs === 0 ||
      transition.kind.type === "instant"
    ) {
      before?.remove();
      return;
    }
    if (transition.kind.type === "smartAnimate" && previous?.parts && state.parts) {
      // The parts of both states move from one to the other, in place of both images: the
      // state left behind may be larger than the parts that shrink from it.
      img.style.visibility = "hidden";
      before.style.visibility = "hidden";
      void playSmart(zone, previous.parts, state.parts, transition, layerUrl).then(() => {
        img.style.visibility = "";
        before.remove();
      });
      return;
    }
    void img
      .animate([{ opacity: 0 }, { opacity: 1 }], {
        ...timing(transition),
      })
      .finished.catch(() => undefined)
      .then(() => before.remove());
  };
  const handlers: FrameHandlers = {
    action: (action, bounds) => {
      if (action.type === "changeTo") show(action.to, action.transition);
      else on.action(action, inFrame(bounds));
    },
    hold: (action, kind, bounds) => {
      if (action.type !== "changeTo") return on.hold(action, kind, inFrame(bounds));
      held.push(current);
      show(action.to, action.transition);
      const release = () => {
        const back = held.pop();
        if (back !== undefined) show(back);
      };
      if (kind === "press") globalThis.addEventListener("pointerup", release, { once: true });
      else zone.addEventListener("pointerleave", release, { once: true });
    },
  };
  // Every state is fetched now, so a change shows at once.
  for (const state of component.states) new Image().src = layerUrl(state.layer);
  show(component.initial);
  return zone;
}

/** How far, in CSS pixels, a press must move to be a drag. */
const DRAG_DISTANCE = 10;

const POINTER_TRIGGERS = [
  "hover",
  "press",
  "mouseEnter",
  "mouseLeave",
  "mouseDown",
  "mouseUp",
] as const;

/**
 * Opens or closes an overlay layer (its `.backdrop` and `.sheet`) with the overlay's transition:
 * slides and moves bring the sheet in from that side of the screen, anything else fades it.
 */
export async function playOverlay(
  layer: HTMLElement,
  transition: Transition,
  closing: boolean,
): Promise<void> {
  const reduced = prefersReducedMotion();
  if (reduced || transition.durationMs === 0 || transition.kind.type === "instant") return;
  const options: KeyframeAnimationOptions = {
    ...timing(transition),
    fill: "both",
  };
  const kind = transition.kind;
  const [hidden, shown]: Keyframe[] =
    kind.type === "slideIn" || kind.type === "moveIn" || kind.type === "push"
      ? [{ transform: offset(kind.from, 1) }, { transform: "none" }]
      : [{ opacity: 0 }, { opacity: 1 }];
  const run = (el: Element | null, from: Keyframe, to: Keyframe) =>
    el?.animate(closing ? [to, from] : [from, to], options).finished.catch(() => undefined);
  await Promise.all([
    run(layer.querySelector(".sheet"), hidden!, shown!),
    run(layer.querySelector(".backdrop"), { opacity: 0 }, { opacity: 1 }),
  ]);
}

/**
 * Smart animate (docs/spec/player.md): over `stage`, the parts of `from` and `to` paired by name move and
 * scale from their bounds in one frame to those in the other, the second image fading in over the
 * first; parts on one side only fade out or in. Resolves when it is over; does nothing when a
 * frame has no parts or motion is reduced.
 */
export async function playSmart(
  stage: HTMLElement,
  fromParts: Part[] | undefined,
  toParts: Part[] | undefined,
  transition: Transition,
  layerUrl: (layer: Layer) => string,
): Promise<void> {
  if (!fromParts?.length || !toParts?.length || prefersReducedMotion()) return;
  const from = { parts: fromParts };
  const to = { parts: toParts };
  const options: KeyframeAnimationOptions = { ...timing(transition), fill: "both" };
  const layer = document.createElement("div");
  layer.className = "smart";
  // Pairs by name, in order: the n-th part named X in `from` with the n-th one in `to`.
  const pairs = new Map<number, number>();
  for (const [i, part] of from.parts.entries()) {
    const j = to.parts.findIndex(
      (other, k) => other.name === part.name && ![...pairs.values()].includes(k),
    );
    if (j >= 0) pairs.set(i, j);
  }
  const partnerOf = (j: number) => [...pairs].find(([, k]) => k === j)?.[0];
  const moves: [HTMLElement, Keyframe[]][] = [];
  for (const [i, part] of from.parts.entries()) {
    const j = pairs.get(i);
    const end = j === undefined ? undefined : to.parts[j]!;
    // Two plain boxes: one box changes size, corners, colour and place; nothing to cross-fade.
    if (end && part.shape && end.shape) {
      const box = partElement(layer, part, layerUrl);
      moves.push([box, [boxFrame(part), boxFrame(end)]]);
      continue;
    }
    const el = partElement(layer, part, layerUrl);
    moves.push([
      el,
      // A paired part stays opaque under its partner fading in: a cross-fade of two half
      // transparent images would show what is behind them.
      end
        ? [{ transform: partTransform(part) }, { transform: partTransform(end, part) }]
        : [{ opacity: 1 }, { opacity: 0 }],
    ]);
  }
  for (const [j, part] of to.parts.entries()) {
    const i = partnerOf(j);
    const start = i === undefined ? undefined : from.parts[i]!;
    if (start?.shape && part.shape) continue;
    const el = partElement(layer, part, layerUrl);
    moves.push([
      el,
      start
        ? [
            { transform: partTransform(start, part), opacity: 0 },
            { transform: partTransform(part), opacity: 1 },
          ]
        : [{ opacity: 0 }, { opacity: 1 }],
    ]);
  }
  stage.append(layer);
  await decoded(layer);
  await Promise.all(
    moves.map(([el, frames]) => el.animate(frames, options).finished.catch(() => undefined)),
  );
  layer.remove();
}

export type Part = NonNullable<FolioFrame["parts"]>[number];

/** Parts moved by `origin`: an overlay's, from its frame's corner to the screen's. */
export function shifted(parts: Part[] | undefined, origin: Point): Part[] | undefined {
  return parts?.map((part) => ({
    ...part,
    layer: {
      ...part.layer,
      bounds: {
        ...part.layer.bounds,
        x: part.layer.bounds.x + origin.x,
        y: part.layer.bounds.y + origin.y,
      },
    },
    matrix: part.matrix && [
      part.matrix[0],
      part.matrix[1],
      part.matrix[2],
      part.matrix[3],
      part.matrix[4] + origin.x,
      part.matrix[5] + origin.y,
    ],
  }));
}

/** A part's element: its image, or a plain box, at the frame's origin, moved by `transform`. */
function partElement(
  layer: HTMLElement,
  part: Part,
  layerUrl: (layer: Layer) => string,
): HTMLElement {
  let el: HTMLElement;
  if (part.shape) {
    el = document.createElement("div");
    Object.assign(el.style, boxFrame(part));
  } else {
    const img = document.createElement("img");
    img.src = layerUrl(part.layer);
    img.alt = "";
    el = img;
  }
  Object.assign(el.style, {
    left: "0px",
    top: "0px",
    width: `${part.layer.bounds.width}px`,
    height: `${part.layer.bounds.height}px`,
    transformOrigin: "0 0",
    transform: partTransform(part),
  });
  layer.append(el);
  return el;
}

/**
 * Where a part goes, as a CSS transform from the frame's origin: its matrix (turned or flipped)
 * or its position; scaled to `as`'s size when given, so its image fills `as`'s place.
 */
function partTransform(part: Part, as?: Part): string {
  const [a, b, c, d, e, f] = part.matrix ?? [1, 0, 0, 1, part.layer.bounds.x, part.layer.bounds.y];
  const matrix = `matrix(${a}, ${b}, ${c}, ${d}, ${e}, ${f})`;
  if (!as) return matrix;
  const sx = part.layer.bounds.width / (as.layer.bounds.width || 1);
  const sy = part.layer.bounds.height / (as.layer.bounds.height || 1);
  return `${matrix} scale(${sx}, ${sy})`;
}

/** A plain box's keyframe: its size, corners, colour and place. */
function boxFrame(part: Part): Keyframe & Record<string, string> {
  return {
    width: `${part.layer.bounds.width}px`,
    height: `${part.layer.bounds.height}px`,
    borderRadius: `${part.shape?.radius ?? 0}px`,
    background: part.shape?.fill ?? "transparent",
    transform: partTransform(part),
  };
}

/** Scrolls the frame `el` to a "scroll to" action's target, smoothly unless it is instant. */
export function scrollFrame(el: HTMLElement, action: Extract<Action, { type: "scrollTo" }>): void {
  const scroller = el.querySelector(".scroll");
  if (!scroller || !action.target) return;
  const reduced = prefersReducedMotion();
  const instant = reduced || action.transition.kind.type === "instant";
  scroller.scrollTo({
    left: action.target.x - (action.offset?.x ?? 0),
    top: action.target.y - (action.offset?.y ?? 0),
    behavior: instant ? "instant" : "smooth",
  });
}

/** Resolves once every image of `el` is decoded, so a frame never shows half-loaded. */
export async function decoded(el: HTMLElement): Promise<void> {
  await Promise.all(
    [...el.querySelectorAll("img")].map((img) => img.decode().catch(() => undefined)),
  );
}

/** Plays `transition` from `outgoing` to `incoming`, both already in the stage. */
export async function play(
  outgoing: HTMLElement | undefined,
  incoming: HTMLElement,
  transition: Transition | undefined,
  reverse: boolean,
): Promise<void> {
  const reduced = prefersReducedMotion();
  if (
    !outgoing ||
    !transition ||
    reduced ||
    transition.durationMs === 0 ||
    transition.kind.type === "instant"
  )
    return;
  const options: KeyframeAnimationOptions = {
    ...timing(transition),
    fill: "both",
  };
  const kind = transition.kind;
  const animations: Animation[] = [];
  const slide = (side: Side, sign: number) => offset(side, sign);
  // Slides move the screen underneath a little too, the other way; moves leave it in place.
  const parallax = kind.type === "slideIn" || kind.type === "slideOut" ? PARALLAX : 0;
  switch (kind.type) {
    case "slideIn":
    case "moveIn":
      // Back from a slide-in: the current screen slides out the way the other came in.
      if (reverse) {
        outgoing.style.zIndex = "1";
        animations.push(
          outgoing.animate([{ transform: "none" }, { transform: slide(kind.from, 1) }], options),
        );
        if (parallax)
          animations.push(
            incoming.animate(
              [{ transform: slide(kind.from, -parallax) }, { transform: "none" }],
              options,
            ),
          );
      } else {
        animations.push(
          incoming.animate([{ transform: slide(kind.from, 1) }, { transform: "none" }], options),
        );
        if (parallax)
          animations.push(
            outgoing.animate(
              [{ transform: "none" }, { transform: slide(kind.from, -parallax) }],
              options,
            ),
          );
      }
      break;
    case "push": {
      const sign = reverse ? -1 : 1;
      animations.push(
        incoming.animate([{ transform: slide(kind.from, sign) }, { transform: "none" }], options),
        outgoing.animate([{ transform: "none" }, { transform: slide(kind.from, -sign) }], options),
      );
      break;
    }
    case "slideOut":
    case "moveOut":
      outgoing.style.zIndex = "1";
      animations.push(
        outgoing.animate([{ transform: "none" }, { transform: slide(kind.to, 1) }], options),
      );
      if (parallax)
        animations.push(
          incoming.animate(
            [{ transform: slide(kind.to, -parallax) }, { transform: "none" }],
            options,
          ),
        );
      break;
    default:
      // Dissolve; smart animate and unknown kinds fall back to it until they are supported.
      animations.push(incoming.animate([{ opacity: 0 }, { opacity: 1 }], options));
  }
  await Promise.all(animations.map((a) => a.finished.catch(() => undefined)));
}

function place(
  el: HTMLElement,
  box: { x: number; y: number; width: number; height: number },
): void {
  Object.assign(el.style, {
    left: `${box.x}px`,
    top: `${box.y}px`,
    width: `${box.width}px`,
    height: `${box.height}px`,
  });
}

/** How far a slide moves the screen underneath, as a share of the screen. */
const PARALLAX = 0.3;

/** Where a screen sits off-stage on `side`, `sign` screens away (negative: the other side). */
function offset(side: Side, sign: number): string {
  const axis = side === "left" || side === "right" ? "X" : "Y";
  const direction = side === "left" || side === "top" ? -1 : 1;
  return `translate${axis}(${direction * sign * 100}%)`;
}

/** Duration and CSS easing of a transition; a spring lasts as long as it takes to settle. */
function timing(transition: Transition): { duration: number; easing: string } {
  const curve = transition.easing;
  if (curve.type === "spring") {
    const motion = spring(curve);
    return {
      duration: motion.durationMs,
      easing: toCssLinear((t) => motion.at(t * motion.durationMs)),
    };
  }
  return { duration: transition.durationMs, easing: easing(transition) };
}

function easing(transition: Transition): string {
  const e = transition.easing;
  switch (e.type) {
    case "linear":
      return "linear";
    case "easeIn":
      return "ease-in";
    case "easeInOut":
      return "ease-in-out";
    case "cubicBezier":
      return `cubic-bezier(${e.points.join(", ")})`;
    default:
      return "ease-out";
  }
}
