// SPDX-License-Identifier: AGPL-3.0-or-later
import { spring, toCssLinear } from "helpers4/animation";
import { prefersReducedMotion } from "helpers4/browser";
import { matrixToCss, rectToRect } from "helpers4/geometry";
import type {
  Action,
  FolioComponent,
  FolioFrame,
  Hotspot,
  Layer,
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
  // then fixed layers and hotspots on top.
  let content: HTMLElement | undefined;
  for (const layer of frame.layers) {
    const img = document.createElement("img");
    img.src = layerUrl(layer);
    img.alt = "";
    img.draggable = false;
    place(img, layer.bounds);
    if (layer.scrolls && frame.scroll) {
      const scroller = document.createElement("div");
      scroller.className = "scroll";
      const { direction } = frame.scroll;
      scroller.style.overflowX = direction === "vertical" ? "hidden" : "auto";
      scroller.style.overflowY = direction === "horizontal" ? "hidden" : "auto";
      content = document.createElement("div");
      content.className = "content";
      content.style.width = `${frame.scroll.content.width}px`;
      content.style.height = `${frame.scroll.content.height}px`;
      content.append(img);
      scroller.append(content);
      el.append(scroller);
    } else {
      el.append(img);
    }
  }
  for (const hotspot of frame.hotspots) {
    const area = renderHotspot(hotspot, on);
    if (area) (hotspot.fixed ? el : (content ?? el)).append(area);
  }
  // Components go last: they sit above the frame's own hotspots, as in the design file.
  for (const component of frame.components ?? []) {
    const zone = renderComponent(component, layerUrl, on);
    (component.fixed ? el : (content ?? el)).append(zone);
  }
  return el;
}

/**
 * A hotspot as a transparent element: a focusable button when it can be clicked, a plain area
 * when it only reacts to the pointer; `undefined` when nothing in it is for the player.
 */
function renderHotspot(hotspot: Hotspot, on: FrameHandlers): HTMLElement | undefined {
  const by = (type: Trigger["type"]) =>
    hotspot.interactions.filter((i) => i.trigger.type === type).map((i) => i.action);
  const clicks = by("click");
  const pointer = POINTER_TRIGGERS.some((type) => by(type).length > 0);
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
  const show = (variant: string, transition?: Transition) => {
    const state = component.states.find((s) => s.variant === variant);
    if (!state) return;
    current = variant;
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
    zone.replaceChildren(...(before ? [before] : []), img, ...areas);
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
    // Smart animate and the rest fade for now.
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
 * Smart animate (ADR 0003): over `stage`, the parts of `from` and `to` paired by name move and
 * scale from their bounds in one frame to those in the other, the second image fading in over the
 * first; parts on one side only fade out or in. Resolves when it is over; does nothing when a
 * frame has no parts or motion is reduced.
 */
export async function playSmart(
  stage: HTMLElement,
  from: FolioFrame,
  to: FolioFrame,
  transition: Transition,
  layerUrl: (layer: Layer) => string,
): Promise<void> {
  if (!from.parts?.length || !to.parts?.length || prefersReducedMotion()) return;
  const options: KeyframeAnimationOptions = { ...timing(transition), fill: "both" };
  const layer = document.createElement("div");
  layer.className = "smart";
  const image = (bounds: Rect, src: Layer) => {
    const img = document.createElement("img");
    img.src = layerUrl(src);
    img.alt = "";
    place(img, bounds);
    layer.append(img);
    return img;
  };
  // Where `a` would be drawn at `b`'s place, from `a`'s own (transform-origin: 0 0).
  const toward = (a: Rect, b: Rect) => matrixToCss(rectToRect(a, b));
  // Pairs by name, in order: the n-th part named X in `from` with the n-th one in `to`.
  const pairs = new Map<number, number>();
  for (const [i, part] of from.parts.entries()) {
    const j = to.parts.findIndex(
      (other, k) => other.name === part.name && ![...pairs.values()].includes(k),
    );
    if (j >= 0) pairs.set(i, j);
  }
  const paired = new Set(pairs.values());
  const moves: [HTMLElement, Keyframe[]][] = [];
  for (const [i, part] of from.parts.entries()) {
    const j = pairs.get(i);
    const end = j === undefined ? undefined : to.parts[j]!.layer.bounds;
    const img = image(part.layer.bounds, part.layer);
    moves.push([
      img,
      // A paired part stays opaque under its partner fading in: a cross-fade of two half
      // transparent images would show what is behind them.
      end
        ? [{ transform: "none" }, { transform: toward(part.layer.bounds, end) }]
        : [{ opacity: 1 }, { opacity: 0 }],
    ]);
  }
  for (const [j, part] of to.parts.entries()) {
    const i = [...pairs].find(([, k]) => k === j)?.[0];
    const start = paired.has(j) && i !== undefined ? from.parts[i]!.layer.bounds : undefined;
    const img = image(part.layer.bounds, part.layer);
    moves.push([
      img,
      start
        ? [
            { transform: toward(part.layer.bounds, start), opacity: 0 },
            { transform: "none", opacity: 1 },
          ]
        : [{ opacity: 0 }, { opacity: 1 }],
    ]);
  }
  stage.append(layer);
  await decoded(layer);
  await Promise.all(
    moves.map(([img, frames]) => img.animate(frames, options).finished.catch(() => undefined)),
  );
  layer.remove();
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
  const slide = (side: Side, sign: 1 | -1) => offset(side, sign);
  switch (kind.type) {
    case "slideIn":
    case "moveIn":
      // Back from a slide-in: the current screen slides out the way the other came in.
      if (reverse) {
        outgoing.style.zIndex = "1";
        animations.push(
          outgoing.animate([{ transform: "none" }, { transform: slide(kind.from, 1) }], options),
        );
      } else {
        animations.push(
          incoming.animate([{ transform: slide(kind.from, 1) }, { transform: "none" }], options),
        );
      }
      break;
    case "push": {
      const sign = reverse ? -1 : 1;
      animations.push(
        incoming.animate([{ transform: slide(kind.from, sign) }, { transform: "none" }], options),
        outgoing.animate(
          [{ transform: "none" }, { transform: slide(kind.from, -sign as 1 | -1) }],
          options,
        ),
      );
      break;
    }
    case "slideOut":
    case "moveOut":
      outgoing.style.zIndex = "1";
      animations.push(
        outgoing.animate([{ transform: "none" }, { transform: slide(kind.to, 1) }], options),
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

/** Where a screen sits off-stage on `side`. */
function offset(side: Side, sign: 1 | -1): string {
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
