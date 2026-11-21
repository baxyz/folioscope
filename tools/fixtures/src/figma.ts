// SPDX-License-Identifier: AGPL-3.0-or-later
import { createHash } from "node:crypto";
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
  type FigPaint,
} from "openfig-core";

export interface Box {
  x: number;
  y: number;
  width: number;
  height: number;
}

export type Trigger =
  | "ON_CLICK"
  | "ON_HOVER"
  | "ON_PRESS"
  | "AFTER_TIMEOUT"
  | "DRAG"
  | "ON_KEY_DOWN";

export type Action =
  | { kind: "navigate"; to: FigGuid; transition?: Transition }
  | { kind: "overlay"; to: FigGuid; transition?: Transition }
  | { kind: "scrollTo"; to: FigGuid; transition?: Transition }
  | { kind: "changeTo"; to: FigGuid; transition?: Transition }
  | { kind: "swap"; to: FigGuid; transition?: Transition }
  | { kind: "back" }
  | { kind: "close" }
  | { kind: "url"; url: string; newTab?: boolean };

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
  /** Image fills' files, by the hex SHA-1 of their bytes, as Figma names them. */
  private readonly images = new Map<string, Uint8Array>();
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

  /**
   * A set of variants: a frame Figma marks as a state group, whose children are the variants of
   * one property, in `values` order.
   */
  componentSet(
    parent: FigGuid,
    name: string,
    box: Box,
    property: string,
    values: string[],
  ): FigGuid {
    return this.add(parent, {
      type: "FRAME",
      name,
      size: { x: box.width, y: box.height },
      transform: { m00: 1, m01: 0, m02: box.x, m10: 0, m11: 1, m12: box.y },
      fillPaints: [],
      frameMaskDisabled: false,
      isStateGroup: true,
      stateGroupPropertyValueOrders: [{ property, values }],
    });
  }

  /** A variant of a set: a component named `property=value`. */
  variant(set: FigGuid, name: string, box: Box, fill: string, radius = 0): FigGuid {
    return this.add(set, {
      type: "SYMBOL",
      name,
      size: { x: box.width, y: box.height },
      transform: { m00: 1, m01: 0, m02: box.x, m10: 0, m11: 1, m12: box.y },
      fillPaints: [solid(fill)],
      frameMaskDisabled: false,
      ...(radius ? { cornerRadius: radius } : {}),
    });
  }

  /**
   * A text that grows with its content, in Inter Regular. Figma stores the outline of each glyph
   * it laid out; this one stores none, so the bake lays it out with the fonts it is given.
   */
  text(parent: FigGuid, name: string, box: Box, characters: string, fill: string, size = 16) {
    return this.add(parent, {
      type: "TEXT",
      name,
      size: { x: box.width, y: box.height },
      transform: { m00: 1, m01: 0, m02: box.x, m10: 0, m11: 1, m12: box.y },
      fillPaints: [solid(fill)],
      textData: { characters },
      fontName: { family: "Inter", style: "Regular", postscript: "" },
      fontSize: size,
      textAutoResize: "WIDTH_AND_HEIGHT",
      textAlignHorizontal: "LEFT",
    } as unknown as Omit<FigNode, "guid" | "parentIndex">);
  }

  /**
   * Lays a frame's children out in a row (auto layout), hugging them, with `padding` around
   * (horizontal, vertical) and `spacing` between.
   */
  row(frame: FigGuid, padding: [number, number], spacing = 0): void {
    Object.assign(this.node(frame), {
      stackMode: "HORIZONTAL",
      stackSpacing: spacing,
      stackHorizontalPadding: padding[0],
      stackPaddingRight: padding[0],
      stackVerticalPadding: padding[1],
      stackPaddingBottom: padding[1],
      stackPrimarySizing: "RESIZE_TO_FIT_WITH_IMPLICIT_SIZE",
      stackCounterSizing: "RESIZE_TO_FIT_WITH_IMPLICIT_SIZE",
      stackCounterAlignItems: "CENTER",
    });
  }

  /** Overrides the characters of `text`, a node of the instance's component. */
  overrideText(instance: FigGuid, text: FigGuid, characters: string): void {
    this.node(instance).symbolData.symbolOverrides.push({
      guidPath: { guids: [text] },
      textData: { characters },
    });
  }

  /**
   * An instance of `component`, with no override. Like Figma, it carries the component's own
   * look (fills, corners, auto layout): only its children come from the component.
   */
  instance(parent: FigGuid, component: FigGuid, name: string, box: Box): FigGuid {
    const source = this.node(component);
    const look = Object.fromEntries(
      Object.entries(source).filter(
        ([field]) => ["fillPaints", "cornerRadius"].includes(field) || field.startsWith("stack"),
      ),
    );
    return this.add(parent, {
      ...structuredClone(look),
      type: "INSTANCE",
      name,
      size: { x: box.width, y: box.height },
      transform: { m00: 1, m01: 0, m02: box.x, m10: 0, m11: 1, m12: box.y },
      symbolData: { symbolID: component, symbolOverrides: [] },
      frameMaskDisabled: false,
    });
  }

  /**
   * An image paint of `png` (`width` × `height` pixels): `STRETCH`, `FIT`, `FILL` or `TILE`, with
   * a tile `scale`, or a crop `transform` for `STRETCH`.
   */
  imagePaint(
    png: Uint8Array,
    width: number,
    height: number,
    mode: "STRETCH" | "FIT" | "FILL" | "TILE",
    extra: { scale?: number; transform?: object } = {},
  ) {
    const sha1 = createHash("sha1").update(png).digest();
    this.images.set(sha1.toString("hex"), png);
    return {
      type: "IMAGE",
      opacity: 1,
      visible: true,
      blendMode: "NORMAL",
      image: { hash: new Uint8Array(sha1), name: "image" },
      imageScaleMode: mode,
      originalImageWidth: width,
      originalImageHeight: height,
      ...extra,
    } as unknown as FigPaint;
  }

  /** Sets fields of a node as Figma stores them (paints, effects, blend mode…). */
  set(node: FigGuid, fields: Partial<FigNode>): void {
    Object.assign(this.node(node), fields);
  }

  /** An ellipse filling its box. */
  ellipse(parent: FigGuid, name: string, box: Box, fill: string): FigGuid {
    return this.add(parent, {
      type: "ELLIPSE",
      name,
      size: { x: box.width, y: box.height },
      transform: { m00: 1, m01: 0, m02: box.x, m10: 0, m11: 1, m12: box.y },
      fillPaints: [solid(fill)],
      strokeWeight: 0,
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

  /** An interaction; `keyCodes` (browser key codes) for `ON_KEY_DOWN`. */
  on(
    node: FigGuid,
    trigger: Trigger,
    action: Action,
    timeoutMs = 0,
    keyCodes: number[] = [],
  ): void {
    const target = this.node(node);
    target.prototypeInteractions ??= [];
    target.prototypeInteractions.push({
      id: this.guid(),
      event: {
        interactionType: trigger,
        ...(trigger === "AFTER_TIMEOUT" ? { transitionTimeout: timeoutMs / 1000 } : {}),
        ...(trigger === "ON_KEY_DOWN"
          ? { keyTrigger: { keyCodes, triggerDevice: "KEYBOARD" } }
          : {}),
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
      ...Object.fromEntries(
        [...this.images].map(([hash, png]) => [`images/${hash}`, [png, stored] as const]),
      ),
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

/** A gradient paint: `type` `GRADIENT_LINEAR`, `_RADIAL`, `_ANGULAR` or `_DIAMOND`, across the box. */
export function gradient(type: string, from: string, to: string) {
  return {
    type,
    opacity: 1,
    visible: true,
    blendMode: "NORMAL",
    // Node space (0–1) to gradient space: the identity, so the gradient spans the whole box.
    transform: { m00: 1, m01: 0, m02: 0, m10: 0, m11: 1, m12: 0 },
    stops: [
      { color: hexToFigColor(from), position: 0 },
      { color: hexToFigColor(to), position: 1 },
    ],
  };
}

/** An effect: a shadow (drop or inner) or a blur (foreground or background). */
export function effect(
  type: "DROP_SHADOW" | "INNER_SHADOW" | "FOREGROUND_BLUR" | "BACKGROUND_BLUR",
  radius: number,
  offsetY = 0,
  color = "#00000040",
) {
  return {
    type,
    visible: true,
    radius,
    spread: 0,
    offset: { x: 0, y: offsetY },
    color: hexToFigColor(color),
    blendMode: "NORMAL",
    showShadowBehindNode: false,
  };
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
    case "changeTo":
    case "swap":
      return {
        connectionType: "INTERNAL_NODE",
        transitionNodeID: action.to,
        navigationType: {
          navigate: "NAVIGATE",
          overlay: "OVERLAY",
          scrollTo: "SCROLL_TO",
          changeTo: "SWAP_STATE",
          swap: "SWAP",
        }[action.kind],
        ...transition(action.transition),
      };
    case "back":
      return { connectionType: "BACK" };
    case "close":
      return { connectionType: "CLOSE" };
    case "url":
      return {
        connectionType: "URL",
        connectionURL: action.url,
        openUrlInNewTab: action.newTab ?? true,
      };
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
