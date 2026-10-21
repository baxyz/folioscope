// SPDX-License-Identifier: AGPL-3.0-or-later
// Types only: the package entry re-exports them as a CommonJS default, which `nodenext` hides.
import type * as FileFormat from "@sketch-hq/sketch-file-format-ts/dist/cjs/types.js";
import { zipSync } from "fflate";
import type { Box } from "./figma.ts";
import { solidPng } from "./png.ts";

const FIXED_DATE = new Date("2026-01-01T00:00:00Z");

export type SketchAnimation =
  | "none"
  | "slideFromLeft"
  | "slideFromRight"
  | "slideFromBottom"
  | "slideFromTop";

const ANIMATIONS: Record<SketchAnimation, number> = {
  none: 0,
  slideFromLeft: 1,
  slideFromRight: 2,
  slideFromBottom: 3,
  slideFromTop: 4,
};

type Layer = FileFormat.Rectangle | FileFormat.Hotspot;

/**
 * Builds a `.sketch` file, typed against Sketch's published file format. Ids are deterministic so
 * regenerating is a no-op.
 */
export class SketchBuilder {
  private nextId = 1;
  private readonly pages: FileFormat.Page[] = [];

  /** UUID-shaped, deterministic. */
  private id(): string {
    return `00000000-0000-4000-8000-${(this.nextId++).toString(16).padStart(12, "0")}`;
  }

  page(name: string): FileFormat.Page {
    const page: FileFormat.Page = {
      ...this.base(name, { x: 0, y: 0, width: 0, height: 0 }),
      _class: "page",
      ...this.container(),
      hasClickThrough: true,
      horizontalRulerData: { _class: "rulerData", base: 0, guides: [] },
      verticalRulerData: { _class: "rulerData", base: 0, guides: [] },
      layers: [],
    };
    this.pages.push(page);
    return page;
  }

  artboard(
    page: FileFormat.Page,
    name: string,
    box: Box,
    background = "#FFFFFF",
  ): FileFormat.Artboard {
    const artboard: FileFormat.Artboard = {
      ...this.base(name, box),
      _class: "artboard",
      ...this.container(),
      grid: { _class: "simpleGrid", isEnabled: false, gridSize: 8, thickGridTimes: 10 },
      hasClickThrough: false,
      horizontalRulerData: { _class: "rulerData", base: 0, guides: [] },
      verticalRulerData: { _class: "rulerData", base: 0, guides: [] },
      backgroundColor: color(background),
      hasBackgroundColor: true,
      includeBackgroundColorInExport: true,
      isFlowHome: false,
      resizesContent: false,
      layers: [],
    };
    // Written by Sketch 69, though missing from schema 6.5.
    Object.assign(artboard, { presetDictionary: {} });
    page.layers.push(artboard);
    return artboard;
  }

  rect(
    artboard: FileFormat.Artboard,
    name: string,
    box: Box,
    fill: string,
    radius = 0,
  ): FileFormat.Rectangle {
    const corner = (point: string): FileFormat.CurvePoint => ({
      _class: "curvePoint",
      cornerRadius: radius,
      cornerStyle: 0,
      curveFrom: point,
      curveTo: point,
      hasCurveFrom: false,
      hasCurveTo: false,
      curveMode: 1,
      point,
    });
    const rect: FileFormat.Rectangle = {
      ...this.base(name, box),
      _class: "rectangle",
      clippingMaskMode: 0,
      hasClippingMask: false,
      style: this.style(fill),
      edited: false,
      isClosed: true,
      pointRadiusBehaviour: 1,
      points: ["{0, 0}", "{1, 0}", "{1, 1}", "{0, 1}"].map(corner),
      fixedRadius: radius,
      hasConvertedToNewRoundCorners: true,
      needsConvertionToNewRoundCorners: false,
    };
    artboard.layers.push(rect);
    return rect;
  }

  /** An invisible clickable area, Sketch's way to add a link without a visible layer. */
  hotspot(artboard: FileFormat.Artboard, name: string, box: Box): FileFormat.Hotspot {
    const hotspot: FileFormat.Hotspot = {
      ...this.base(name, box),
      _class: "MSImmutableHotspotLayer",
    };
    artboard.layers.push(hotspot);
    return hotspot;
  }

  /** Sketch's only trigger is a click or tap. */
  link(layer: Layer, to: FileFormat.Artboard | "back", animation: SketchAnimation = "none"): void {
    layer.flow = {
      _class: "MSImmutableFlowConnection",
      destinationArtboardID: to === "back" ? "back" : to.do_objectID,
      animationType: ANIMATIONS[animation],
    };
  }

  flowHome(artboard: FileFormat.Artboard): void {
    artboard.isFlowHome = true;
  }

  toBytes(): Uint8Array {
    const json = (value: unknown) => new TextEncoder().encode(JSON.stringify(value));
    const document: FileFormat.Document = {
      _class: "document",
      do_objectID: this.id(),
      assets: {
        _class: "assetCollection",
        do_objectID: this.id(),
        colorAssets: [],
        gradientAssets: [],
        images: [],
        colors: [],
        gradients: [],
        exportPresets: [],
        imageCollection: { _class: "imageCollection", images: {} },
      },
      colorSpace: 1,
      currentPageIndex: 0,
      foreignLayerStyles: [],
      foreignSymbols: [],
      foreignTextStyles: [],
      foreignSwatches: [],
      fontReferences: [],
      layerStyles: { _class: "sharedStyleContainer", do_objectID: this.id(), objects: [] },
      layerSymbols: { _class: "symbolContainer", do_objectID: this.id(), objects: [] },
      sharedSwatches: { _class: "swatchContainer", do_objectID: this.id(), objects: [] },
      documentState: { _class: "documentState" },
      layerTextStyles: { _class: "sharedTextStyleContainer", do_objectID: this.id(), objects: [] },
      perDocumentLibraries: [],
      pages: this.pages.map((page) => ({
        _class: "MSJSONFileReference",
        _ref_class: "MSImmutablePage",
        _ref: `pages/${page.do_objectID}`,
      })),
    };
    const created = {
      commit: "folioscope-fixtures",
      appVersion: "69.1",
      build: 0,
      app: "com.bohemiancoding.sketch3" as FileFormat.BundleId,
      compatibilityVersion: 99,
      version: 134,
      variant: "NONAPPSTORE" as FileFormat.SketchVariant,
    };
    const meta: FileFormat.Meta = {
      ...created,
      version: 134,
      compatibilityVersion: 99,
      pagesAndArtboards: Object.fromEntries(
        this.pages.map((page) => [
          page.do_objectID,
          {
            name: page.name,
            artboards: Object.fromEntries(
              page.layers
                .filter((l) => l._class === "artboard")
                .map((a) => [a.do_objectID, { name: a.name }]),
            ),
          },
        ]),
      ),
      autosaved: 0,
      created,
      saveHistory: [],
    };
    Object.assign(meta, { fonts: [] }); // as above
    const user: FileFormat.User = {
      document: { pageListHeight: 110, pageListCollapsed: 0 },
      ...Object.fromEntries(
        this.pages.map((page) => [
          page.do_objectID,
          { scrollOrigin: "{100, 100}", zoomValue: 0.5 },
        ]),
      ),
    };
    const stored = { level: 0, mtime: FIXED_DATE } as const;
    return zipSync({
      "document.json": [json(document), stored],
      "meta.json": [json(meta), stored],
      "user.json": [json(user), stored],
      ...Object.fromEntries(
        this.pages.map((page) => [`pages/${page.do_objectID}.json`, [json(page), stored]]),
      ),
      "previews/preview.png": [solidPng(400, 300, { r: 0.96, g: 0.96, b: 0.96 }), stored],
    });
  }

  /** The fields every layer shares. */
  private base(name: string, box: Box) {
    return {
      do_objectID: this.id(),
      booleanOperation: -1,
      exportOptions: {
        _class: "exportOptions" as const,
        exportFormats: [],
        includedLayerIds: [],
        layerOptions: 0,
        shouldTrim: false,
      },
      frame: { _class: "rect" as const, constrainProportions: false, ...box },
      isFixedToViewport: false,
      isFlippedHorizontal: false,
      isFlippedVertical: false,
      isLocked: false,
      isTemplate: false,
      isVisible: true,
      layerListExpandedType: 0,
      name,
      nameIsFixed: true,
      resizingConstraint: 63,
      resizingType: 0,
      rotation: 0,
      shouldBreakMaskChain: false,
    };
  }

  /** Fields Sketch writes on pages and artboards, optional in the schema but expected by tools. */
  private container() {
    return {
      clippingMaskMode: 0,
      hasClippingMask: false,
      style: this.style(),
      groupLayout: { _class: "MSImmutableFreeformGroupLayout" as const },
      includeInCloudUpload: true,
    };
  }

  private style(fill?: string): FileFormat.Style {
    return {
      _class: "style",
      do_objectID: this.id(),
      borderOptions: {
        _class: "borderOptions",
        isEnabled: true,
        dashPattern: [],
        lineCapStyle: 0,
        lineJoinStyle: 0,
      },
      blur: {
        _class: "blur",
        isEnabled: false,
        center: "{0.5, 0.5}",
        motionAngle: 0,
        radius: 10,
        saturation: 1,
        type: 0,
      },
      borders: [],
      shadows: [],
      contextSettings: { _class: "graphicsContextSettings", blendMode: 0, opacity: 1 },
      fills:
        fill === undefined
          ? []
          : [
              {
                _class: "fill",
                isEnabled: true,
                color: color(fill),
                fillType: 0,
                noiseIndex: 0,
                noiseIntensity: 0,
                patternFillType: 1,
                patternTileScale: 1,
                contextSettings: { _class: "graphicsContextSettings", blendMode: 0, opacity: 1 },
                gradient: {
                  _class: "gradient",
                  gradientType: 0,
                  elipseLength: 0,
                  from: "{0.5, 0}",
                  to: "{0.5, 1}",
                  stops: [],
                },
              },
            ],
      startMarkerType: 0,
      endMarkerType: 0,
      miterLimit: 10,
      windingRule: 1,
      innerShadows: [],
      colorControls: {
        _class: "colorControls",
        isEnabled: false,
        brightness: 0,
        contrast: 1,
        hue: 0,
        saturation: 1,
      },
    };
  }
}

function color(hex: string): FileFormat.Color {
  const h = hex.replace("#", "");
  const channel = (i: number) => parseInt(h.slice(i, i + 2), 16) / 255;
  return {
    _class: "color",
    red: channel(0),
    green: channel(2),
    blue: channel(4),
    alpha: h.length >= 8 ? channel(6) : 1,
  };
}
