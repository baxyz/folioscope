// SPDX-License-Identifier: AGPL-3.0-or-later
import type { Action, FolioFrame, Layer, Rect, Side, Transition, Trigger } from "./folio.ts";

/** A hold lasts while the pointer hovers its hotspot, or until the press is released. */
export type HoldKind = "hover" | "press";

export interface FrameHandlers {
  /** Runs an action: a click, a timer, a pointer entering or leaving. */
  action: (action: Action) => void;
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
  for (const layer of frame.layers) {
    const img = document.createElement("img");
    img.src = layerUrl(layer);
    img.alt = "";
    img.draggable = false;
    place(img, layer.bounds);
    el.append(img);
  }
  for (const hotspot of frame.hotspots) {
    const by = (type: Trigger["type"]) =>
      hotspot.interactions.filter((i) => i.trigger.type === type).map((i) => i.action);
    const clicks = by("click");
    const pointer = POINTER_TRIGGERS.some((type) => by(type).length > 0);
    if (clicks.length === 0 && !pointer) continue;
    const area: HTMLElement = document.createElement(clicks.length > 0 ? "button" : "div");
    area.className = "hotspot";
    place(area, hotspot.bounds);
    if (area instanceof HTMLButtonElement) {
      area.type = "button";
      area.setAttribute("aria-label", hotspot.name);
      area.addEventListener("click", (event) => {
        event.stopPropagation();
        clicks.forEach(on.action);
      });
    }
    if (pointer) {
      const run = (actions: Action[]) => actions.forEach(on.action);
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
    el.append(area);
  }
  return el;
}

const POINTER_TRIGGERS = [
  "hover",
  "press",
  "mouseEnter",
  "mouseLeave",
  "mouseDown",
  "mouseUp",
] as const;

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
  const reduced = globalThis.matchMedia?.("(prefers-reduced-motion: reduce)").matches;
  if (
    !outgoing ||
    !transition ||
    reduced ||
    transition.durationMs === 0 ||
    transition.kind.type === "instant"
  )
    return;
  const timing: KeyframeAnimationOptions = {
    duration: transition.durationMs,
    easing: easing(transition),
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
          outgoing.animate([{ transform: "none" }, { transform: slide(kind.from, 1) }], timing),
        );
      } else {
        animations.push(
          incoming.animate([{ transform: slide(kind.from, 1) }, { transform: "none" }], timing),
        );
      }
      break;
    case "push": {
      const sign = reverse ? -1 : 1;
      animations.push(
        incoming.animate([{ transform: slide(kind.from, sign) }, { transform: "none" }], timing),
        outgoing.animate(
          [{ transform: "none" }, { transform: slide(kind.from, -sign as 1 | -1) }],
          timing,
        ),
      );
      break;
    }
    case "slideOut":
    case "moveOut":
      outgoing.style.zIndex = "1";
      animations.push(
        outgoing.animate([{ transform: "none" }, { transform: slide(kind.to, 1) }], timing),
      );
      break;
    default:
      // Dissolve; smart animate and unknown kinds fall back to it until they are supported.
      animations.push(incoming.animate([{ opacity: 0 }, { opacity: 1 }], timing));
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
