// SPDX-License-Identifier: AGPL-3.0-or-later
import { zipSync } from "fflate";
import type { Box } from "./figma.ts";

const FIXED_DATE = new Date("2026-01-01T00:00:00Z");

export type XdTransition = "dissolve" | "slide-left" | "slide-right" | "push-left" | "none";

export interface XdLink {
  /** `artboard-transition`, or `auto-animate` to pair the layers of both artboards by name. */
  action?: "artboard-transition" | "auto-animate";
  trigger?: "tap" | "time";
  transition?: XdTransition;
  /** Seconds. */
  duration?: number;
  easing?: "linear" | "ease-in" | "ease-out" | "ease-in-out" | "wind-up";
  /** Seconds, for a `time` trigger. */
  delay?: number;
}

interface Node {
  type: "shape" | "group" | "text";
  name: string;
  id: string;
  transform: { a: number; b: number; c: number; d: number; tx: number; ty: number };
  [key: string]: unknown;
}

interface Artboard {
  id: string;
  name: string;
  box: Box;
  background: string;
  children: Node[];
}

const rgb = (hex: string) => ({
  mode: "RGB",
  value: {
    r: Number.parseInt(hex.slice(1, 3), 16),
    g: Number.parseInt(hex.slice(3, 5), 16),
    b: Number.parseInt(hex.slice(5, 7), 16),
  },
});
const solid = (hex: string) => ({ type: "solid", color: rgb(hex) });
const json = (value: unknown) => new TextEncoder().encode(JSON.stringify(value));

/**
 * Builds an `.xd` file shaped like the ones XD 24 writes (see `docs/format-xd.md`): a manifest, one
 * graphics tree per artboard, `interactions.json`. Adobe publishes no schema, so only what the
 * importer reads is written. Ids are deterministic so regenerating is a no-op.
 */
export class XdBuilder {
  private nextId = 1;
  private readonly artboards: Artboard[] = [];
  private readonly interactions: Record<string, unknown[]> = {};
  private home?: string;
  private readonly name: string;

  constructor(name: string) {
    this.name = name;
  }

  /** UUID-shaped and deterministic. */
  private id(): string {
    return `00000000-0000-4000-8000-${(this.nextId++).toString(16).padStart(12, "0")}`;
  }

  /** The artboard the prototype starts on. */
  start(artboard: Artboard): void {
    this.home = artboard.id;
  }

  artboard(name: string, box: Box, background = "#FFFFFF"): Artboard {
    const artboard = { id: this.id(), name, box, background, children: [] };
    this.artboards.push(artboard);
    return artboard;
  }

  /** A rectangle, in the artboard's coordinates (XD stores the canvas'). */
  rect(artboard: Artboard, name: string, box: Box, fill: string, radius = 0): Node {
    return this.add(artboard, {
      type: "shape",
      name,
      id: this.id(),
      transform: { a: 1, b: 0, c: 0, d: 1, tx: artboard.box.x + box.x, ty: artboard.box.y + box.y },
      style: { fill: solid(fill), stroke: { type: "none", color: rgb("#707070"), width: 1 } },
      shape: {
        type: "rect",
        x: 0,
        y: 0,
        width: box.width,
        height: box.height,
        ...(radius ? { r: [radius, radius, radius, radius] } : {}),
      },
    });
  }

  circle(artboard: Artboard, name: string, box: Box, fill: string): Node {
    const r = Math.min(box.width, box.height) / 2;
    return this.add(artboard, {
      type: "shape",
      name,
      id: this.id(),
      transform: { a: 1, b: 0, c: 0, d: 1, tx: artboard.box.x + box.x, ty: artboard.box.y + box.y },
      style: { fill: solid(fill), stroke: { type: "none", color: rgb("#707070"), width: 1 } },
      shape: { type: "circle", cx: r, cy: r, r },
    });
  }

  /**
   * Text laid out the way XD stores it: one line at a baseline, with the font by family and style.
   * The text is drawn from the baseline `y` of the node's origin.
   */
  text(
    artboard: Artboard,
    name: string,
    at: { x: number; y: number },
    text: string,
    font: { family: string; style: string; size: number },
    fill = "#111111",
  ): Node {
    const postscript = `${font.family}-${font.style}`.replaceAll(" ", "");
    return this.add(artboard, {
      type: "text",
      name,
      id: this.id(),
      transform: { a: 1, b: 0, c: 0, d: 1, tx: artboard.box.x + at.x, ty: artboard.box.y + at.y },
      style: {
        fill: solid(fill),
        font: { postscriptName: postscript, ...font },
      },
      text: {
        frame: { type: "positioned" },
        paragraphs: [{ lines: [[{ x: 0, y: 0, from: 0, to: text.length }]] }],
        rawText: text,
      },
      meta: {
        ux: {
          singleTextObject: true,
          rangedStyles: [
            {
              length: text.length,
              fontFamily: font.family,
              fontStyle: font.style,
              postscriptName: postscript,
              fontSize: font.size,
              charSpacing: 0,
            },
          ],
        },
      },
    });
  }

  /** Makes `node` a link to `to`. */
  link(node: Node | Artboard, to: Artboard, link: XdLink = {}): void {
    const action = link.action ?? "artboard-transition";
    const properties: Record<string, unknown> = {
      transition: link.transition ?? "dissolve",
      duration: link.duration ?? 0.3,
      easing: link.easing ?? "ease-out",
      preserveScrollPosition: false,
      destination: to.id,
    };
    if (link.delay !== undefined) properties.delay = link.delay;
    (this.interactions[node.id] ??= []).push({
      triggerEvent: link.trigger ?? "tap",
      action,
      properties,
      id: this.id(),
    });
  }

  private add(artboard: Artboard, node: Node): Node {
    artboard.children.push(node);
    return node;
  }

  toBytes(): Uint8Array {
    const stored = { level: 0, mtime: FIXED_DATE } as const;
    const manifest = {
      id: this.id(),
      name: this.name,
      type: "application/vnd.adobe.sparkler.project+dcxucf",
      "manifest-format-version": 6,
      children: [
        {
          name: "artwork",
          path: "artwork",
          children: this.artboards.map((a) => ({
            id: this.id(),
            name: a.name,
            path: `artboard-${a.id}`,
            "uxdesign#bounds": { x: a.box.x, y: a.box.y, width: a.box.width, height: a.box.height },
          })),
        },
        { name: "interactions", path: "interactions" },
        { name: "resources", path: "resources" },
      ],
    };
    const interactions = {
      version: "0.24",
      ...(this.home ? { homeArtboard: this.home } : {}),
      interactions: this.interactions,
    };
    const files: Record<string, [Uint8Array, typeof stored]> = {
      mimetype: [new TextEncoder().encode("application/vnd.adobe.sparkler.project+dcxucf"), stored],
      manifest: [json(manifest), stored],
      "interactions/interactions.json": [json(interactions), stored],
      "resources/graphics/graphicContent.agc": [
        json({
          version: "1.5.0",
          children: [],
          resources: { gradients: {}, clipPaths: {}, meta: { ux: { symbols: [] } } },
        }),
        stored,
      ],
    };
    for (const a of this.artboards) {
      files[`artwork/artboard-${a.id}/graphics/graphicContent.agc`] = [
        json({
          version: "1.5.0",
          children: [
            {
              type: "artboard",
              id: a.id,
              artboard: { children: a.children, meta: { ux: {} } },
              style: { fill: solid(a.background) },
            },
          ],
          resources: { href: "/resources/graphics/graphicContent.agc" },
          artboards: { href: "/resources/graphics/graphicContent.agc" },
        }),
        stored,
      ];
    }
    return zipSync(files);
  }
}
