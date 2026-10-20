// SPDX-License-Identifier: AGPL-3.0-or-later
import { zstdCompressSync, constants } from "node:zlib";
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
    // Same layout as openfig-core's createFigZip, with a fixed date so regenerating is a no-op.
    const stored = { level: 0, mtime: FIXED_DATE } as const;
    return zipSync({
      "canvas.fig": [canvasFig, stored],
      "meta.json": [new TextEncoder().encode(JSON.stringify(this.doc.meta)), stored],
    });
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
