// SPDX-License-Identifier: AGPL-3.0-or-later

/** `index.json` of a `.folio` folder, as `folioscope bake` writes it (ADR 0004, ADR 0008). */
export interface FolioIndex {
  folio: number;
  irVersion: number;
  densities: number[];
  source: "fig" | "sketch" | "xd" | "penpot";
  pages: FolioPage[];
}

export interface FolioPage {
  id: string;
  name: string;
  flows: Flow[];
  frames: FolioFrame[];
}

export interface Flow {
  name: string;
  description?: string;
  start: string;
}

export interface Rect {
  x: number;
  y: number;
  width: number;
  height: number;
}

export interface FolioFrame {
  id: string;
  name: string;
  bounds: Rect;
  hotspots: Hotspot[];
  layers: Layer[];
  /** ThumbHash, base64: a blurred preview shown until the layers load. */
  placeholder?: string;
  /** How the frame shows when opened as an overlay. */
  overlay?: OverlaySettings;
  /** Set when the frame's content scrolls past its bounds. */
  scroll?: {
    direction: "vertical" | "horizontal" | "both";
    /** What scrolls, from the frame's top-left corner. */
    content: Rect;
    fixed?: string[];
  };
  /** Interactive component instances, whose variants change in place. */
  components?: FolioComponent[];
  /** For smart animate: the background (named `""`), then each direct child, by name. */
  parts?: { name: string; layer: Layer }[];
}

/** An instance switching between variants ("change to"), each a state with its own layer. */
export interface FolioComponent {
  /** Ids of the instances from the frame down to this one, joined by `;`. */
  id: string;
  name: string;
  /** In the frame's coordinates. */
  bounds: Rect;
  /** The variant shown first. */
  initial: string;
  fixed?: boolean;
  states: {
    variant: string;
    name: string;
    /** From the component's top-left corner. */
    bounds: Rect;
    /** In the state's coordinates. */
    hotspots: Hotspot[];
    layer: Layer;
  }[];
}

export interface OverlaySettings {
  position:
    | "center"
    | "topLeft"
    | "topCenter"
    | "topRight"
    | "bottomLeft"
    | "bottomCenter"
    | "bottomRight"
    | "manual";
  closeOnClickOutside: boolean;
  /** CSS `#rrggbbaa`. */
  backdrop?: string;
}

export interface Point {
  x: number;
  y: number;
}

/** An image drawn at `bounds`; its files are `layers/<src>@<density>x.<format>`. */
export interface Layer {
  src: string;
  /** File extension: `webp`, or `avif` for large, photo-like layers. */
  format: "webp" | "avif";
  bounds: Rect;
  /** The frame's scrolling content: moves with the scroll, under the layers that follow. */
  scrolls?: boolean;
}

export interface Hotspot {
  node: string;
  name: string;
  bounds: Rect;
  interactions: Interaction[];
  /** In a fixed child of a scrolling frame: stays in place while the content scrolls. */
  fixed?: boolean;
}

export interface Interaction {
  trigger: Trigger;
  action: Action;
}

export type Trigger =
  | { type: "click" | "hover" | "press" | "mouseEnter" | "mouseLeave" | "mouseDown" | "mouseUp" }
  | { type: "afterDelay"; delayMs: number }
  | { type: "drag" | "key" }
  | { type: "other"; name: string };

export type Action =
  | { type: "navigate"; to: string; transition: Transition }
  /** Switches the hotspot's interactive component to another of its variants. */
  | { type: "changeTo"; to: string; transition: Transition }
  | {
      type: "scrollTo";
      to: string;
      transition: Transition;
      /** The node scrolled to, in the frame's coordinates. */
      target?: Rect;
      /** Stops this far before the target. */
      offset?: Point;
    }
  | {
      type: "overlay" | "swapOverlay";
      to: string;
      transition: Transition;
      /** For a `manual` overlay: its top-left corner, from the hotspot's. */
      offset?: Point;
    }
  | { type: "back" | "close" }
  | { type: "openUrl"; url: string; newTab?: boolean }
  | { type: "unsupported"; name: string };

export type Side = "left" | "right" | "top" | "bottom";

export interface Transition {
  kind:
    | { type: "instant" | "dissolve" | "smartAnimate" }
    | { type: "slideIn" | "push" | "moveIn"; from: Side }
    | { type: "slideOut" | "moveOut"; to: Side }
    | { type: "other"; name: string };
  durationMs: number;
  easing:
    | { type: "linear" | "easeIn" | "easeOut" | "easeInOut" }
    | { type: "cubicBezier"; points: [number, number, number, number] }
    /** Lasts as long as the spring takes to settle, whatever `durationMs` says. */
    | { type: "spring"; mass: number; stiffness: number; damping: number }
    | { type: "other"; name: string };
}

/** Major versions this player reads. */
export const SUPPORTED_FOLIO = 1;
export const SUPPORTED_IR = 1;
