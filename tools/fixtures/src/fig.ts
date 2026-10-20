// SPDX-License-Identifier: AGPL-3.0-or-later
import { constants, crc32, deflateSync, zstdCompressSync } from "node:zlib";
import { zipSync } from "fflate";
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
  | { kind: "back" }
  | { kind: "close" };

export interface Transition {
  type:
    | "INSTANT_TRANSITION"
    | "DISSOLVE"
    | "SLIDE_FROM_RIGHT"
    | "PUSH_FROM_RIGHT"
    | "SMART_ANIMATE";
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
export class FigBuilder {
  readonly doc: FigDocument = createEmptyFigDoc();
  private nextLocalId = 1;
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

  /** The first page of the empty document, renamed. */
  page(name: string): FigGuid {
    const canvas = this.doc.message.nodeChanges.find((n: FigNode) => n.type === "CANVAS");
    canvas.name = name;
    return canvas.guid;
  }

  frame(parent: FigGuid, name: string, box: Box, fill = "#FFFFFF"): FigGuid {
    return this.add(parent, {
      type: "FRAME",
      name,
      size: { x: box.width, y: box.height },
      transform: { m00: 1, m01: 0, m02: box.x, m10: 0, m11: 1, m12: box.y },
      fillPaints: [solid(fill)],
    });
  }

  rect(parent: FigGuid, name: string, box: Box, fill: string, radius = 0): FigGuid {
    return this.add(parent, {
      type: "ROUNDED_RECTANGLE",
      name,
      size: { x: box.width, y: box.height },
      transform: { m00: 1, m01: 0, m02: box.x, m10: 0, m11: 1, m12: box.y },
      fillPaints: [solid(fill)],
      strokeWeight: 0,
      cornerRadius: radius,
      rectangleTopLeftCornerRadius: radius,
      rectangleTopRightCornerRadius: radius,
      rectangleBottomLeftCornerRadius: radius,
      rectangleBottomRightCornerRadius: radius,
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
      return {
        connectionType: "INTERNAL_NODE",
        transitionNodeID: action.to,
        navigationType: action.kind === "navigate" ? "NAVIGATE" : "OVERLAY",
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

/** A PNG of a single color: 8-bit RGB, unfiltered rows. */
function solidPng(width: number, height: number, color: { r: number; g: number; b: number }) {
  const rgb = [color.r, color.g, color.b].map((c) => Math.round(c * 255));
  const row = Buffer.alloc(1 + width * 3);
  for (let x = 0; x < width; x++) row.set(rgb, 1 + x * 3);
  const raw = Buffer.concat(Array.from({ length: height }, () => row));
  const chunk = (type: string, data: Buffer) => {
    const head = Buffer.alloc(8);
    head.writeUInt32BE(data.length, 0);
    head.write(type, 4, "ascii");
    const crc = Buffer.alloc(4);
    crc.writeUInt32BE(crc32(Buffer.concat([head.subarray(4), data])), 0);
    return Buffer.concat([head, data, crc]);
  };
  const ihdr = Buffer.alloc(13);
  ihdr.writeUInt32BE(width, 0);
  ihdr.writeUInt32BE(height, 4);
  ihdr.set([8, 2, 0, 0, 0], 8);
  return new Uint8Array(
    Buffer.concat([
      Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]),
      chunk("IHDR", ihdr),
      chunk("IDAT", deflateSync(raw)),
      chunk("IEND", Buffer.alloc(0)),
    ]),
  );
}
