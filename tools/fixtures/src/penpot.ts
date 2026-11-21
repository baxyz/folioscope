// SPDX-License-Identifier: AGPL-3.0-or-later
import { readFileSync } from "node:fs";
import { zipSync } from "fflate";
import type { Box } from "./figma.ts";
import { solidPng } from "./png.ts";

// What a Penpot 2.18 export declares about its file: format version, features and the schema
// migrations already applied, so an import does not replay them.
const PENPOT = JSON.parse(readFileSync(new URL("./penpot-2.18.json", import.meta.url), "utf8")) as {
  generatedBy: string;
  features: string[];
  version: number;
  migrations: string[];
};

const ROOT = "00000000-0000-0000-0000-000000000000";
const FIXED_DATE = new Date("2026-01-01T00:00:00Z");

export interface PenpotAnimation {
  animationType: "dissolve" | "slide" | "push";
  duration: number;
  easing: "linear" | "ease" | "ease-in" | "ease-out" | "ease-in-out";
  way?: "in" | "out";
  direction?: "right" | "left" | "up" | "down";
}

type Shape = Record<string, unknown> & { id: string; shapes?: string[]; interactions?: object[] };

interface PenpotPage {
  id: string;
  name: string;
  flows: Record<string, { id: string; name: string; startingFrame: string }>;
  shapes: Map<string, Shape>;
}

/** Builds a `.penpot` v3 export (Penpot 2.x), with deterministic ids. */
export class PenpotBuilder {
  private nextId = 1;
  private readonly fileId = this.id();
  private readonly pages: PenpotPage[] = [];
  private readonly media: {
    id: string;
    mediaId: string;
    width: number;
    height: number;
    png: Uint8Array;
  }[] = [];

  private readonly name: string;

  constructor(name: string) {
    this.name = name;
  }

  private id(): string {
    return `00000000-0000-4000-8000-${(this.nextId++).toString(16).padStart(12, "0")}`;
  }

  page(name: string): PenpotPage {
    const page: PenpotPage = { id: this.id(), name, flows: {}, shapes: new Map() };
    page.shapes.set(ROOT, {
      ...this.geometry(page, { x: 0, y: 0, width: 0.01, height: 0.01 }, ROOT, "#FFFFFF"),
      id: ROOT,
      name: "Root Frame",
      type: "frame",
      shapes: [],
    });
    this.pages.push(page);
    return page;
  }

  /** A top-level board: a screen of the prototype. Coordinates are absolute, as in Penpot. */
  board(page: PenpotPage, name: string, box: Box, fill = "#FFFFFF"): Shape {
    return this.add(page, ROOT, { name, type: "frame", shapes: [] }, box, fill);
  }

  rect(page: PenpotPage, board: Shape, name: string, box: Box, fill: string, radius = 0): Shape {
    return this.add(page, board.id, { name, type: "rect" }, box, fill, radius);
  }

  /** Any shape under `parent` (a board or a group), with extra fields merged in. */
  shape(
    page: PenpotPage,
    parent: Shape,
    name: string,
    type: string,
    box: Box,
    extra: Record<string, unknown> = {},
  ): Shape {
    const shape = this.add(page, parent.id, { name, type }, box, "#FFFFFF");
    Object.assign(shape, extra);
    if (type === "group" || type === "frame" || type === "svg-raw") shape.shapes ??= [];
    return shape;
  }

  /** A text of one line per run, as Penpot lays it out (`y` is the bottom of each run's box). */
  text(
    page: PenpotPage,
    parent: Shape,
    name: string,
    box: Box,
    runs: {
      text: string;
      family: string;
      weight?: number;
      size: number;
      color: string;
      decoration?: "underline" | "line-through";
    }[],
  ): Shape {
    let bottom = box.y;
    const positionData = runs.map((run) => {
      bottom += run.size * 1.2;
      return {
        text: run.text,
        x: box.x,
        y: bottom,
        width: box.width,
        height: run.size * 1.2,
        fontFamily: run.family,
        fontWeight: String(run.weight ?? 400),
        fontSize: `${run.size}px`,
        fontStyle: "normal",
        fills: [{ fillColor: run.color, fillOpacity: 1 }],
        ...(run.decoration ? { textDecoration: `${run.decoration} ${run.color}` } : {}),
      };
    });
    return this.shape(page, parent, name, "text", box, { positionData, fills: [] });
  }

  /** An image shape showing a PNG of one colour, stored as a media of the file. */
  image(
    page: PenpotPage,
    parent: Shape,
    name: string,
    box: Box,
    color: { r: number; g: number; b: number },
  ): Shape {
    const id = this.id();
    const mediaId = this.id();
    this.media.push({
      id,
      mediaId,
      width: box.width,
      height: box.height,
      png: solidPng(8, 8, color),
    });
    return this.shape(page, parent, name, "image", box, {
      metadata: { id, width: box.width, height: box.height, mtype: "image/png" },
      fills: [],
    });
  }

  /** Any interaction, as Penpot stores it. */
  interact(shape: Shape, interaction: Record<string, unknown>): void {
    (shape.interactions ??= []).push(interaction);
  }

  flow(page: PenpotPage, board: Shape, name: string): void {
    const id = this.id();
    page.flows[id] = { id, name, startingFrame: board.id };
  }

  navigate(shape: Shape, to: Shape, animation?: PenpotAnimation): void {
    (shape.interactions ??= []).push({
      eventType: "click",
      actionType: "navigate",
      destination: to.id,
      preserveScroll: false,
      ...(animation ? { animation } : {}),
    });
  }

  back(shape: Shape): void {
    (shape.interactions ??= []).push({ eventType: "click", actionType: "prev-screen" });
  }

  toBytes(): Uint8Array {
    const json = (value: unknown) => new TextEncoder().encode(JSON.stringify(value, null, 2));
    const stored = { level: 0, mtime: FIXED_DATE } as const;
    const files: Record<string, [Uint8Array, typeof stored]> = {
      "manifest.json": [
        json({
          type: "penpot/export-files",
          version: 1,
          generatedBy: PENPOT.generatedBy,
          files: [{ id: this.fileId, name: this.name, features: PENPOT.features }],
          relations: [],
        }),
        stored,
      ],
      [`files/${this.fileId}.json`]: [
        json({
          id: this.fileId,
          name: this.name,
          features: PENPOT.features,
          isShared: false,
          options: { componentsV2: true },
          migrations: PENPOT.migrations,
          version: PENPOT.version,
        }),
        stored,
      ],
    };
    this.pages.forEach((page, index) => {
      const dir = `files/${this.fileId}/pages/${page.id}`;
      files[`${dir}.json`] = [
        json({ id: page.id, name: page.name, flows: page.flows, index }),
        stored,
      ];
      for (const [id, shape] of page.shapes) files[`${dir}/${id}.json`] = [json(shape), stored];
    });
    for (const m of this.media) {
      files[`files/${this.fileId}/media/${m.id}.json`] = [
        json({
          id: m.id,
          name: "image",
          width: m.width,
          height: m.height,
          mtype: "image/png",
          mediaId: m.mediaId,
        }),
        stored,
      ];
      files[`objects/${m.mediaId}.json`] = [
        json({
          id: m.mediaId,
          size: m.png.length,
          contentType: "image/png",
          bucket: "file-media-object",
        }),
        stored,
      ];
      files[`objects/${m.mediaId}.png`] = [m.png, stored];
    }
    return zipSync(files);
  }

  private add(
    page: PenpotPage,
    parent: string,
    fields: { name: string; type: string; shapes?: string[] },
    box: Box,
    fill: string,
    radius = 0,
  ): Shape {
    const shape: Shape = {
      ...this.geometry(page, box, parent, fill, radius),
      id: this.id(),
      ...fields,
    };
    page.shapes.set(shape.id, shape);
    // Penpot lists children bottom to top and shows the last one first, at the top of the layer
    // panel and in view mode: new shapes go first, so the first board created is the one shown.
    page.shapes.get(parent)!.shapes!.unshift(shape.id);
    return shape;
  }

  /** The geometry Penpot stores redundantly: box, selection rect, corner points, matrices. */
  private geometry(page: PenpotPage, box: Box, parent: string, fill: string, radius = 0) {
    const { x, y, width, height } = box;
    const identity = { a: 1, b: 0, c: 0, d: 1, e: 0, f: 0 };
    const parentShape = page.shapes.get(parent);
    // A shape's frame is its nearest board: the parent itself unless it is the root.
    const frameId =
      parent === ROOT
        ? ROOT
        : parentShape?.type === "frame"
          ? parent
          : (parentShape?.frameId as string);
    return {
      x,
      y,
      width,
      height,
      rotation: 0,
      selrect: { x, y, width, height, x1: x, y1: y, x2: x + width, y2: y + height },
      points: [
        { x, y },
        { x: x + width, y },
        { x: x + width, y: y + height },
        { x, y: y + height },
      ],
      transform: identity,
      transformInverse: identity,
      parentId: parent,
      frameId,
      pageId: page.id,
      flipX: null,
      flipY: null,
      proportion: 1,
      proportionLock: false,
      hideFillOnExport: false,
      rx: radius,
      ry: radius,
      r1: radius,
      r2: radius,
      r3: radius,
      r4: radius,
      strokes: [],
      fills: [{ fillColor: fill, fillOpacity: 1 }],
    };
  }
}
