// SPDX-License-Identifier: AGPL-3.0-or-later

/** `index.json` of a `.folio` folder, as `folioscope bake` writes it (docs/spec/folio.md). */
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
  /** Frames inside the screen whose content scrolls in place. */
  scrollers?: {
    id: string;
    name: string;
    /** Where it shows, in the frame's coordinates. */
    bounds: Rect;
    direction: "vertical" | "horizontal" | "both";
    /** What scrolls, from its corner. */
    content: Rect;
    sticky?: string[];
  }[];
  /**
   * For smart animate: the background (named `""`), then each layer by its path (`Card/Title`),
   * drawn without its own parts.
   */
  parts?: {
    name: string;
    layer: Layer;
    /** `[a, b, c, d, e, f]`: where it goes in the frame, its top-left corner at the origin. */
    matrix?: [number, number, number, number, number, number];
    /** A plain rounded rectangle, animated as a box. */
    shape?: { fill: string; radius: number };
  }[];
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
    /** Interactive components inside this state, in its coordinates. */
    components?: FolioComponent[];
    /** For smart animate between states: as a frame's `parts`, in the state's coordinates. */
    parts?: FolioFrame["parts"];
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

/**
 * An image drawn at `bounds`; its files are `layers/<src>@<density>x.<format>`, or the single
 * `layers/<src>.svg` for every density.
 */
export interface Layer {
  src: string;
  /** File extension: `webp`; `avif` for large, photo-like layers; `svg` for flat ones (docs/spec/bake.md). */
  format: "webp" | "avif" | "svg";
  bounds: Rect;
  /** The frame's scrolling content: moves with the scroll, under the layers that follow. */
  scrolls?: boolean;
  /** In a frame that scrolls inside the screen, by its id; `bounds` are in its content. */
  scroller?: string;
  /** Scrolls with its content, then sticks at its scroller's top (or left) edge. */
  sticky?: boolean;
}

export interface Hotspot {
  node: string;
  name: string;
  bounds: Rect;
  interactions: Interaction[];
  /** In a fixed child of a scrolling frame: stays in place while the content scrolls. */
  fixed?: boolean;
  /** In a frame that scrolls inside the screen, by its id; `bounds` are in its content. */
  scroller?: string;
}

export interface Interaction {
  trigger: Trigger;
  action: Action;
}

export type Trigger =
  | { type: "click" | "hover" | "press" | "mouseEnter" | "mouseLeave" | "mouseDown" | "mouseUp" }
  | { type: "afterDelay"; delayMs: number }
  | { type: "drag" }
  /** Browser key codes pressed together; any key when absent. */
  | { type: "key"; codes?: number[] }
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
      /** An `overlay` that closes the overlay instead when it is already open. */
      toggle?: boolean;
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
