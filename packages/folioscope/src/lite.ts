// SPDX-License-Identifier: AGPL-3.0-or-later
import type { Action, FolioFrame, Layer, Side, Transition } from "./folio.ts";

/** Builds a frame: its layers as images, its hotspots as transparent, focusable buttons. */
export function renderFrame(
  frame: FolioFrame,
  layerUrl: (layer: Layer) => string,
  onAction: (action: Action) => void,
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
    const clicks = hotspot.interactions.filter((i) => i.trigger.type === "click");
    if (clicks.length === 0) continue;
    const button = document.createElement("button");
    button.className = "hotspot";
    button.type = "button";
    button.setAttribute("aria-label", hotspot.name);
    place(button, hotspot.bounds);
    button.addEventListener("click", (event) => {
      event.stopPropagation();
      for (const { action } of clicks) onAction(action);
    });
    el.append(button);
  }
  return el;
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
