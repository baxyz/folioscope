// SPDX-License-Identifier: AGPL-3.0-or-later
import { constants, zstdCompressSync } from "node:zlib";
import { zipSync } from "fflate";
import { solidPng } from "./png.ts";
import {
  assembleCanvasFig,
  createEmptyFigDoc,
  encodeFigParts,
  hexToFigColor,
  type FigDocument,
  type FigGuid,
  type FigNode,
} from "openfig-core";

export interface Box {
  x: number;
  y: number;
  width: number;
  height: number;
}

export type Trigger = "ON_CLICK" | "ON_HOVER" | "ON_PRESS" | "AFTER_TIMEOUT";

export type Action =
  | { kind: "navigate"; to: FigGuid; transition?: Transition }
  | { kind: "overlay"; to: FigGuid; transition?: Transition }
  | { kind: "scrollTo"; to: FigGuid; transition?: Transition }
  | { kind: "back" }
  | { kind: "close" };

export interface OverlaySettings {
  position: "CENTER" | "TOP_CENTER" | "BOTTOM_CENTER";
  closeOnClickOutside: boolean;
  /** Backdrop color, e.g. `#00000066`; none when absent. */
  backdrop?: string;
}

export interface Transition {
  type:
    | "INSTANT_TRANSITION"
    | "DISSOLVE"
    | "SLIDE_FROM_RIGHT"
    | "PUSH_FROM_RIGHT"
    | "MOVE_FROM_BOTTOM"
    | "SMART_ANIMATE"
    | "SCROLL_ANIMATE";
  durationMs: number;
  easing?: "LINEAR" | "IN_CUBIC" | "OUT_CUBIC" | "INOUT_CUBIC";
}

const FIXED_DATE = new Date("2026-01-01T00:00:00Z");

// Sibling order keys, compared as strings by Figma: "!", '"', "#", …
function position(index: number): string {
  if (index > 0x7d - 0x21) throw new Error("fixture parent has too many children");
  return String.fromCharCode(0x21 + index);
}

/** Builds a `.fig` file node by node, on top of openfig-core's empty document. */
export class FigmaBuilder {
  readonly doc: FigDocument = createEmptyFigDoc();
  private nextLocalId = 1;
  private pageNamed = false;
  private readonly childCount = new Map<string, number>();

  private guid(): FigGuid {
    return { sessionID: 1, localID: this.nextLocalId++ };
  }

  private add(parent: FigGuid, node: Omit<FigNode, "guid" | "parentIndex">): FigGuid {
    const key = `${parent.sessionID}:${parent.localID}`;
    const index = this.childCount.get(key) ?? 0;
    this.childCount.set(key, index + 1);
    const guid = this.guid();
    this.doc.message.nodeChanges.push({
      guid,
      phase: "CREATED",
      parentIndex: { guid: parent, position: position(index) },
      visible: true,
      opacity: 1,
      ...node,
    });
    return guid;
  }

  /** Renames the empty document's page on the first call, then adds pages after it. */
  page(name: string): FigGuid {
    const canvases = this.doc.message.nodeChanges.filter(
      (n: FigNode) => n.type === "CANVAS" && !n.internalOnly,
    );
    const first = canvases[0];
    if (!this.pageNamed) {
      this.pageNamed = true;
      first.name = name;
      return first.guid;
    }
    const guid = this.guid();
    // The first page sits at "a" and the internal canvas at "b": new pages go in between.
    this.doc.message.nodeChanges.push({
      ...structuredClone(first),
      guid,
      name,
      parentIndex: { ...first.parentIndex, position: `a${position(canvases.length - 1)}` },
    });
    return guid;
  }

  frame(parent: FigGuid, name: string, box: Box, fill = "#FFFFFF"): FigGuid {
    return this.add(parent, {
      type: "FRAME",
      name,
      size: { x: box.width, y: box.height },
      transform: { m00: 1, m01: 0, m02: box.x, m10: 0, m11: 1, m12: box.y },
      fillPaints: [solid(fill)],
      frameMaskDisabled: false,
    });
  }

  /** The device a page's prototype is presented in, as Figma's presets name it. */
  device(page: FigGuid, preset: string, width: number, height: number): void {
    this.node(page).prototypeDevice = {
      type: "PRESET",
      size: { x: width, y: height },
      presetIdentifier: preset,
      rotation: "NONE",
    };
  }

  /** Lets a frame scroll; its content is whatever overflows its size. */
  scrollable(frame: FigGuid, direction: "VERTICAL" | "HORIZONTAL" | "BOTH" = "VERTICAL"): void {
    this.node(frame).scrollDirection = direction;
  }

  /** Keeps a child of a scrolling frame in place while the rest scrolls. */
  fixed(node: FigGuid): void {
    this.node(node).scrollBehavior = "FIXED_WHEN_CHILD_OF_SCROLLING_FRAME";
  }

  /** How a frame shows when it is opened as an overlay. */
  overlay(frame: FigGuid, settings: OverlaySettings): void {
    const node = this.node(frame);
    node.overlayPositionType = settings.position;
    node.overlayBackgroundInteraction = settings.closeOnClickOutside
      ? "CLOSE_ON_CLICK_OUTSIDE"
      : "NONE";
    node.overlayBackgroundAppearance = settings.backdrop
      ? { backgroundType: "SOLID_COLOR", backgroundColor: hexToFigColor(settings.backdrop) }
      : { backgroundType: "NONE" };
  }

  rect(parent: FigGuid, name: string, box: Box, fill: string, radius = 0): FigGuid {
    return this.add(parent, {
      type: "ROUNDED_RECTANGLE",
      name,
      size: { x: box.width, y: box.height },
      transform: { m00: 1, m01: 0, m02: box.x, m10: 0, m11: 1, m12: box.y },
      fillPaints: [solid(fill)],
      strokeWeight: 0,
      // Figma drops zero radii when it saves.
      ...(radius
        ? {
            cornerRadius: radius,
            rectangleTopLeftCornerRadius: radius,
            rectangleTopRightCornerRadius: radius,
            rectangleBottomLeftCornerRadius: radius,
            rectangleBottomRightCornerRadius: radius,
          }
        : {}),
    });
  }

  /** Marks a top-level frame as the start of a prototype flow. */
  flowStart(frame: FigGuid, name: string, description = ""): void {
    const flows = this.doc.message.nodeChanges.filter((n: FigNode) => n.prototypeStartingPoint);
    this.node(frame).prototypeStartingPoint = {
      name,
      description,
      position: position(flows.length),
    };
  }

  on(node: FigGuid, trigger: Trigger, action: Action, timeoutMs = 0): void {
    const target = this.node(node);
    target.prototypeInteractions ??= [];
    target.prototypeInteractions.push({
      id: this.guid(),
      event: {
        interactionType: trigger,
        ...(trigger === "AFTER_TIMEOUT" ? { transitionTimeout: timeoutMs / 1000 } : {}),
      },
      actions: [prototypeAction(action)],
      isDeleted: false,
    });
  }

  /** Encodes the document as a `.fig` archive. */
  toBytes(): Uint8Array {
    const parts = encodeFigParts(this.doc);
    // Figma rejects a message chunk that is not zstd-compressed.
    const messageCompressed = zstdCompressSync(parts.messageRaw, {
      params: { [constants.ZSTD_c_compressionLevel]: 3 },
    });
    const canvasFig = assembleCanvasFig({ ...parts, messageCompressed });
    const { meta, thumbnail } = this.metaAndThumbnail();
    // Figma rejects an archive without a thumbnail. Fixed dates keep regeneration a no-op.
    const stored = { level: 0, mtime: FIXED_DATE } as const;
    return zipSync({
      "canvas.fig": [canvasFig, stored],
      "thumbnail.png": [thumbnail, stored],
      "meta.json": [new TextEncoder().encode(JSON.stringify(meta)), stored],
    });
  }

  /** `meta.json` as Figma writes it, and a plain thumbnail of the first page's bounds. */
  metaAndThumbnail(): { meta: object; thumbnail: Uint8Array } {
    const page = this.doc.message.nodeChanges.find((n: FigNode) => n.type === "CANVAS");
    const boxes: Box[] = this.doc.message.nodeChanges
      .filter(
        (n: FigNode) =>
          n.parentIndex?.guid.sessionID === page.guid.sessionID &&
          n.parentIndex?.guid.localID === page.guid.localID,
      )
      .map((f: FigNode) => {
        const x = f.transform?.m02 ?? 0;
        const y = f.transform?.m12 ?? 0;
        return { x, y, width: f.size?.x ?? 0, height: f.size?.y ?? 0 };
      });
    const bounds = boxes.length
      ? {
          x: Math.min(...boxes.map((b) => b.x)),
          y: Math.min(...boxes.map((b) => b.y)),
          right: Math.max(...boxes.map((b) => b.x + b.width)),
          bottom: Math.max(...boxes.map((b) => b.y + b.height)),
        }
      : { x: 0, y: 0, right: 100, bottom: 100 };
    const width = bounds.right - bounds.x;
    const height = bounds.bottom - bounds.y;
    const scale = Math.min(1, 400 / width);
    const thumbnailSize = {
      width: Math.max(1, Math.round(width * scale)),
      height: Math.max(1, Math.round(height * scale)),
    };
    const background = page.backgroundColor ?? { r: 0.96, g: 0.96, b: 0.96, a: 1 };
    return {
      meta: {
        client_meta: {
          background_color: background,
          thumbnail_size: thumbnailSize,
          render_coordinates: { x: bounds.x, y: bounds.y, width, height },
        },
        file_name: page.name,
        developer_related_links: [],
        exported_at: FIXED_DATE.toISOString(),
      },
      thumbnail: solidPng(thumbnailSize.width, thumbnailSize.height, background),
    };
  }

  private node(guid: FigGuid): FigNode {
    const node = this.doc.message.nodeChanges.find(
      (n: FigNode) => n.guid.sessionID === guid.sessionID && n.guid.localID === guid.localID,
    );
    if (!node) throw new Error(`no node ${guid.sessionID}:${guid.localID}`);
    return node;
  }
}

function solid(hex: string) {
  return {
    type: "SOLID",
    color: hexToFigColor(hex),
    opacity: 1,
    visible: true,
    blendMode: "NORMAL",
  };
}

function prototypeAction(action: Action) {
  switch (action.kind) {
    case "navigate":
    case "overlay":
    case "scrollTo":
      return {
        connectionType: "INTERNAL_NODE",
        transitionNodeID: action.to,
        navigationType: { navigate: "NAVIGATE", overlay: "OVERLAY", scrollTo: "SCROLL_TO" }[
          action.kind
        ],
        ...transition(action.transition),
      };
    case "back":
      return { connectionType: "BACK" };
    case "close":
      return { connectionType: "CLOSE" };
  }
}

function transition(t: Transition = { type: "INSTANT_TRANSITION", durationMs: 0 }) {
  return {
    transitionType: t.type,
    transitionDuration: t.durationMs / 1000,
    easingType: t.easing ?? "OUT_CUBIC",
    transitionShouldSmartAnimate: t.type === "SMART_ANIMATE",
  };
}
